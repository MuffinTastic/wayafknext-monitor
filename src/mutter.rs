use std::{collections::HashSet, process, time::Duration};

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use wayland_protocols_async::ext_idle_notify_v1::handler::IdleNotifyEvent;
use zbus::{Connection, Proxy};

use crate::{Status, print_status};

const BUS_NAME: &str = "org.gnome.Mutter.IdleMonitor";
const OBJECT_PATH: &str = "/org/gnome/Mutter/IdleMonitor/Core";
const INTERFACE: &str = "org.gnome.Mutter.IdleMonitor";

// this reuses the Wayland IdleNotifyEvent enum because i am lazy
// also this whole thing was completely winged cause i don't use gnome
pub async fn start_idle_notify_mutter(
    timeout: Duration,
    sender: mpsc::Sender<IdleNotifyEvent>,
) -> Result<CancellationToken, zbus::Error> {
    let connection = Connection::session().await?;
    let proxy = Proxy::new(&connection, BUS_NAME, OBJECT_PATH, INTERFACE).await?;

    let idle_watch_id: u64 = proxy
        .call("AddIdleWatch", &(timeout.as_millis() as u64))
        .await?;

    let mut active_watches = HashSet::new();

    let mut watch_fired = proxy.receive_signal("WatchFired").await?;

    let token = CancellationToken::new();
    let child_token = token.child_token();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = child_token.cancelled() => {
                    // attempt to remove the watch before exit
                    let _: Result<(), zbus::Error> = proxy.call("RemoveWatch", &(idle_watch_id,)).await;
                    break;
                }
                Some(msg) = watch_fired.next() => {
                    let res: Result<(u64,), zbus::Error> = msg.body().deserialize();
                    match res {
                        Ok(watch_id) => {
                            let watch_id = watch_id.0;

                            if watch_id == idle_watch_id {
                                let _ = sender.send(IdleNotifyEvent::Idled{key:"1".into()}).await;

                                let res: Result<u64, zbus::Error> = proxy.call("AddUserActiveWatch", &()).await;
                                match res {
                                    Ok(active_watch_id) => {
                                        active_watches.insert(active_watch_id);
                                    },
                                    Err(active_watch_err) => {
                                        print_status(Status::Error(format!("{:?}", active_watch_err)));
                                        process::exit(1);
                                    }
                                }

                            } else if active_watches.contains(&watch_id) {
                                active_watches.remove(&watch_id);

                            }
                        }
                        Err(read_err) => {
                            print_status(Status::Error(format!("{:?}", read_err)));
                            process::exit(1);
                        }
                    }
                }
            }

            print_status(Status::WatchStopped(()));
        }
    });

    Ok(token)
}
