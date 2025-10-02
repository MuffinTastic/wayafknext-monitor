mod util;

use std::{collections::HashMap, time::Duration};

use anyhow::anyhow;
use tokio::sync::mpsc;
use wayland_protocols_async::ext_idle_notify_v1::handler::IdleNotifyHandler;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if !util::is_wayland() {
        return Err(anyhow!("Not running on Wayland!"));
    }

    let (idle_notify_event_tx, mut idle_notify_event_rx) = mpsc::channel(128);

    let mut subscribers = HashMap::new();

    subscribers.insert(String::from("1"), Duration::from_secs(5));

    let mut idle_notify_handler = IdleNotifyHandler::new(subscribers, idle_notify_event_tx);

    let idle_notify_t = tokio::spawn(async move {
        let _ = idle_notify_handler.run().await;
    });

    let idle_notify_event_t = tokio::spawn(async move {
        loop {
            let evt = idle_notify_event_rx.recv().await;
            if evt.is_none() {
                continue;
            }
            println!("event: {:?}", evt);
        }
    });

    let _ = idle_notify_t.await.unwrap();
    let _ = idle_notify_event_t.await.unwrap();

    Ok(())
}
