# Microsoft Store Listing - English (en-US)
# Win2Pod Bridge - Version 0.1.18

## 0. What's New in This Version (Release Notes - Paste into Partner Center)
```text
[Win2Pod Bridge 0.1.18 Major Update - AirPlay Audio Receiver is Here!]
• 📲 All-New "AirPlay Audio Receiver":
  - Transform your Windows PC into a genuine AirPlay wireless speaker! You can now stream music, podcasts, and videos directly from the Control Center on your iPhone, iPad, or Mac ("AirPlay") to your PC speakers or headphones.
  - Native 44.1 kHz / 16-bit Lossless ALAC decoding delivers pristine, audiophile-grade wireless sound.
• ✨ Maximum Dynamic Range (Unity Gain Passthrough):
  - Bypasses iOS Control Center volume attenuation restrictions. The PC receiver decodes audio directly at 1.0 (0 dB Unity Gain) for maximum dynamic range, preserving micro-details and acoustic punch even if the phone slider is lowered.
  - Displays a dedicated "✨ Max Dynamic Range" badge with intuitive hover tooltips showing phone volume.
• ⏯️ Smart Standby Resume (DACP Target Caching):
  - When playback pauses on iPhone and closes the streaming session, the PC standby card retains a quick-resume play button. Tap ▶ to wake iPhone playback over the air, or receive instant connection feedback if disconnected.
• 🎛️ Bi-Directional Media Controls & A/V Lip-Sync Calibration:
  - Reverse-control iPhone playback (Play/Pause, Next/Prev Track) straight from the PC, complete with synchronized real-time album artwork and track metadata.
  - Built-in dual-latency lip-sync presets: Standard (1.5s) and Deep (2.0s) for YouTube and video streaming.
```

## 1. Product Name
```text
Win2Pod Bridge
```

---

## 2. Short Description (Max 270 characters)
*(Displays in search results, category pages, and promotional cards)*

```text
Bridge Windows & Apple audio seamlessly! Stream PC sound to HomePod in lossless ALAC, or receive AirPlay audio from iPhone & iPad with Unity Gain Max Dynamic Range! Features AirPods handoff, 3D oil-painting visualizer, bi-directional media remote & 10-band pro EQ.
```
*(Character count: ~249 chars, comfortably under the 270-character limit)*

---

## 3. Full Description (Store Overview)
*(Copy and paste into the Description field in Microsoft Partner Center)*

