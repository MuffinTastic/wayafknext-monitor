mod mutter;
mod util;

use std::{collections::HashMap, io, process, time::Duration};

use anyhow::Context;
use tokio::{sync::mpsc, task};
use tokio_util::sync::CancellationToken;
use wayland_protocols_async::ext_idle_notify_v1::{
    errors::IdleNotifyHandlerErrorCodes,
    handler::{IdleNotifyEvent, IdleNotifyHandler},
};

use crate::mutter::start_idle_notify_mutter;

#[derive(Debug, serde::Serialize)]
pub enum Status {
    Idle(bool),
    WatchStarted(u64),
    WatchStopped(()),
    Error(String),
}

pub fn print_status(status: Status) {
    println!("{}", serde_json::to_string(&status).unwrap());
}

#[tokio::main]
async fn main() {
    if !util::is_wayland() {
        println!("ERR Not running on Wayland!");
        process::exit(1);
    }

    let (idle_notify_event_tx, mut idle_notify_event_rx) = mpsc::channel(32);
    let (stdin_tx, mut stdin_rx) = mpsc::channel(32);

    let stdin_token = CancellationToken::new();
    let mut idle_notify_token = CancellationToken::new();

    let stdin_child_token = stdin_token.child_token();
    task::spawn_blocking(|| read_stdin(stdin_tx, stdin_child_token));

    'main: loop {
        tokio::select! {
            Some(line) = stdin_rx.recv() => {
                if line == "quit" {
                    break 'main;
                } else if line == "stop" {
                    idle_notify_token.cancel();
                } else if let Ok(timeout_mins) = line.parse::<u64>() {
                    idle_notify_token.cancel();
                    if timeout_mins > 0 {
                        let timeout = Duration::from_secs(timeout_mins * 60);
                        let res = start_idle_notify(timeout, idle_notify_event_tx.clone()).await;
                        match res {
                            Ok(token) => idle_notify_token = token,
                            Err(start_err) => {
                                print_status(Status::Error(format!("{:?}", start_err)));
                                process::exit(1);
                            }
                        }
                        print_status(Status::WatchStarted(timeout_mins));
                    }
                }
            }
            Some(evt) = idle_notify_event_rx.recv() => {
                let idle = match evt {
                    IdleNotifyEvent::Idled{key: _} => true,
                    IdleNotifyEvent::Resumed{key: _} => false
                };

                print_status(Status::Idle(idle));
            }
        }
    }

    stdin_token.cancel();

    // i don't like this but rust doesn't provide an easy way of doing non-blocking
    // stdin reads without hanging the async runtime on quit so eh whatever just exit()
    process::exit(0);
}

fn read_stdin(line: mpsc::Sender<String>, token: CancellationToken) -> Result<(), io::Error> {
    let mut buf = String::new();

    loop {
        if token.is_cancelled() {
            break;
        }

        let _ = io::stdin().read_line(&mut buf)?;
        let _ = line.blocking_send(buf.trim().to_owned());
        buf.clear();
    }

    Ok(())
}

async fn start_idle_notify(
    timeout: Duration,
    sender: mpsc::Sender<IdleNotifyEvent>,
) -> anyhow::Result<CancellationToken> {
    if util::is_mutter() {
        start_idle_notify_mutter(timeout, sender)
            .await
            .context("Couldn't start Mutter idle notifier")
    } else {
        start_idle_notify_wayland(timeout, sender)
            .await
            .context("Couldn't start Wayland idle notifier")
    }
}

async fn start_idle_notify_wayland(
    timeout: Duration,
    sender: mpsc::Sender<IdleNotifyEvent>,
) -> Result<CancellationToken, IdleNotifyHandlerErrorCodes> {
    let token = CancellationToken::new();

    let mut subscribers = HashMap::new();

    subscribers.insert(String::from("1"), timeout);

    let mut idle_notify_handler = IdleNotifyHandler::new(subscribers, sender)?;

    let child_token = token.child_token();
    tokio::spawn(async move {
        let _ = idle_notify_handler.run(child_token).await;
        print_status(Status::WatchStopped(()));
    });

    Ok(token)
}
