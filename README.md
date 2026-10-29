# Win2Pod Bridge

English • [繁體中文](README.zh-TW.md)

**Win2Pod Bridge** is a high-performance Windows audio streaming bridge designed specifically for **Apple HomePod, HomePod mini, Apple TV**, and AirPlay 2-compatible audio receivers.

Windows cannot natively send AirPlay audio, and Apple HomePods mandate HomeKit transient encrypted pairing—causing most standard AirPlay 1 tools to fail. Written purely in Rust, **Win2Pod Bridge** captures system audio via low-latency WASAPI loopback, completes the software-only transient pairing handshake (without requiring Apple MFi hardware chips), and streams bit-perfect, lossless ALAC audio over AirPlay 2.

---

## ✨ Features

### 1. 🎵 Engineered for Apple HomePod & AirPlay 2
- **HomeKit Transient Pairing**: Fully implements SRP-6a, Curve25519, and ChaCha20-Poly1305 software-level cryptographic handshakes. Compatible with the latest HomePod and HomePod mini firmware.
- **Lossless Real-time ALAC Compression**: Encodes audio in real time using Apple Lossless Audio Codec (ALAC) at 44.1 kHz, 16-bit stereo for bit-perfect audio delivery.
- **PTP High-Precision Timing (IEEE 802.1AS / gPTP)**: Features Mac-style BMCA clock negotiation and yielding, eliminating audio jitter, drift, and synchronization delays.

### 2. 🔊 HomePod Stereo Pairs & Multi-Room Streaming
- **Automatic Stereo Pair Recognition**: Automatically detects paired left/right HomePods as a single unified stereo speaker group for immersive playback.
- **Multi-Room Group Streaming**: Broadcast your Windows audio to multiple AirPlay speakers across rooms simultaneously.
- **Dynamic Group Management**: Seamlessly add (`+ Add to group`) or remove speakers on the fly without interrupting ongoing playback.
- **Independent & Master Volume Controls**: Adjust master volume or fine-tune individual volume sliders per speaker during group playback.

### 3. 🎨 Modern Fluent Flyout GUI & System Tray Integration
- **Windows 11 Fluent UI**: Clean, modern flyout interface built with Rust (`egui`/`eframe`) that opens instantly from the taskbar notification area.
- **State-Aware Tray Icon**: Real-time visual feedback for connection states (Idle gray ring, Connecting orange ring, Streaming green ring, Error red ring) with native balloon notifications.
- **Extreme Resource Efficiency**: UI rendering is completely suspended and working set memory is trimmed to ~10–20 MB RAM when minimized to the tray.
- **Silent Background Startup**: Supports Windows startup flags (`--startup` / `--minimized` / `-m`) to launch silently to the tray on boot.

### 4. 🎛️ Audio Routing & Split Mode
- **System Mix Capture (WASAPI Loopback)**: Capture all PC desktop sounds out of the box.
- **Audio Source Selection (Split Mode)**: Select a specific audio output device/endpoint as the capture source instead of the entire system mix. For example, route Spotify or Apple Music to your HomePod while games, Discord, or calls stay on your PC headphones.
- **Quick Volume Mixer Access**: Direct shortcut button to open Windows App Volume Mixer (`ms-settings:apps-volume`).

### 5. ⚡ Smart Controls & Power Management
- **Auto-Mute PC Speakers**: Automatically mutes local speakers during streaming to eliminate echo and delay dissonance, faithfully restoring original volume upon disconnect or exit.
- **Smart Sleep Prevention**: Prevents Windows from going to sleep only while audio is actively playing, automatically releasing power locks when silent.
- **Low-Latency Mode**: Toggleable low-latency buffering for responsive audio playback.
- **Auto-Connect**: Remembers your preferred speaker and automatically reconnects upon startup.
- **Customizable Global Hotkey**: Configurable system-wide shortcut (default `Ctrl + Alt + H`, with alternatives like `Alt + H`, `Win + Alt + H`, `F9`, `F10`) to toggle streaming on/off from anywhere.

### 6. 🌐 11 Built-in Languages
- Full localization for: English, Traditional Chinese (繁體中文), Simplified Chinese (简体中文), French, German, Italian, Japanese, Korean, Brazilian Portuguese, Russian, and Spanish.
- Easily toggle languages via the dropdown menu in **Settings ⚙** (defaults to auto-detecting your Windows system language).

---

## 📋 Requirements

1. **Operating System**: Windows 10 (version 1809 or later) or Windows 11 (64-bit).
2. **Network**: PC and HomePod / AirPlay speakers must be connected to the **same local network (Wi-Fi or LAN)**.
3. **Windows Firewall Rule**:  
   AirPlay 2 requires speakers to connect back to the sender's RTSP and timing ports. On first launch, allow incoming connections or run PowerShell as Administrator:

```powershell
netsh advfirewall firewall add rule name="Win2Pod Bridge" dir=in action=allow program="C:\path\to\win2pod.exe" enable=yes profile=any
```

---

## 🚀 Usage

1. Launch `win2pod.exe` (or launch from the Start Menu if installed via MSIX).
2. Click the system tray icon to open the flyout control panel. The app automatically discovers AirPlay devices via mDNS.
3. Click **Connect** next to your target speaker to start streaming Windows audio immediately!
4. Click **⚙** in the top-right corner to open Settings and configure audio sources, auto-mute, low latency, hotkeys, and language.

---

## 🛠️ Build from Source

Requires the Rust **GNU** toolchain and **MinGW-w64** (`gcc`, `dlltool`) on `PATH`:

```powershell
# 1. Switch to Rust GNU toolchain
rustup default stable-x86_64-pc-windows-gnu

# 2. Build standalone portable release executable (output -> dist\win2pod.exe)
.\build.ps1

# 3. Or build self-signed MSIX package (requires Windows SDK makeappx / signtool)
.\build-msix.ps1
```

---

## 📜 Acknowledgements & License

This project builds upon foundational research and code from [**airplay2-rs**](https://github.com/lmcgartland/airplay2-rs) by Lucas McGartland, extended with:
- Pure Rust Fluent flyout GUI & system tray management (`crates/homepod-cast-gui`)
- Windows native WASAPI Loopback capture pipeline and real-time ALAC encoder
- IEEE 802.1AS / gPTP clock synchronization and multi-room group streaming architecture
- MSIX packaging, Microsoft Store licensing integration, and 11-language localization

**License**: GPL-2.0, inherited from upstream `airplay2-rs`. See [`LICENSE`](LICENSE).
