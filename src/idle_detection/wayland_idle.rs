// Taken and modified from https://github.com/unobserved-io/Furtherance/blob/main/src/helpers/wayland_idle.rs#L277

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, Sender, channel},
};
use std::thread;
use wayland_client::protocol::wl_registry::{self, WlRegistry};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_client::{Connection, Dispatch, QueueHandle};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notification_v1::{
    self, ExtIdleNotificationV1,
};
use wayland_protocols::ext::idle_notify::v1::client::ext_idle_notifier_v1::ExtIdleNotifierV1;

use crate::idle_detection::IdleEvent as WayIdleEvent;

lazy_static::lazy_static! {
    static ref IDLE_SIGNAL: Arc<Mutex<Option<Sender<WayIdleEvent>>>> = Arc::new(Mutex::new(None));
    static ref WAYLAND_INITIALIZED: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    static ref MONITOR_RUNNING: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    static ref STOP_SIGNAL: Arc<Mutex<Option<Sender<()>>>> = Arc::new(Mutex::new(None));
}

enum IdleManager {
    Standard(ExtIdleNotifierV1),
}

struct WaylandState {
    idle_state: Arc<Mutex<Option<Sender<WayIdleEvent>>>>,
    seats: HashMap<u32, WlSeat>,
    idle_manager: Option<IdleManager>,
}

impl WaylandState {
    fn new(idle_state: Arc<Mutex<Option<Sender<WayIdleEvent>>>>) -> Self {
        Self {
            idle_state,
            seats: HashMap::new(),
            idle_manager: None,
        }
    }

    fn handle_global(
        &mut self,
        registry: &WlRegistry,
        name: u32,
        interface: String,
        version: u32,
        qh: &QueueHandle<Self>,
    ) {
        match &interface[..] {
            "wl_seat" => {
                let seat = registry.bind::<WlSeat, _, _>(name, version, qh, ());
                if let Some(idle_manager) = &self.idle_manager {
                    let timeout_ms = 1000; // 1 second
                    match idle_manager {
                        IdleManager::Standard(manager) => {
                            let _notification =
                                manager.get_idle_notification(timeout_ms, &seat, qh, name);
                        }
                    }
                }
                self.seats.insert(name, seat);
            }
            "ext_idle_notifier_v1" => {
                let idle_manager: ExtIdleNotifierV1 = registry.bind(name, version, qh, ());
                // Set up idle notifications for existing seats
                for (name, seat) in &self.seats {
                    let _notification = idle_manager.get_idle_notification(1000, seat, qh, *name);
                }
                self.idle_manager = Some(IdleManager::Standard(idle_manager));
            }
            _ => {}
        }
    }
}

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => state.handle_global(registry, name, interface, version, qh),
            _ => {}
        }
    }
}

impl Dispatch<ExtIdleNotificationV1, u32> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &ExtIdleNotificationV1,
        event: ext_idle_notification_v1::Event,
        _data: &u32,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            ext_idle_notification_v1::Event::Idled => {
                if let Ok(mut state) = state.idle_state.lock() {
                    if let Some(sender) = state.as_mut() {
                        sender.send(WayIdleEvent::Idled);
                    }
                }
            }
            ext_idle_notification_v1::Event::Resumed => {
                if let Ok(mut state) = state.idle_state.lock() {
                    if let Some(sender) = state.as_mut() {
                        sender.send(WayIdleEvent::Resumed);
                    }
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WlSeat,
        _event: <WlSeat as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}
impl Dispatch<ExtIdleNotifierV1, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &ExtIdleNotifierV1,
        _event: <ExtIdleNotifierV1 as wayland_client::Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
    }
}

pub fn set_signal_sender(idle_signal: Sender<WayIdleEvent>) {
    if let Ok(mut state) = IDLE_SIGNAL.lock() {
        *state = Some(idle_signal);
    }
}

fn run_wayland_monitor(
    conn: Connection,
    timeout: u32,
    rx: Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let display = conn.display();
    display.get_registry(&qh, ());

    let state = WaylandState::new(IDLE_SIGNAL.clone());
    let mut state = state;

    loop {
        if !MONITOR_RUNNING.load(Ordering::SeqCst) {
            break;
        }

        // Check if we received a stop signal
        if rx.try_recv().is_ok() {
            break;
        }

        event_queue.blocking_dispatch(&mut state)?;
        thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(())
}

pub fn start_idle_monitor(timeout: u32) -> anyhow::Result<()> {
    if let Ok(mut initialized) = WAYLAND_INITIALIZED.lock() {
        if *initialized {
            return Ok(());
        }

        let conn = Connection::connect_to_env()?;

        MONITOR_RUNNING.store(true, Ordering::SeqCst);

        // Create a channel for stop signaling
        let (tx, rx) = channel();
        if let Ok(mut stop_signal) = STOP_SIGNAL.lock() {
            *stop_signal = Some(tx);
        }

        thread::spawn(move || {
            if let Err(e) = run_wayland_monitor(conn, timeout, rx) {
                eprintln!("Wayland monitor error: {}", e);
            }
        });

        *initialized = true;
    }

    Ok(())
}

pub fn stop_idle_monitor() {
    MONITOR_RUNNING.store(false, Ordering::SeqCst);

    // Signal the monitor thread to stop
    if let Ok(stop_signal) = STOP_SIGNAL.lock() {
        if let Some(tx) = stop_signal.as_ref() {
            let _ = tx.send(());
        }
    }

    // Reset idle state
    if let Ok(mut state) = IDLE_SIGNAL.lock() {
        *state = None;
    }

    // Reset initialized state
    if let Ok(mut initialized) = WAYLAND_INITIALIZED.lock() {
        *initialized = false;
    }

    // Clear the stop signal
    if let Ok(mut stop_signal) = STOP_SIGNAL.lock() {
        *stop_signal = None;
    }
}
