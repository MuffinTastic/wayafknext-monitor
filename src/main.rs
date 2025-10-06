mod monitor;
mod util;

use std::{fs, thread, time::Duration};

use anyhow::anyhow;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    iterator::Signals,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{UnixListener, UnixStream},
    sync::{broadcast, mpsc},
};
use tokio_util::sync::CancellationToken;

use crate::monitor::MonitorEvent;

#[derive(Debug, Clone, serde::Serialize)]
pub enum Broadcast {
    WatchEvent(monitor::WatchEvent),
    WatchStarted { status_mins: u64, notifs_mins: u64 },
    WatchStopped(()),
}

#[derive(Debug, serde::Deserialize)]
pub enum ClientInput {
    Quit(()),
    StartWatch { status_mins: u64, notifs_mins: u64 },
    StopWatch(()),
}

pub enum ClientEvent {
    Input(ClientInput),
    Error(Box<anyhow::Error>),
}

const SOCKET_NAME: &str = "wayafknext.sock";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !util::is_wayland() {
        return Err(anyhow!("Not a Wayland desktop"));
    }

    let main_token = CancellationToken::new();

    let mut proc_signals = Signals::new([SIGINT, SIGTERM])?;

    let signal_token = main_token.clone();
    thread::spawn(move || {
        for sig in proc_signals.forever() {
            println!("Process signal {:?} received", sig);
            signal_token.cancel();
        }
    });

    let socket_path = util::get_exe_dir()?.join(SOCKET_NAME);

    if socket_path.exists() {
        fs::remove_file(&socket_path)?;
        println!("Removed old socket");
    }

    let listener = UnixListener::bind(socket_path)?;
    let (client_tx, mut client_rx) = mpsc::channel(8);
    let (monitor_tx, mut monitor_rx) = mpsc::channel(8);
    let (broadcast_tx, broadcast_rx) = broadcast::channel(8);

    // start off with a cancelled token to indicate that nothing is running
    let mut monitor_token = CancellationToken::new();
    monitor_token.cancel();

    println!("Listening for clients");

    'main: loop {
        tokio::select! {
            _ = main_token.cancelled() => {
                break 'main;
            }
            accept_res = listener.accept() => {
                let (stream, _addr) = accept_res?;
                let client_tx = client_tx.clone();
                let broadcast_rx = broadcast_rx.resubscribe();
                let main_token = main_token.child_token();
                tokio::spawn(async move {
                    let res = handle_client(stream, client_tx.clone(), broadcast_rx, main_token).await;
                    if let Err(err) = res {
                        client_tx.send(ClientEvent::Error(Box::new(err))).await.unwrap();
                    }
                });
            }
            Some(client_incoming) = client_rx.recv() => {
                match client_incoming {
                    ClientEvent::Input(ClientInput::Quit(())) => {
                        println!("Quit requested");
                        main_token.cancel();
                    }
                    ClientEvent::Input(ClientInput::StartWatch { status_mins, notifs_mins }) => {
                        if !monitor_token.is_cancelled() {
                            println!("Stopped old watch");
                            monitor_token.cancel();
                            broadcast_tx.send(Broadcast::WatchStopped(())).unwrap();
                        }

                        let status_timeout = Duration::from_secs(status_mins * 60);
                        let notifs_timeout = Duration::from_secs(notifs_mins * 60);

                        let monitor_tx = monitor_tx.clone();

                        let (join, token) = monitor::start(status_timeout, notifs_timeout, monitor_tx.clone()).await?;
                        monitor_token = token;

                        // this is kind of a hacky mess but i think it's the easiest way to grab any errors from the monitor
                        tokio::spawn(async move {
                            let res = match join.await {
                                Ok(Ok(())) => Ok(()),
                                Ok(Err(err)) => Err(err),
                                Err(err) => Err(err.into())
                            };

                            if let Err(err) = res {
                                monitor_tx.send(MonitorEvent::Error(Box::new(err))).await.unwrap();
                            }
                        });

                        println!("Started watch: {status_mins} mins, {notifs_mins} mins");
                        broadcast_tx.send(Broadcast::WatchStarted { status_mins, notifs_mins } ).unwrap();
                    }
                    ClientEvent::Input(ClientInput::StopWatch(())) => {
                        if !monitor_token.is_cancelled() {
                            println!("Stopped watch");
                            monitor_token.cancel();
                            broadcast_tx.send(Broadcast::WatchStopped(())).unwrap();
                        }
                    }
                    ClientEvent::Error(err_box) => {
                        eprintln!("Client errored: {:?}", *err_box);
                        main_token.cancel();
                        return Err(*err_box);
                    }
                }
            }
            Some(monitor_incoming) = monitor_rx.recv() => {
                match monitor_incoming {
                    MonitorEvent::WatchEvent(watch_event) => {
                        broadcast_tx.send(Broadcast::WatchEvent(watch_event)).unwrap();
                    }
                    MonitorEvent::Error(err_box) => {
                        eprintln!("Monitor errored: {:?}", *err_box);
                        main_token.cancel();
                        return Err(*err_box);
                    }
                }
            }
        }
    }

    println!("Shutting down");

    Ok(())
}

async fn handle_client(
    stream: UnixStream,
    incoming_tx: mpsc::Sender<ClientEvent>,
    mut broadcast_rx: broadcast::Receiver<Broadcast>,
    main_token: CancellationToken,
) -> anyhow::Result<()> {
    println!("Accepted client");

    let (reader, mut writer) = tokio::io::split(stream);

    let bufreader = BufReader::new(reader);
    let mut lines = bufreader.lines();

    'client: loop {
        tokio::select! {
            _ = main_token.cancelled() => {
                break 'client;
            }
            broadcast_res = broadcast_rx.recv() => {
                let broadcast = broadcast_res?;
                let json = serde_json::to_string(&broadcast)?;
                writer.write_all(json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
            }
            read_res = lines.next_line() => {
                if let Some(line) = read_res? {
                    let input = serde_json::from_str(&line)?;
                    incoming_tx.send(ClientEvent::Input(input)).await?;
                }
            }
        }
    }

    Ok(())
}
