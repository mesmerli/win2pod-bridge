# Microsoft Store Listing - English (Win2Pod Bridge)

## 1. Product Name
```text
Win2Pod Bridge
```

---

## 2. Short Description (Max 270 characters)
*(Displays in search results, category pages, and promotional cards)*

```text
Stream your Windows 10 & 11 desktop audio directly to Apple HomePod, HomePod mini, and AirPlay 2 speakers in lossless ALAC quality with low latency, multi-room sync, and modern Fluent controls.
```
*(Character count: ~207 chars)*

---

## 3. Full Description (Store Overview)
*(Copy and paste into the Description field in Microsoft Partner Center)*

```text
Transform your Windows PC into a seamless AirPlay 2 audio transmitter! 

Win2Pod Bridge is a modern, lightweight, and high-performance audio bridge engineered specifically for Windows 10 and Windows 11. It captures your desktop audio via native low-latency WASAPI loopback and streams it in bit-perfect, lossless ALAC quality directly to Apple HomePod, HomePod mini, Apple TV, and AirPlay 2-compatible speakers.

Windows cannot natively send AirPlay 2 audio, and Apple HomePods strictly mandate HomeKit transient encrypted pairing—causing most standard third-party tools to fail. Win2Pod Bridge solves this completely by implementing the software-only transient pairing protocol in pure Rust. No hardware modifications, no MFi chips, and no complex third-party background services required!

============================================================
KEY HIGHLIGHTS & FEATURES
============================================================

🎵 BUILT SPECIFICALLY FOR APPLE HOMEPOD & AIRPLAY 2
• HomeKit Transient Pairing: Native encrypted handshake compatible with all modern HomePod & HomePod mini firmware.
• Bit-Perfect Lossless ALAC: Encodes real-time audio using Apple Lossless Audio Codec (44.1 kHz, 16-bit stereo) for studio-grade acoustic clarity.
• IEEE 802.1AS / gPTP Clock Sync: Mac-grade PTP clock negotiation eliminates audio jitter, drift, and sync delay.

🔊 HOMEPOD STEREO PAIRS & MULTI-ROOM GROUP STREAMING
• Automatic Stereo Pair Recognition: Automatically detects paired left and right HomePods as a unified stereo group.
• Multi-Room Audio: Broadcast your PC audio to multiple AirPlay speakers across rooms simultaneously.
• Dynamic Group Controls: Add or remove speakers on the fly without stopping playback.
• Independent & Master Volume: Adjust master volume or fine-tune individual volume sliders per speaker.

🎨 MODERN WINDOWS 11 FLUENT UI & SYSTEM TRAY
• Fluent Flyout Panel: Clean, modern floating window that matches your Windows 11 desktop theme.
• State-Aware Tray Icon: Real-time visual feedback (Idle, Connecting, Streaming, Error) right in your taskbar.
• Ultra-Low Resource Usage: Suspends UI rendering and trims memory down to ~10–20 MB RAM when minimized to the tray.
• Silent Boot Support: Starts minimized to the tray automatically with Windows so it's always ready when you need it.

🎛️ AUDIO ROUTING & SPLIT MODE
• System-Wide Audio Mix: Stream all desktop sounds, browsers, games, and media players.
• Per-Endpoint Split Mode: Select a specific audio output device as your capture source. Stream music (Spotify, Apple Music) to HomePod while keeping gaming chat, Discord, or Teams calls on your PC headset.
• Quick Volume Mixer Access: One-click shortcut to Windows App Volume Mixer.

⚡ SMART PLAYBACK & POWER MANAGEMENT
• Smart Auto-Mute: Automatically mutes PC speakers while streaming to eliminate room echo and delay dissonance, restoring volume on disconnect.
• Intelligent Sleep Prevention: Prevents Windows from entering sleep mode only while audio is actively playing; releases power locks when silent.
• Low-Latency Tuning: Toggleable low-latency buffering for responsive multimedia playback.
• Auto-Connect: Remembers your favorite speaker and reconnects automatically upon startup.
• Global Hotkey: Press your custom shortcut (default: Ctrl + Alt + H) to toggle streaming instantly from anywhere.

🌐 MULTILINGUAL LOCALIZATION
Available in 11 languages: English, Traditional Chinese (繁體中文), Simplified Chinese (简体中文), French, German, Italian, Japanese, Korean, Brazilian Portuguese, Russian, and Spanish.

============================================================
SYSTEM REQUIREMENTS & NOTES
============================================================
• Windows 10 (version 1809 or higher) or Windows 11 (64-bit).
• PC and speakers must be connected to the same local network (Wi-Fi or LAN).
• Windows Firewall: Because AirPlay 2 speakers establish an inbound connection back to the PC for timing and RTSP controls, an inbound firewall permission rule is required on initial setup.
```

---

## 4. Product Features (Bullet points - up to 20 items, max 120 chars each)
*(Paste each line into the "Features" fields in Partner Center)*

1. Bit-perfect lossless ALAC audio streaming (44.1 kHz, 16-bit stereo)
2. HomeKit Transient Pairing: Native encryption support for Apple HomePod and HomePod mini
3. Automatic HomePod Stereo Pair recognition and synchronized stereo playback
4. Multi-room streaming: Broadcast audio to multiple AirPlay 2 speakers simultaneously
5. Dynamic group management with master volume and individual speaker volume controls
6. Audio Split Mode: Route specific apps (e.g. music) to HomePod while keeping calls on headphones
7. Native low-latency WASAPI loopback audio capture
8. High-precision IEEE 802.1AS (gPTP) timing sync to eliminate audio jitter and drift
9. Modern Windows 11 Fluent flyout interface and state-aware system tray icon
10. Ultra-lightweight background memory footprint (~10–20 MB RAM when minimized)
11. Auto-mute PC speakers during streaming to eliminate delay dissonance and echo
12. Smart sleep prevention only while active audio is playing
13. Customizable global hotkey (default Ctrl + Alt + H) to toggle streaming instantly
14. Auto-connect to last-used AirPlay speaker on application launch
15. Full localization in 11 international languages

---

## 5. Search Terms / Keywords (Up to 7 terms, max 30 chars each)
```text
1. HomePod
2. AirPlay
3. AirPlay Windows
4. HomePod Windows
5. Audio Streamer
6. Multi-Room Audio
7. ALAC Lossless
```

---

## 6. Copyright and Trademark Information
```text
Copyright © 2026 mesmerli. All rights reserved.
Apple, HomePod, HomePod mini, Apple TV, and AirPlay are trademarks of Apple Inc., registered in the U.S. and other countries. Win2Pod Bridge is an independent open-source project and is not affiliated with, endorsed by, or sponsored by Apple Inc.
```

---

## 7. Additional System Requirements / Notes
```text
• Both your Windows PC and your HomePod / AirPlay speakers must be connected to the same local network (LAN / Wi-Fi).
• Windows Firewall must allow inbound local connections for Win2Pod Bridge so speakers can synchronize timing and audio streams.
```

---

## 8. Release Notes / What's New (v0.1.0)
```text
Initial Release of Win2Pod Bridge:
• Lossless ALAC audio streaming to Apple HomePod & AirPlay 2 receivers.
• Automatic HomePod Stereo Pair detection and multi-room audio playback.
• Audio Split Mode for independent application audio routing.
• Modern Windows 11 Fluent flyout user interface with system tray integration.
• Smart auto-mute, active sleep prevention, and custom global hotkeys.
• 11 built-in languages with automatic system language detection.
```
