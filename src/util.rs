use std::{env, path::Path};

use uzers::get_current_uid;

pub fn is_wayland() -> bool {
    if let Ok(true) = env::var("XDG_SESSION_TYPE").map(|v| v.to_lowercase() == "wayland") {
        return true;
    } else if let Ok(_) = env::var("WAYLAND_DISPLAY") {
        return Path::new(&format!("/run/user/{}/wayland-0", get_current_uid())).exists();
    }
    false
}

pub fn is_mutter() -> bool {
    if let Ok(true) = env::var("XDG_CURRENT_DESKTOP").map(|v| v.to_lowercase() == "gnome") {
        return true;
    } else if let Ok(_) = env::var("GNOME_SHELL_SESSION_MODE") {
        return true;
    }
    false
}
