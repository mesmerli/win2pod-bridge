# Microsoft Store Listing - English (en-US)
# Win2Pod Bridge - Version 0.1.16

## 0. What's New in This Version (Release Notes - Paste into Partner Center)
```text
[Win2Pod Bridge Major Update - Version 0.1.16]
• 🔔 Redesigned Two-Line Compact System Tray Tooltip: Re-architected system tray tooltips into a clean, dedicated two-line layout. Line 1 focuses on HomePod / AirPlay streaming state and volume (e.g., "🔊 Streaming: Living Room (9%)" or Muted); Line 2 provides AirPods real-time wear and battery levels (e.g., "🎧 AirPods 3: ● L 100%"). Eliminates clutter and horizontal truncation.
• 🔄 Dynamic AirPods ↔ HomePod State Reflection: Tooltip dynamically reflects local enhanced playback when earbud priority handoff activates, seamlessly resuming HomePod streaming indicators upon case storage.
• 🛡️ Auto Pre-Amp Dynamic Headroom & Anti-Clipping: The 10-band equalizer now features intelligent dynamic headroom management. Boosting any frequency band automatically applies an equivalent pre-amp attenuation based on the maximum gain boost, paired with a transparent soft peak limiter to completely eliminate digital clipping distortion while preserving pristine, uncolored acoustic clarity.
• 🔄 Global Audio Endpoint Fallback & Self-Healing: Added system-wide audio endpoint liveness protection. If default speakers or proxy devices are disconnected or powered off before application exit or during routing, Win2Pod automatically fails over to another active physical playback device, ensuring Windows audio is never stuck or left unassigned.
• 🎧 AirPods Audio & Volume State Isolation: Implemented dynamic hardware GUID resolution to fix a volume/mute discrepancy when transitioning from HomePod to AirPods, perfectly synchronizing real earbud volume and stem touch adjustments.
• ⚡ Event-Driven Media Handoff & Accurate Auto-Resume: Upgraded earbud-in-case detection and playback restitution to an event-driven convergence model, reliably resuming media playback upon restoring local audio routes within 30 seconds.
• 🛡️ Anti-Feedback Loop Protection: Accurately identifies loopback risks in single-device environments via hardware GUID comparison, preventing audio feedback squeals while continuing to drive live visualizer spectrums with zero-CPU smart sleep.
• 🚀 Architectural Refactoring & Performance Optimizations: Streamlined route switching and state detection code paths for improved long-term streaming reliability and responsiveness.
```

## 1. Product Name
```text
Win2Pod Bridge
```

---

## 2. Short Description (Max 270 characters)
*(Displays in search results, category pages, and promotional cards)*

```text
Stream Windows 10 & 11 audio to Apple HomePod & AirPlay 2 speakers in lossless ALAC quality. Features smart AirPods ↔ HomePod handoff, tray earbud wear & battery indicators, Win2Pod Motion 3D visualizer, stereo balance, HomePod touch gestures, Harmonic Exciter & 10-band EQ.
```
*(Character count: ~268 chars, comfortably under the 270-character limit)*

---

## 3. Full Description (Store Overview)
*(Copy and paste into the Description field in Microsoft Partner Center)*

