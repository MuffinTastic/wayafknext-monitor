mod wayland_idle;

use std::sync::mpsc::Sender;

use anyhow::Result;

pub enum IdleEvent {
    Idled,
    Resumed,
}

pub fn idle_start(timeout: u32, idle_signal: Sender<IdleEvent>) -> Result<()> {
    // TODO use different path for GNOME

    wayland_idle::set_signal_sender(idle_signal);
    wayland_idle::start_idle_monitor(timeout)?;

    Ok(())
}

pub fn idle_stop() {
    wayland_idle::stop_idle_monitor();
}
