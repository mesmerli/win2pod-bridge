# Microsoft Store Listing - English (en-US)
# Win2Pod Bridge - Version 0.1.16

## 0. What's New in This Version (Release Notes - Paste into Partner Center)
```text
[Win2Pod Bridge 0.1.16 Major Update]
• 🔔 Redesigned Two-Line Compact System Tray Tooltip: Re-architected system tray tooltips into a clean, dedicated two-line layout. Line 1 focuses on HomePod / AirPlay streaming state and volume (e.g., "🔊 Streaming: Living Room (9%)" or Muted); Line 2 provides AirPods real-time wear and battery levels (e.g., "🎧 AirPods 3: ● L 100%"). Eliminates clutter and horizontal truncation.
• 🔄 Dynamic AirPods ↔ HomePod State Reflection: Tooltip dynamically reflects local enhanced playback when earbud priority handoff activates, seamlessly resuming HomePod streaming indicators upon case storage.
• 🛡️ Auto Pre-Amp Dynamic Headroom & Anti-Clipping: The 10-band equalizer features intelligent dynamic headroom management. Boosting any frequency band automatically applies an equivalent pre-amp attenuation based on the maximum gain boost, paired with a transparent soft peak limiter to completely eliminate digital clipping distortion while preserving pristine, uncolored acoustic clarity.
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
Stream Windows 10 & 11 audio to Apple HomePod & AirPlay 2 speakers in lossless ALAC quality. Features Win2Pod Motion 3D visualizer, click album art to open media player, dynamic pointer stereo balance, HomePod touch gestures, Harmonic Exciter & 10-band EQ.
```
*(Character count: ~262 chars, comfortably under the 270-character limit)*

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
• 🎧 Smart AirPods ↔ HomePod Handoff & Dedicated Audio Routing:
  - Automatically transfers audio seamlessly to AirPods when worn, and accurately hands playback back to HomePod when stored in the case!
• All-New Dynamic Pointer Stereo Balance (◄ Center ►):
  - Intuitive Direction Cue: Displays "◄ Center ►" at center, dynamically pointing "◄ L 30%" or "30% R ►" when adjusted.
  - Single-Click Center Reset: Click the text label or double-click the slider thumb to instantaneously reset back to exact center.
  - Pure 32-Bit Float Software DSP: Bypasses problematic Windows hardware driver volume ganging, guaranteeing 100% full dynamic range with zero digital clipping or volume decay.
• Quick Launch Source Media Player: Click the vintage vinyl cover or track title in Win2Pod Player to instantly wake up and foreground the source playback app (supports Spotify, Apple Music, Tidal, Chrome, Edge, etc.).
• Full Balance & Mono Support for Local Headphones / Audio Enhancement (AO Mode): Experience real-time stereo panning and seamless mono mix switching whether streaming to HomePod or listening through PC headphones.
• Symmetrical Audio Lifecycle Restitution & Anti-Feedback Shield:
  - Automatically snapshots the Windows initial default audio endpoint on startup and 100% faithfully restores the original playback device, hardware volume, and mute states upon application exit.
  - Accurately identifies loopback risks in single-device environments via hardware GUID comparison, preventing audio feedback squeals while continuing to drive live visualizer spectrums with zero-CPU smart sleep.

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
• Restore Lost Air & Sparkle: Specially designed to compensate for high-frequency rolloff in streaming, Bluetooth, and lossy compression formats. Smartly regenerates natural acoustic overtone harmonics from 8 kHz to 20 kHz.
• Three Mastered Presets:
  - Natural: Gentle presence enhancement (+1.2dB Pre-EQ / 20% harmonic drive), ideal for classical, vocal podcasts, and high-res acoustic recordings.
  - Dynamic: Balanced everyday enhancement (+2.5dB Pre-EQ / 38% harmonic drive), highly recommended for pop, rock, and modern streaming services.
  - Brilliant: Vibrant high-end shimmer (+4.0dB Pre-EQ / 55% harmonic drive), injects crisp transparency and sparkling presence into vintage tracks and low-bitrate audio.

📻 WARMTH ENHANCER (WE) - ANALOG VINYL ACOUSTICS
• Recreate Authentic Turntable Warmth: Emulates the physical acoustics of vintage vinyl turntables and analog magnetic tape, eliminating harsh digital fatigue.
• Low-Shelf Physical Resonance: Applies a subtle +1.5dB to +2.0dB boost at 50Hz–90Hz to emulate the natural mechanical resonance of the tonearm and platter, providing deep, rounded low-end body.
• Analog Saturation Drive: Injects 2.5%–5.0% analog warmth drive to emulate warm vacuum tube and magnetic tape harmonic overtones.
• Integrated Peak Soft Limiter: Ensures deep bass warmth without digital clipping or distortion.
• Three Vinyl Modes: Subtle, Warm, and Vintage.