```text
Transform your Windows PC into a powerful AirPlay 2 high-fidelity audio transmitter!

Win2Pod Bridge is a modern, lightweight, and high-performance audio bridge engineered specifically for Windows 10 and Windows 11. It captures your desktop or application audio via native low-latency WASAPI loopback and streams it in bit-perfect, lossless ALAC quality directly to Apple HomePod, HomePod mini, Apple TV, and AirPlay 2-compatible speakers.

Windows cannot natively send AirPlay 2 audio, and Apple HomePods strictly mandate HomeKit transient encrypted pairing—causing most standard third-party tools to fail. Win2Pod Bridge solves this completely by implementing the software-only transient pairing protocol in pure Rust. No hardware modifications, no MFi chips, and no complex third-party background services required!

============================================================
🌟 KEY FEATURES & CAPABILITIES
============================================================

• 🛡️ Auto Pre-Amp Dynamic Headroom & 10-Band Professional Equalizer:
  - The 10-band equalizer features intelligent dynamic headroom management. Boosting any frequency band automatically applies an equivalent pre-amp attenuation based on max boost, paired with a transparent soft peak limiter to completely eliminate digital clipping distortion.
• 🔄 Global Audio Endpoint Fallback & Self-Healing:
  - System-wide audio endpoint liveness protection. If default speakers or proxy devices are disconnected before exit or during routing, Win2Pod automatically fails over to another active physical playback device, ensuring Windows audio is never stuck or left unassigned.
• 🔔 Compact Two-Line System Tray Tooltip & Real-Time Monitoring:
  - Clean two-line notification layout: Line 1 focuses on HomePod / AirPlay streaming state and volume (e.g. "🔊 Streaming: Living Room (9%)" or Muted); Line 2 provides AirPods wear status and per-ear battery levels (e.g. "🎧 AirPods 3: ● L 100%"). Eliminates text truncation and visual clutter.
• Symmetrical Audio Lifecycle Restitution:
  - Automatically snapshots the Windows initial default audio endpoint on startup and 100% faithfully restores the original playback device, hardware volume, and mute states upon application exit.
• 🛡️ Anti-Feedback Loop Protection:
  - Accurately identifies loopback risks in single-device environments via hardware GUID comparison, preventing audio feedback squeals while continuing to drive live visualizer spectrums with zero-CPU smart sleep.

• Smart AirPods ↔ HomePod Handoff & Dedicated Audio Routing:
  - Put on Earbuds: Audio seamlessly transfers to AirPods in real-time with local DSP spatial enhancement; automatically snapshots the interrupted HomePod route.
  - Earbuds in Case & Lid Closed: Seamlessly hands audio back to HomePod based on the snapshot! If target is unavailable or no snapshot exists, cleanly falls back to initial idle state, restoring full volume and mute control to the user.
  - Independent Source vs Route Snapshotting: Changing media players (such as refreshing Spotify capture source) never purges output route handoff snapshots.
• Symmetric Audio Shield Architecture:
  - Precisely records original endpoint volume and mute states upon entering protection and restores them cleanly on exit.
  - Completely eliminates previous volume lockups at 100% or driver-enforced mute clamping when transitioning back to local speakers.
• System Tray Real-Time AirPods Wear & Battery Indicator:
  - Context menu cleanly displays real-time per-ear wear status, battery percentages, and charging indicators (e.g. 🎧 Worn(● L:100%⚡  ● R:100%⚡  🔋100%⚡)).
  - Includes clear onboarding guidance reminding users to pair AirPods in Windows Bluetooth Settings.

🎨 WIN2POD MOTION - 3D IMPRESSIONIST AUDIO VISUALIZER
• Vibrant Audio Motion (Paint Pulse): Real-time audio reactive 3D sphere scattering from deep red bass bursts to violet treble glows, pulsing and dispersing with rhythm and melody.
• Impressionist Painterly Pass (PainterlyPass): World-anchored 3D brush stroke technology with adjustable stroke size, orbital camera flow, and auto-rotation.
• Standalone Fullscreen & Always on Top: Switch to immersive full screen (F key) or pin on top (📌). Automatically inhibits system and display sleep during fullscreen motion.
• Interactive Keyboard Shortcuts Overlay (Press ? key): Cleanly categorized into system controls and animation parameters.
• 100% Offline & Embedded: Fully embedded Three.js and shader pipelines with zero external network requests for instantaneous startup.
• Real-time Multilingual Sync: Supports 11 languages with live instant hot-switching without restarting the visualizer window.

🎛️ BIDIRECTIONAL SMART MEDIA REMOTE CONTROL
• Full Support for HomePod Physical Top Touch Surface Gestures:
  - Single tap: Play / Pause toggle
  - Double tap: Next Track
  - Triple tap: Previous Track
• Full Support for iPhone / iPad Apple Home App & Control Center cards for remote playback & track navigation!
• Standard AirPlay 2 updateMRSupportedCommands protocol capability advertisement with encrypted reverse events channel, seamlessly controlling Windows Global System Media Transport Controls (GSMTC) and media hotkeys.
• Real-time progress timeline (SET_PARAMETER progress) and DMAP track metadata synchronized to HomePod.

✨ HARMONIC EXCITER (HE) - HIGH-FREQUENCY RESTORATION
• Airiness & Crisp Detail Recovery: Designed specifically for compressed streaming, restoring high-frequency overtone details from 8 kHz to 20 kHz.
• Three Handcrafted Profiles:
  - Natural: Gentle high-end lift (+1.2dB Pre-EQ / 20% harmonics), ideal for classical, jazz, podcasts, and Hi-Res acoustic tracks.
  - Dynamic: Balanced excitation (+2.5dB Pre-EQ / 38% harmonics), universally recommended for pop, rock, and streaming playlists.
  - Brilliant: Maximum clarity (+4.0dB Pre-EQ / 55% harmonics), injecting shimmering sparkle, perfect for electronic music and older recordings.

📻 WARMTH ENHANCER (WE) - ANALOG VINYL EMBODIMENT
• Vintage Vinyl & Tape Acoustic Character: Relieves harsh digital edge with analog acoustic warmth.
• Physical Low-End Resonance (Low-Shelf Filter): +1.5dB to +2.0dB boost at 50Hz–90Hz, simulating turntable mechanical tonearm mass resonance.
• Analog Saturation Drive: 2.5% to 5.0% subtle saturation reproducing tube and tape warm harmonics.
• Integrated Peak Soft Limiter: Guarantees deep bass punch without digital clipping.
• Three Vinyl Moods: Subtle, Warm, and Vintage.

🎛️ MASTERING-GRADE AUDIO DSP CHAIN (HE ➔ EQ ➔ WE)
• Scientific Signal Processing Order: Exciter first (HE) -> 10-band graphic equalizer (EQ) -> Vinyl warmth and soft limiter (WE).
• Silence Fast-Bypass: DSP enters complete bypass during silence with 0% CPU overhead, waking up instantaneously in 0ms when playback resumes.

📊 POST-DSP REAL-TIME SPECTRUM ANALYZER
• 16-Band Real-Time Dynamic Monitoring: Displays the true energy spectrum after HE, EQ, and WE processing.
• Instant Visual Feedback: Watch EQ boosts, high-frequency exciter sparkle, and bass harmonics dance across the spectrum in real time.

⚡ ADAPTIVE FRAMERATE & ULTRA-LOW RESOURCE CONSUMPTION
• Dynamic Adaptive Framerate: Silky smooth 30 FPS during music playback, automatically dropping down to 2 FPS when idle, reducing window CPU to ~0.5%!
• Minimize to System Tray (0.0% CPU): Zero rendering overhead when minimized, consuming only ~8–15 MB RAM in the background.

📻 WIN2POD PLAYER - VINTAGE HI-FI COMPACT CONSOLE
• Classic Hi-Fi Aesthetic: Integrates album art, dynamic VFD status display, 16-band spectrum analyzer, and 10-band equalizer.
• Real-time GSMTC Track Info: Live song title, artist, and millisecond-accurate timeline progress bar.
• Quick Launch Source Media Player: Click album art or song title to bring Spotify, Apple Music, or Tidal directly to the front.
• Dynamic Pointer Stereo Balance (◄ Center ►): Pure 32-bit float software DSP balance control with single-click center reset.

📌 ALWAYS ON TOP (PIN TO DESKTOP)
• Instant Topmost Pinning: One-click pin button (📌) keeps the control panel or retro player floating above games and workflows.

🎯 PER-APP AUDIO ROUTING
• Stream Only What You Want: Route music from Spotify or browser to HomePod, keeping game sound effects and Discord voice chat on your PC headset.

🎵 ENGINEERED DEEPLY FOR APPLE HOMEPOD & AIRPLAY 2
• HomeKit Transient Encryption: Full handshake support for latest HomePod & HomePod mini firmware.
• Bit-Perfect ALAC Compression: 44.1 kHz / 16-bit stereo lossless audio packets.
• IEEE 802.1AS / gPTP Clock Sync: Mac-grade BMCA time synchronization eliminating audio jitter and drift.

🔊 HOMEPOD STEREO PAIR & MULTI-ROOM BROADCASTING
• Automatic Stereo Pair Discovery: Detects matched left/right HomePods as a unified stereo system.
• Multi-Room Broadcasting: Stream audio simultaneously to multiple AirPlay 2 speakers across rooms.
• Independent & Master Volume: Adjust volume across all devices together or fine-tune individual speaker volumes.

============================================================
SYSTEM REQUIREMENTS
============================================================
• OS: Windows 10 (Version 1809 or higher) or Windows 11 (64-bit).
• Network: Windows PC and HomePod / AirPlay speakers must be connected to the same local area network (Wi-Fi or Ethernet).
• Windows Firewall: Allow inbound network traffic when prompted so HomePod can establish reverse RTSP and PTP clock sync.
```

