use crate::config::Config;
use anyhow::Result;
pub use zeroclaw_misc::hardware::*;

pub fn handle_command(cmd: crate::HardwareCommands, _config: &Config) -> Result<()> {
    #[cfg(not(feature = "hardware"))]
    {
        let _ = &cmd;
        println!("Hardware discovery requires the 'hardware' feature.");
        println!("Build with: cargo build --features hardware");
        Ok(())
    }

    #[cfg(all(
        feature = "hardware",
        not(any(target_os = "linux", target_os = "macos", target_os = "windows"))
    ))]
    {
        let _ = &cmd;
        println!("Hardware USB discovery is not supported on this platform.");
        println!("Supported platforms: Linux, macOS, Windows.");
        return Ok(());
    }

    #[cfg(all(
        feature = "hardware",
        any(target_os = "linux", target_os = "macos", target_os = "windows")
    ))]
    match cmd {
        crate::HardwareCommands::Discover => run_discover(),
        crate::HardwareCommands::Introspect { path } => run_introspect(&path),
        crate::HardwareCommands::Info { chip } => run_info(&chip),
    }
}

#[cfg(all(
    feature = "hardware",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn run_discover() -> Result<()> {
    let devices = discover::list_usb_devices()?;

    if devices.is_empty() {
        println!("No USB devices found.");
        println!();
        println!("Connect a board (e.g. Nucleo-F401RE) via USB and try again.");
        return Ok(());
    }

    println!("USB devices:");
    println!();
    for d in &devices {
        let board = d.board_name.as_deref().unwrap_or("(unknown)");
        let arch = d.architecture.as_deref().unwrap_or("—");
        let product = d.product_string.as_deref().unwrap_or("—");
        println!(
            "  {:04x}:{:04x}  {}  {}  {}",
            d.vid, d.pid, board, arch, product
        );
    }
    println!();
    println!("Known boards: nucleo-f401re, nucleo-f411re, arduino-uno, arduino-mega, cp2102");

    Ok(())
}

#[cfg(all(
    feature = "hardware",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn run_introspect(path: &str) -> Result<()> {
    let result = introspect::introspect_device(path)?;

    println!("Device at {}:", result.path);
    println!();
    if let (Some(vid), Some(pid)) = (result.vid, result.pid) {
        println!("  VID:PID     {:04x}:{:04x}", vid, pid);
    } else {
        println!("  VID:PID     (could not correlate with USB device)");
    }
    if let Some(name) = &result.board_name {
        println!("  Board       {}", name);
    }
    if let Some(arch) = &result.architecture {
        println!("  Architecture {}", arch);
    }
    println!("  Memory map  {}", result.memory_map_note);

    Ok(())
}

#[cfg(all(
    feature = "hardware",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn run_info(chip: &str) -> Result<()> {
    #[cfg(feature = "probe")]
    {
        match info_via_probe(chip) {
            Ok(()) => return Ok(()),
            Err(e) => {
                println!("probe-rs attach failed: {}", e);
                println!();
                println!(
                    "Ensure Nucleo is connected via USB. The ST-Link is built into the board."
                );
                println!("No firmware needs to be flashed — probe-rs reads chip info over SWD.");
                return Err(e.into());
            }
        }
    }

    #[cfg(not(feature = "probe"))]
    {
        println!("Chip info via USB requires the 'probe' feature.");
        println!();
        println!("Build with: cargo build --features hardware,probe");
        println!();
        println!("Then run: zeroclaw hardware info --chip {}", chip);
        println!();
        println!("This uses probe-rs to attach to the Nucleo's ST-Link over USB");
        println!("and read chip info (memory map, etc.) — no firmware on target needed.");
        Ok(())
    }
}

