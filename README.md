# Win2Pod Bridge

Stream **all your Windows PC audio to a HomePod** (or any AirPlay 2 speaker) with a modern GUI and lightweight system-tray app.

Windows can't send AirPlay on its own, and a HomePod requires the encrypted AirPlay 2 handshake (HomeKit transient pairing) — which is why most tools don't work with it. **Win2Pod Bridge** captures your system audio via low-latency WASAPI loopback and streams it smoothly over AirPlay 2.

```
  ◯  (tray icon: ring = stopped, ● dot = streaming)
  ┌──────────────────────┐
  │ ■ Stopped         ✓  │   ← single-tick: exactly one is true
  │ ▶ Living Room        │
  │ ▶ Kitchen            │
  │ Settings…            │   ← volume slider + custom toggle hotkey
  │ Quit                 │
  └──────────────────────┘
```

## Features

- **Modern GUI & System Tray**: One-click streaming to any discovered AirPlay 2 speaker.
- **State-aware Tray Icon**: Hollow ring = stopped, filled dot = streaming.
- **Volume Slider & Auto-mute**: Live volume adjustment (defaults to a safe 25%), with optional auto-mute of PC speakers while streaming.
- **Custom Global Hotkey**: Press your own shortcut (defaults to `Ctrl+Alt+H`) to toggle streaming on/off from anywhere.
- **Multi-language Support**: Built-in English and Traditional Chinese (繁體中文).
- **Single Executable & MSIX Package**: Standalone portable ~15 MB executable or MSIX packaged for Windows 10/11.

## Requirements

- Windows 10 / 11
- PC and speakers connected to the **same local network (Wi-Fi/LAN)**
- A one-time **Windows Firewall** inbound rule (AirPlay 2 has the speaker connect back to the PC):

```powershell
netsh advfirewall firewall add rule name="Win2Pod Bridge" dir=in action=allow program="C:\path\to\win2pod.exe" enable=yes profile=any
```

## Usage

1. Run `win2pod.exe` (or launch via Start Menu if installed via MSIX).
2. Right-click the tray icon → pick your speaker. Play audio on Windows and it streams immediately.
3. Open **Settings** to adjust volume, configure the global hotkey, switch language, or toggle auto-mute.

## Build from Source

Requires the Rust **GNU** toolchain and **MinGW-w64** (`gcc` + `dlltool`) on `PATH`:

```powershell
rustup default stable-x86_64-pc-windows-gnu

# Build portable release executable (output -> dist\win2pod.exe)
.\build.ps1

# Or build signed MSIX package (requires Windows SDK makeappx)
.\build-msix.ps1
```

## Upstream & Acknowledgements

This repo builds upon [**airplay2-rs**](https://github.com/lmcgartland/airplay2-rs) by Lucas McGartland and subsequent Windows portability improvements, adding:
- `crates/homepod-cast-gui` — Modern `egui`/`eframe` GUI, tray management, and Windows Store integration
- Windows WASAPI loopback audio capture and real-time ALAC encoder pipeline
- Packaging scripts for MSIX and asset generation

## License

GPL-2.0, inherited from upstream `airplay2-rs`. See [`LICENSE`](LICENSE).
