#[cfg(target_os = "linux")]
use {
    super::wayland_idle::{get_idle_time as get_wayland_idle_time, initialize_wayland},
    std::env,
    std::path::Path,
    uzers::get_current_uid,
};

use std::time::Duration;
pub fn get_idle_time() -> Duration {
    #[cfg(target_os = "linux")]
    {
        if cfg!(target_os = "linux") {
            if is_wayland() {
                let _ = initialize_wayland();

                return Duration::new(get_wayland_idle_time(), 0);
            } else if is_x11() {
                return match user_idle::UserIdle::get_time() {
                    Ok(duration) => duration.duration(),
                    _ => Duration::from_secs(0),
                };
            } else {
                return Duration::from_secs(0);
            };
        }
    }
    return match user_idle::UserIdle::get_time() {
        Ok(duration) => duration.duration(),
        _ => Duration::from_secs(0),
    };
}
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    if let Ok(true) = env::var("XDG_SESSION_TYPE").map(|v| v == "wayland") {
        return true;
    } else if let Ok(_) = env::var("WAYLAND_DISPLAY") {
        return Path::new(&format!("/run/user/{}/wayland-0", get_current_uid())).exists();
    }
    false
}

#[cfg(target_os = "linux")]
fn is_x11() -> bool {
    x11rb::connect(None).is_ok()
}
