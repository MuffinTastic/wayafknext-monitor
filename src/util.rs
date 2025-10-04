use std::{
    env, io,
    path::{Path, PathBuf},
};

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

pub fn get_exe_dir() -> io::Result<PathBuf> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or(io::Error::new(
        io::ErrorKind::Other,
        "Couldn't get executable directory",
    ))?;
    Ok(exe_dir.to_owned())
}
