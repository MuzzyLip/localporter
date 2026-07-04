# 🛡️ LocalPorter

> Your system's native gatekeeper for local network ports and active processes.

**LocalPorter** is a blazing-fast, cross-platform local port monitor and process manager built entirely with a native Rust stack. Living quietly in your macOS menu bar or Windows/Linux system tray, it provides instant visibility into your local network activity and gives you the power to manage rogue processes with a single click. Say goodbye to typing `netstat` `lsof -i` and `kill -9` over and over again!

## ✨ Features

- **⚡️ Lightweight & Native**: Built entirely in Rust without heavy webviews (No Electron or Tauri Webviews). It guarantees minimal memory footprint and zero UI lag.
- **🌐 True Cross-Platform Experience**:
  - **macOS**: Seamlessly integrates into the Menu Bar with native-feeling Popover interactions.
  - **Windows / Linux**: Supports standard system tray interactions with a centered, floating dashboard.
  - **Detachable Panel**: Pin or detach the monitoring panel into a standalone window for continuous observation.
- **📊 Real-Time Telemetry**:
  - Automatically maps listening ports to their respective Process IDs (PID).
  - Displays live execution commands, working directories, and uptime.
  - Monitors **CPU usage**, **RAM consumption**, and macOS-style **Energy Impact** ratings.
- **⚔️ Process Management**: Instantly kill a specific stubborn process or nuke all processes occupying a specific port with a single click.

## 🛠️ Tech Stack

- **GUI Framework**: [egui](https://github.com/emilk/egui) (Highly performant, immediate mode GUI)
- **Tray Integration**: [tray-icon](https://github.com/tauri-apps/tray-icon) (System tray & menu bar support)
- **System Telemetry**: [sysinfo](https://github.com/GuillaumeGomez/sysinfo) & `netstat2` (Cross-platform system information)

## 🚀 Getting Started

### Prerequisites

Ensure you have the latest stable Rust toolchain installed (1.75+ recommended).

### Installation & Build

```bash
git clone [https://github.com/your-username/LocalPorter.git](https://github.com/your-username/LocalPorter.git)
cd LocalPorter

# Build in release mode for optimal performance
cargo build --release

# Run the application
cargo run --release
```
