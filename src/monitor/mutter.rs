use std::time::Duration;

use futures_util::StreamExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use zbus::{Connection, Proxy};

use crate::monitor::{MonitorEvent, MonitorJoinHandle, WatchEvent};

const BUS_NAME: &str = "org.gnome.Mutter.IdleMonitor";
const OBJECT_PATH: &str = "/org/gnome/Mutter/IdleMonitor/Core";
const INTERFACE: &str = "org.gnome.Mutter.IdleMonitor";

pub async fn start_monitor(
    status_timeout: Duration,
    notifs_timeout: Duration,
    sender: mpsc::Sender<MonitorEvent>,
    token: CancellationToken,
) -> Result<MonitorJoinHandle, zbus::Error> {
    let connection = Connection::session().await?;
    let proxy = Proxy::new(&connection, BUS_NAME, OBJECT_PATH, INTERFACE).await?;

    let status_idle_watch: u64 = proxy
        .call("AddIdleWatch", &(status_timeout.as_millis() as u64))
        .await?;

    let notifs_idle_watch: u64 = proxy
        .call("AddIdleWatch", &(notifs_timeout.as_millis() as u64))
        .await?;

    let mut status_resume_watch: u64 = 0;
    let mut notifs_resume_watch: u64 = 0;

    let mut signal = proxy.receive_signal("WatchFired").await?;

    let child_token = token.child_token();

    let join = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = child_token.cancelled() => {
                    let _: () = proxy.call("RemoveWatch", &(status_idle_watch,)).await?;
                    let _: () = proxy.call("RemoveWatch", &(notifs_idle_watch,)).await?;
                    if status_resume_watch != 0 {
                        let _: () = proxy.call("RemoveWatch", &(status_resume_watch,)).await?;
                    }
                    if notifs_resume_watch != 0 {
                        let _: () = proxy.call("RemoveWatch", &(notifs_resume_watch,)).await?;
                    }
                    break;
                }
                Some(msg) = signal.next() => {
                    let res: (u64,) = msg.body().deserialize()?;
                    let watch_id = res.0;

                    if watch_id == status_idle_watch {
                        sender.send(MonitorEvent::WatchEvent(WatchEvent::StatusIdle(true))).await?;
                        status_resume_watch = proxy.call("AddUserActiveWatch", &()).await?;
                    }

                    if watch_id == notifs_idle_watch {
                        sender.send(MonitorEvent::WatchEvent(WatchEvent::NotifsIdle(true))).await?;
                        notifs_resume_watch = proxy.call("AddUserActiveWatch", &()).await?;
                    }

                    if watch_id == status_resume_watch {
                        sender.send(MonitorEvent::WatchEvent(WatchEvent::StatusIdle(false))).await?;
                        status_resume_watch = 0;
                    }

                    if watch_id == notifs_resume_watch {
                        sender.send(MonitorEvent::WatchEvent(WatchEvent::NotifsIdle(false))).await?;
                        notifs_resume_watch = 0;
                    }
                }
            }
        }

        Ok(())
    });

    Ok(join)
}