```text
Transform your Windows PC into a high-fidelity AirPlay 2 audio transmitter and receiver!

Win2Pod Bridge is a modern, lightweight audio bridge for Windows 10 & 11. It captures desktop audio via low-latency WASAPI loopback and streams it in bit-perfect, lossless ALAC quality directly to Apple HomePod, HomePod mini, Apple TV, and AirPlay 2 speakers. It also functions as an AirPlay Audio Receiver—allowing your PC to act as a wireless speaker for iPhone, iPad, and Mac!

Windows cannot natively send or receive AirPlay audio. Win2Pod Bridge implements both AirPlay 2 sender and receiver protocols in pure Rust. No hardware modifications, no MFi chips, and no complex background services required!

========================================
🌟 KEY FEATURES & CAPABILITIES
========================================

📲 AIRPLAY AUDIO RECEIVER
• Wireless PC Speaker: Stream music and video sound directly from iPhone, iPad, or Mac Control Center ("AirPlay") to your PC speakers or headphones.
• 44.1 kHz / 16-bit Lossless ALAC: Bit-perfect real-time ALAC decoding preserves studio-grade clarity.
• ✨ Maximum Dynamic Range (Unity Gain): Local decoding at 1.0 (0 dB full-scale) bypasses iOS slider compression. Hover tooltip displays phone volume.
• ⏯️ Smart Standby Resume: Tap ▶ on the standby card to wake paused iPhone playback over the air.
• Bi-Directional Remote Controls & Artwork: Control Previous/Next/Play/Pause from PC while synchronizing live album art, track title, and artist.
• Lip-Sync Latency Compensation: Standard (1.5s) and Deep (2.0s) presets align audio and video for YouTube and streaming movies.

• 🛡️ Dynamic Headroom Auto Pre-Amp & 10-Band Pro Equalizer:
  - 10-band graphic equalizer with dynamic headroom management. Boosting frequency bands automatically applies equivalent pre-amp attenuation, paired with a transparent peak limiter to eliminate clipping.
• 🔄 Global Audio Endpoint Fallback & Self-Healing:
  - Endpoint protection: if default speakers disconnect, Win2Pod automatically fails over to active physical devices.
• 🔔 Compact Two-Line System Tray Tooltip:
  - Line 1 shows HomePod streaming state and volume; Line 2 displays AirPods wear status and battery levels (e.g. "🎧 AirPods 3: ● L 100%").
• 🎧 Smart AirPods ↔ HomePod Handoff:
  - Seamlessly transfers audio to AirPods when worn, and hands playback back to HomePod when stored in the case!
• Dynamic Pointer Stereo Balance (◄ Center ►):
  - Intuitive direction cues ("◄ Center ►", "◄ L 30%", "30% R ►"). Single-click label or double-click slider to reset center. Pure 32-bit float software DSP guarantees full dynamic range.
• Quick Launch Source Media Player: Click album art or track title in Win2Pod Player to instantly foreground source apps (Spotify, Apple Music, Tidal, etc.).
• Symmetrical Audio Lifecycle Restitution & Anti-Feedback Shield:
  - Snapshots Windows initial default audio endpoint on startup and faithfully restores playback device, hardware volume, and mute states upon exit.
  - Prevents audio feedback loops in single-device environments via hardware GUID comparison while keeping visualizer meters active.

🎨 WIN2POD MOTION - 3D IMPRESSIONIST AUDIO VISUALIZER
• Vibrant Audio Motion: Real-time reactive 3D particle sphere scattering from deep red bass bursts to violet treble glows.
• Impressionist Painterly Pass: World-anchored 3D brush stroke technology with adjustable stroke size, orbital camera flow, and auto-rotation.
• Fullscreen & Always on Top: Switch to full screen (F key) or pin on top (📌). Prevents system sleep during fullscreen motion.
• Embedded & Offline: Three.js and shaders run 100% locally with zero external network requests.
• Multilingual: Live instant hot-switching across 11 languages.

🎛️ BIDIRECTIONAL SMART MEDIA REMOTE CONTROL
• HomePod Top Touch Surface Gestures: Single tap (Play/Pause), double tap (Next Track), triple tap (Previous Track).
• Apple Home App & Control Center: Remote playback and track navigation from iOS cards.
• Standard AirPlay 2 updateMRSupportedCommands: Encrypted reverse event channel seamlessly controlling Windows GSMTC and media hotkeys.
• Live progress timeline and DMAP metadata synchronized to HomePod.

✨ HARMONIC EXCITER (HE) - HIGH-FREQUENCY RESTORATION
• Restore Lost Air & Sparkle: Compensates for high-frequency rolloff in streaming formats, regenerating harmonics from 8 kHz to 20 kHz.
• Three Mastered Presets:
  - Natural: Gentle enhancement (+1.2dB Pre-EQ / 20% harmonics) for acoustic tracks and podcasts.
  - Dynamic: Balanced excitation (+2.5dB Pre-EQ / 38% harmonics) for pop, rock, and streaming playlists.
  - Brilliant: Maximum clarity (+4.0dB Pre-EQ / 55% harmonics) for electronic tracks and older recordings.

📻 WARMTH ENHANCER (WE) - ANALOG VINYL ACOUSTICS
• Recreate Turntable Warmth: Emulates analog physical acoustics and tape warmth, eliminating harsh digital fatigue.
• Low-Shelf Physical Resonance: +1.5dB to +2.0dB boost at 50Hz–90Hz simulating tonearm resonance for rounded low-end body.
• Analog Saturation Drive: 2.5%–5.0% analog drive simulating tube and tape harmonic warmth.
• Integrated Soft Limiter: Prevents clipping while delivering rich, punchy bass.
• Three Vinyl Modes: Subtle, Warm, and Vintage.

🎛️ PROFESSIONAL AUDIO DSP PIPELINE (HE ➔ EQ ➔ WE)
• Audiophile Processing Chain: Restores overtones first (HE), shapes frequency balance (10-Band EQ), and finishes with analog warmth (WE).
• Silence Fast-Bypass: DSP filters enter 100% sleep mode when audio is silent, waking instantly with 0ms latency when sound resumes.

📊 POST-DSP REAL-TIME SPECTRUM & OUTPUT MONITORING
• 16-Band Output Response: Monitors audio directly after DSP processing, visualizing the exact acoustic profile sent to speakers.
• Immediate Visual Feedback: EQ and HE/WE adjustments animate across 16 frequency bars in real time.

⚡ DYNAMIC ADAPTIVE FRAMERATE & ULTRA-LOW IDLE POWER
• Smart Framerate: 30 FPS during playback; drops to 2 FPS when idle, reducing open-window CPU to ~0.5%!
• 0.0% CPU in Tray: Stops all rendering passes when minimized to tray, consuming only 8–15 MB RAM.

📻 WIN2POD PLAYER VINTAGE HI-FI AUDIO CHASSIS
• Classic Vintage Hi-Fi Design: Integrates live album art, dynamic VFD vacuum fluorescent screen, 16-band spectrum analyzer, and 10-band graphic EQ in one unified chassis.
• GSMTC Live Metadata & Timecode: Real-time track title, artist, and elapsed / total duration timecode.
• Smooth Interactive Seek Bar: Drag and seek precisely upon mouse release without stutter.
• Status Badges: VFD screen indicates real-time AirPlay 2 connection status and STEREO / MONO mode.

📌 WINDOWS "ALWAYS ON TOP" PIN (📌)
• Keep in View Anywhere: Quick-toggle pin button on Main Panel and Player keeps interface floating above workspaces and games.
• Robust State Retention: Preserved across navigation, minimize-to-tray, and hotkey toggles.

🎯 PER-APP AUDIO ROUTING (APPLICATION AUDIO CAPTURE)
• System Mix or App Isolation: Switch between full desktop audio capture or isolated single-app capture (Spotify, Apple Music, browser).
• Work & Play Undisturbed: Cast your playlist to HomePod while voice calls (Discord, Teams) and gaming stay in your PC headphones.

🎵 BUILT SPECIFICALLY FOR APPLE HOMEPOD & AIRPLAY 2
• HomeKit Transient Pairing: Native encrypted handshake compatible with all HomePod & HomePod mini firmware.
• Bit-Perfect Lossless ALAC: Encodes real-time audio using Apple Lossless Codec (44.1 kHz, 16-bit stereo).
• IEEE 802.1AS / gPTP Clock Sync: Mac-grade PTP clock negotiation eliminates jitter, drift, and sync delay.

🎚️ 10-BAND GRAPHIC EQUALIZER & POPULAR PRESETS
• 10-Band Precision EQ: Full frequency band control from 31 Hz to 16 kHz with ±12 dB adjustments.
• Popular Sound Presets: One-click presets including Bass Boost, Treble Boost, Vocal/Podcast, Rock, Classical, Jazz, Electronic, and Flat.

🔊 HOMEPOD STEREO PAIR & MULTI-ROOM AUDIO STREAMING
• Automatic Stereo Pair Detection: Smartly recognizes and combines paired left and right HomePods into an immersive stereo soundstage.
• Multi-Room Broadcasting: Stream Windows PC sound to multiple AirPlay 2 speakers simultaneously.
• Dynamic Grouping: Add or remove speakers on the fly without stopping playback.
• Master & Independent Volume: Adjust group master loudness or fine-tune individual speaker volumes.
• Hardware Volume Synchronization: Physical +/- buttons on HomePod synchronize with Windows sliders in real time.

🎨 MODERN WINDOWS 11 FLUENT DESIGN
• Remembers Last Visited Tab: Restores directly to your last view upon reopening.
• Aggressive Memory Trimming: Flushes unused caches, reducing background footprint to ~8–15 MB RAM.
• Silent Startup: Supports running silently in system tray on boot (--startup / --minimized).

⚡ SMART POWER & NETWORK MANAGEMENT
• Smart Sleep Prevention: Inhibits Windows sleep during playback; releases power lock when muted or paused.
• Virtual Adapter Filtering: Excludes ZeroTier, Tailscale, WireGuard, and VM adapters to pin routing directly to physical Wi-Fi/LAN.
• Auto-Mute Local Speakers: Silences PC speakers while streaming to prevent double-audio delay; restores volume on disconnect.
• Customizable Global Hotkey: Toggle streaming anywhere using keyboard hotkeys (default: Ctrl + Alt + H).

🌐 11 LANGUAGES SUPPORTED
English, Traditional Chinese, Simplified Chinese, French, German, Italian, Japanese, Korean, Portuguese (Brazil), Russian, and Spanish.

========================================
SYSTEM REQUIREMENTS & SETUP NOTES
========================================
• OS: Windows 10 (Version 1809 or higher) or Windows 11 (64-bit).
• Network: Windows PC and HomePod/AirPlay speakers must be on the same local subnet (Wi-Fi or Ethernet LAN).
• Windows Firewall: Allow incoming connections on first launch so speakers can establish RTSP and PTP timing clock links back to the PC.
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
