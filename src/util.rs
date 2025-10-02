use std::{env, path::Path};

use uzers::get_current_uid;

pub fn is_wayland() -> bool {
    if let Ok(true) = env::var("XDG_SESSION_TYPE").map(|v| v == "wayland") {
        return true;
    } else if let Ok(_) = env::var("WAYLAND_DISPLAY") {
        return Path::new(&format!("/run/user/{}/wayland-0", get_current_uid())).exists();
    }
    false
}