#[cfg(all(
    feature = "hardware",
    feature = "probe",
    any(target_os = "linux", target_os = "macos", target_os = "windows")
))]
fn info_via_probe(chip: &str) -> anyhow::Result<()> {
    use probe_rs::config::MemoryRegion;
    use probe_rs::{Session, SessionConfig};

    println!("Connecting to {} via USB (ST-Link)...", chip);
    let session = Session::auto_attach(chip, SessionConfig::default())
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let target = session.target();
    println!();
    println!("Chip: {}", target.name);
    println!("Architecture: {:?}", session.architecture());
    println!();
    println!("Memory map:");
    for region in target.memory_map.iter() {
        match region {
            MemoryRegion::Ram(ram) => {
                let start = ram.range.start;
                let end = ram.range.end;
                let size_kb = (end - start) / 1024;
                println!("  RAM: 0x{:08X} - 0x{:08X} ({} KB)", start, end, size_kb);
            }
            MemoryRegion::Nvm(flash) => {
                let start = flash.range.start;
                let end = flash.range.end;
                let size_kb = (end - start) / 1024;
                println!("  Flash: 0x{:08X} - 0x{:08X} ({} KB)", start, end, size_kb);
            }
            _ => {}
        }
    }
    println!();
    println!("Info read via USB (SWD) — no firmware on target needed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::hardware::load_hardware_context_from_dir;
    use std::fs;

    fn write(path: &std::path::Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn empty_dir_returns_empty_string() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(load_hardware_context_from_dir(tmp.path(), &[]), "");
    }

    #[test]
    fn hardware_md_only_returns_its_content() {
        let tmp = tempfile::tempdir().unwrap();
        write(&tmp.path().join("HARDWARE.md"), "# Global HW\npin 25 = LED");
        let result = load_hardware_context_from_dir(tmp.path(), &[]);
        assert!(result.contains("pin 25 = LED"), "got: {result}");
    }

    #[test]
    fn device_profile_loaded_for_matching_alias() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("devices").join("pico0.md"),
            "# pico0\nPort: /dev/cu.usbmodem1101",
        );
        let result = load_hardware_context_from_dir(tmp.path(), &["pico0"]);
        assert!(result.contains("/dev/cu.usbmodem1101"), "got: {result}");
    }

    #[test]
    fn device_profile_skipped_for_non_matching_alias() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("devices").join("pico0.md"),
            "# pico0\nPort: /dev/cu.usbmodem1101",
        );
        // No alias provided — device profile must not appear
        let result = load_hardware_context_from_dir(tmp.path(), &[]);
        assert!(!result.contains("pico0"), "got: {result}");
    }

    #[test]
    fn skills_loaded_and_sorted() {
        let tmp = tempfile::tempdir().unwrap();
        write(
            &tmp.path().join("skills").join("blink.md"),
            "# Skill: Blink\nuse device_exec",
        );
        write(
            &tmp.path().join("skills").join("gpio.md"),
            "# Skill: GPIO\ngpio_write",
        );
        let result = load_hardware_context_from_dir(tmp.path(), &[]);
        // blink.md sorts before gpio.md
        let blink_pos = result.find("device_exec").unwrap();
        let gpio_pos = result.find("gpio_write").unwrap();
        assert!(blink_pos < gpio_pos, "skills not sorted; got: {result}");
    }

    #[test]
    fn sections_joined_with_double_newline() {
        let tmp = tempfile::tempdir().unwrap();
        write(&tmp.path().join("HARDWARE.md"), "global");
        write(&tmp.path().join("devices").join("pico0.md"), "device");
        let result = load_hardware_context_from_dir(tmp.path(), &["pico0"]);
        assert!(result.contains("global\n\ndevice"), "got: {result}");
    }

    #[test]
    fn hardware_context_contains_device_exec_rule() {
        // Verify that the installed HARDWARE.md (from Section 3) contains
        // the device_exec rule so the LLM knows to use it for blink/loops.
        // This acts as the Section 5 BUG-2 behavioral gate.
        if let Some(home) = directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()) {
            let hw_md = home.join(".zeroclaw").join("hardware").join("HARDWARE.md");
            if hw_md.exists() {
                let content = fs::read_to_string(&hw_md).unwrap_or_default();
                assert!(
                    content.contains("device_exec"),
                    "HARDWARE.md must mention device_exec for blink/loop operations; got: {content}"
                );
            }
        }
    }
}