---

## 4. Product Features (Max 20 items, each within 120 characters)
*(Copy each item into the Features field in Partner Center)*

1. `Smart AirPods ↔ HomePod handoff: Wear earbuds to route instantly; store in case with closed lid to hand back to HomePod`
2. `Real-time system tray battery & wear indicator: View per-earbud wear status and battery percentages at a glance`
3. `Symmetric audio shield architecture: Cleanly restores endpoint states on exit with zero volume lockups or forced mutes`
4. `Click album art or track title to wake and foreground media source app (Spotify, Apple Music, Tidal, etc.)`
5. `Dynamic pointer stereo balance (◄ Center ►): Pure 32-bit software DSP with one-click center reset`
6. `Win2Pod Motion: 3D impressionist audio visualizer with fullscreen, topmost pinning, and sleep inhibition`
7. `Lossless 44.1 kHz / 16-bit stereo ALAC real-time audio streaming directly to Apple HomePod & AirPlay 2 speakers`
8. `Pure Rust HomeKit transient encrypted pairing: No MFi chips or third-party background services required`
9. `Harmonic Exciter (HE): High-frequency overtone recovery with Natural, Dynamic, and Brilliant presets`
10. `Warmth Enhancer (WE): Turntable low-shelf physical resonance and analog saturation with Peak soft limiter`
11. `Scientific audio DSP mastering chain (HE ➔ EQ ➔ WE) with 10-band graphic EQ and silence fast-bypass`
12. `Post-DSP real-time 16-band spectrum analyzer showing true post-processing audio dynamics`
13. `Adaptive framerate: Drops to 2 FPS when idle, reducing window CPU consumption to ~0.5%`
14. `Win2Pod Player: Vintage Hi-Fi console with live GSMTC track metadata, album cover, and smooth seek bar`
15. `Always on Top (📌): Float main panel or player window above all apps with tray wake preservation`
16. `Per-App audio routing: Stream specific music apps to HomePod while keeping gaming and voice calls on PC headset`
17. `10-band graphic equalizer with ±12 dB fine adjustment and popular genre presets (Bass Boost, Vocal, etc.)`
18. `Automatic HomePod stereo pair recognition for immersive, room-filling soundstage`
19. `Multi-room synchronized broadcasting: Play the same audio across multiple AirPlay 2 speakers simultaneously`
20. `Two-way physical volume synchronization: Physical touch volume adjustments reflect live in Windows GUI`

---

## 5. Search Terms / Keywords (Max 7 items, each within 30 characters)
*(Copy into the Search terms field in Partner Center)*

1. `Audio`
2. `AirPlay`
3. `Cast`
4. `Equalizer`
5. `Music`
6. `Player`
7. `Streamer`