🎛️ PROFESSIONAL AUDIO DSP PIPELINE (HE ➔ EQ ➔ WE)
• Audiophile Processing Chain: Restores delicate high-frequency overtones first (HE), shapes frequency balance to your preference (10-Band EQ), and finishes with analog warmth resonance and peak protection (WE).
• Silence Fast-Bypass (DSP Sleep Mode): When no audio is playing, DSP filters naturally ring down and enter a 100% sleep bypass mode, eliminating wasteful math operations. Instantly wakes up with 0ms latency when sound resumes!

📊 POST-DSP REAL-TIME SPECTRUM & OUTPUT MONITORING
• 16-Band Output Response: The 16-band live spectrum analyzer now monitors audio directly after the DSP processing chain, visualizing the exact acoustic profile delivered to your HomePod.
• Immediate Visual Feedback: Adjusting EQ bands or toggling HE / WE modes immediately animates across the 16 frequency bars and peak caps in real time.

⚡ DYNAMIC ADAPTIVE FRAMERATE & ULTRA-LOW IDLE POWER
• Smart Framerate Adaptation: Runs at a fluid 30 FPS during music playback; automatically relaxes to 2 FPS when audio is silent and meters have settled, slashing open-window idle CPU consumption down to ~0.5%!
• 0.0% CPU in System Tray: Completely stops all DWM rendering passes when minimized to tray, consuming only 8–15 MB RAM in the background.

📻 WIN2POD PLAYER VINTAGE HI-FI AUDIO CHASSIS
• Classic Vintage Hi-Fi Design: Seamlessly integrates live album art, dynamic VFD vacuum fluorescent screen, 16-band segmented spectrum analyzer, and a 10-band graphic equalizer in one unified chassis.
• GSMTC Live Metadata & Timecode: Real-time synchronization with Windows Global System Media Transport Controls, displaying track title, artist, and elapsed / total duration timecode (mm:ss / mm:ss).
• Jitter-Free Interactive Seek Bar: Drag the progress thumb smoothly across tracks; seeking commits precisely upon mouse release without audio stutters or timestamp drift.
• Dynamic Status Badges: VFD screen dynamically indicates real-time AirPlay 2 connection status (emerald green when streaming, dimmed slate gray when idle/disconnected) and audio mode (STEREO green / MONO amber).

📌 WINDOWS "ALWAYS ON TOP" PIN (📌)
• Keep in View Anywhere: Quick-toggle pin button on both the Main Control Panel and Win2Pod Player keeps the interface floating above your workspaces, browsers, and games.
• Robust State Retention: Pinned status is safely preserved across page navigation, minimize-to-tray, and hotkey toggling.

🎯 PER-APP AUDIO ROUTING (APPLICATION AUDIO CAPTURE)
• System Mix or App Isolation: Switch seamlessly between full desktop audio capture or isolated single-application capture (Spotify, Apple Music, web browser).
• Work & Play Undisturbed: Cast your favorite playlist to HomePod while voice calls (Discord, Teams) and gaming sound effects stay right in your PC headphones.

🎵 BUILT SPECIFICALLY FOR APPLE HOMEPOD & AIRPLAY 2
• HomeKit Transient Pairing: Native encrypted handshake compatible with all modern HomePod & HomePod mini firmware.
• Bit-Perfect Lossless ALAC: Encodes real-time audio using Apple Lossless Audio Codec (44.1 kHz, 16-bit stereo) for studio-grade acoustic clarity.
• IEEE 802.1AS / gPTP Clock Sync: Mac-grade PTP clock negotiation eliminates audio jitter, drift, and sync delay.

🎚️ 10-BAND GRAPHIC EQUALIZER & POPULAR PRESETS
• 10-Band Precision EQ: Full frequency band control from 31 Hz to 16 kHz (31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k) with ±12 dB fine adjustments.
• Popular Sound Presets: One-click presets including Bass Boost, Treble Boost, Vocal/Podcast, Rock, Classical, Jazz, Electronic, and Flat Reset.

