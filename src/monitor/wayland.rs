use std::{collections::HashMap, time::Duration};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use wayland_protocols_async::ext_idle_notify_v1::{
    errors::IdleNotifyHandlerErrorCodes,
    handler::{IdleNotifyEvent, IdleNotifyHandler},
};

use crate::monitor::{MonitorEvent, MonitorJoinHandle, WatchEvent};

const STATUS_KEY: &str = "status";
const NOTIFS_KEY: &str = "notifs";

pub async fn start_monitor(
    status_timeout: Duration,
    notifs_timeout: Duration,
    sender: mpsc::Sender<MonitorEvent>,
    token: CancellationToken,
) -> Result<MonitorJoinHandle, IdleNotifyHandlerErrorCodes> {
    let subscribers = HashMap::from([
        (STATUS_KEY.into(), status_timeout),
        (NOTIFS_KEY.into(), notifs_timeout),
    ]);

    let (handler_tx, mut handler_rx) = mpsc::channel(8);

    let handler_token = token.child_token();
    let translate_token = token.child_token();

    let mut idle_notify_handler = IdleNotifyHandler::new(subscribers, handler_tx)?;

    let handler_join = tokio::spawn(async move {
        let _ = idle_notify_handler.run(handler_token).await;
    });

    let join = tokio::spawn(async move {
        'translate: loop {
            tokio::select! {
                _ = translate_token.cancelled() => {
                    let _ = handler_join.await.unwrap();
                    break 'translate;
                }
                Some(evt) = handler_rx.recv() => {
                    match evt {
                        IdleNotifyEvent::Idled{ key: name } => {
                            if name == STATUS_KEY {
                                sender.send(MonitorEvent::WatchEvent(WatchEvent::StatusIdle(true))).await.unwrap();
                            } else if name == NOTIFS_KEY {
                                sender.send(MonitorEvent::WatchEvent(WatchEvent::NotifsIdle(true))).await.unwrap();
                            }
                        }
                        IdleNotifyEvent::Resumed{ key: name } => {
                            if name == STATUS_KEY {
                                sender.send(MonitorEvent::WatchEvent(WatchEvent::StatusIdle(false))).await.unwrap();
                            } else if name == NOTIFS_KEY {
                                sender.send(MonitorEvent::WatchEvent(WatchEvent::NotifsIdle(false))).await.unwrap();
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    });

    Ok(join)
}
