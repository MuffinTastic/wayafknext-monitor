use anyhow::anyhow;

mod idle_detection;
mod util;

fn main() -> anyhow::Result<()> {
    if !util::is_wayland() {
        return Err(anyhow!("Not running on Wayland!"));
    }

    

    Ok(())
}