🔊 HOMEPOD STEREO PAIR & MULTI-ROOM AUDIO STREAMING
• Automatic Stereo Pair Detection: Smartly recognizes and combines paired left and right HomePods into an immersive stereo soundstage.
• Multi-Room Broadcasting: Stream Windows PC sound to multiple AirPlay 2 speakers across different rooms simultaneously.
• Dynamic Grouping: Add or remove speakers on the fly without stopping active playback.
• Master & Independent Volume Sliders: Adjust group master loudness or fine-tune individual speaker volumes.
• Hardware Volume Synchronization: Pressing physical +/- buttons on the HomePod instantly synchronizes with the Windows app volume sliders.

🎨 MODERN WINDOWS 11 FLUENT DESIGN
• Remembers Last Visited Tab: Restores directly to your last view (such as the Player) upon clicking tray icon or keyboard hotkey.
• Aggressive Memory Trimming (Working Set Trim): Automatically flushes unused DLL caches, reducing background footprint to ~8–15 MB RAM.
• Silent Startup: Supports running silently in the system tray on Windows boot (--startup / --minimized).

⚡ SMART POWER & NETWORK MANAGEMENT
• Smart Sleep Prevention: Inhibits Windows sleep during active playback; immediately releases power assertion when muted or paused.
• Virtual Network Card Filtering: Automatically excludes ZeroTier, Tailscale, WireGuard, VMware, and Hyper-V virtual adapters to pin routing directly to physical Wi-Fi/LAN.
• Auto-Mute Local PC Speakers: Silences local PC speakers while streaming to prevent double-audio delay, restoring original volume upon disconnect.
• Customizable Global Hotkey: Toggle streaming anywhere using keyboard hotkeys (default: Ctrl + Alt + H).

🌐 11 LANGUAGES SUPPORTED
Traditional Chinese, Simplified Chinese, English, French, German, Italian, Japanese, Korean, Portuguese (Brazil), Russian, and Spanish. Automatically aligns with your Windows display language.

============================================================
SYSTEM REQUIREMENTS & SETUP NOTES
============================================================
• OS: Windows 10 (Version 1809 or higher) or Windows 11 (64-bit).
• Network: Windows PC and HomePod/AirPlay speakers must be on the same local network subnet (Wi-Fi or Ethernet LAN).
• Windows Firewall: Allow incoming connections on first launch so the speaker can establish RTSP and PTP timing clock links back to the PC.
```

---

## 4. Product Features (Max 20 items, up to 120 characters each)
*(Paste each item into the Features field in Microsoft Partner Center)*

1. `Click album art or track title to bring up source player: Spotify, Apple Music & Tidal`
2. `Dynamic direction stereo balance (◄ Center ►): 32-bit DSP, single click to center`
3. `Win2Pod Motion: 3D Impressionist audio visualizer with full screen, pin on top & sleep guard`
4. `44.1 kHz / 16-bit 2-channel bit-perfect lossless ALAC audio streaming to AirPlay 2`
5. `HomeKit Transient encrypted pairing: 100% pure Rust, no Apple MFi chip required`
6. `Harmonic Exciter (HE): restores natural airy presence & high-frequency overtones`
7. `Warmth Enhancer (WE): analog vinyl physical resonance, saturation drive & soft limiter`
8. `Audiophile DSP Pipeline (HE ➔ EQ ➔ WE) with 10-band graphic EQ & zero-CPU silence bypass`
9. `Post-DSP live spectrum analyzer: displays exact output acoustics delivered to speakers`
10. `Adaptive Framerate: automatically drops to 2 FPS when idle, slashing CPU to ~0.5%`
11. `Win2Pod Player: retro Hi-Fi chassis with album art, GSMTC metadata & smooth seek bar`
12. `Always on Top pin (📌): keep Main Panel and Player floating above all windows`
13. `Per-app audio routing: stream Spotify or browser to HomePod while gaming on headphones`
14. `10-band precision equalizer with 8 curated sound presets (Bass Boost, Vocal, Rock, etc.)`
15. `Automatic HomePod Stereo Pair recognition for an immersive stereo soundstage`
16. `Multi-Room broadcasting: stream PC audio to multiple AirPlay speakers simultaneously`
17. `Dynamic grouping: add or remove speakers on the fly with master & individual volume`
18. `Two-way hardware volume sync: physical HomePod buttons update Windows sliders in real time`
19. `0.0% CPU in system tray: suspends all rendering passes, using only 8–15 MB RAM`
20. `Smart auto-mute PC speakers during stream to prevent echo; restores volume on exit`

---

## 5. Search Terms / Keywords (Max 7 terms, up to 30 characters each)
*(Paste into the Search terms field in Microsoft Partner Center)*

1. `AirPlay`
2. `HomePod`
3. `Audio`
4. `Equalizer`
5. `Streamer`
6. `Music`
7. `Player`
