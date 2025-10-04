use std::time::Duration;

use anyhow::Context;
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::util;

mod mutter;
mod wayland;

#[derive(Debug, Clone, serde::Serialize)]
pub enum WatchEvent {
    StatusIdle(bool),
    NotifsIdle(bool),
}

pub enum MonitorEvent {
    WatchEvent(WatchEvent),
    Error(Box<anyhow::Error>),
}

pub type MonitorJoinHandle = JoinHandle<anyhow::Result<()>>;

pub async fn start(
    status_timeout: Duration,
    notifs_timeout: Duration,
    sender: mpsc::Sender<MonitorEvent>,
) -> anyhow::Result<(MonitorJoinHandle, CancellationToken)> {
    let token = CancellationToken::new();

    let join = if util::is_mutter() {
        mutter::start_monitor(status_timeout, notifs_timeout, sender, token.clone())
            .await
            .context("Couldn't start Mutter idle notifier")?
    } else {
        wayland::start_monitor(status_timeout, notifs_timeout, sender, token.clone())
            .await
            .context("Couldn't start Wayland idle notifier")?
    };

    Ok((join, token))
}
