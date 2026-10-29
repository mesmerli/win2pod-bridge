# Win2Pod Bridge

[English](README.md) • 繁體中文

**Win2Pod Bridge** 是一款專為 Windows 10 / 11 打造的高效能音訊串流工具。透過 Windows 原生 WASAPI Loopback 擷取技術與 AirPlay 2 協定實作，讓您的 PC 能夠無縫、低延遲地將電腦聲音串流至 **Apple HomePod、HomePod mini、Apple TV** 以及任何支援 AirPlay 2 的音響設備。

Windows 系統原生無法直接發送 AirPlay 訊號，且 Apple HomePod 採用專屬的加密握手流程（HomeKit Transient Pairing），導致多數第三方工具無法與其配對。Win2Pod Bridge 完全採用純 Rust 實現該握手協定，無需額外的 Apple MFi 硬體授權晶片，即可在 Windows 上享有如同 Mac 般的流暢投播體驗。

---

## ✨ 核心特色

### 1. 🎵 專為 Apple HomePod 與 AirPlay 2 深度打造
- **HomeKit 暫態配對 (Transient Pairing)**：完整實作 SRP-6a、Curve25519 與 ChaCha20-Poly1305 軟體級加密握手，相容最新韌體的 HomePod 與 HomePod mini。
- **無損 ALAC 即時壓縮編碼**：以 44.1 kHz / 16-bit 雙聲道高保真 ALAC（Apple Lossless Audio Codec）即時編碼傳輸，原音重現。
- **PTP 高精度時鐘同步 (IEEE 802.1AS / gPTP)**：實作 Mac 等級的 BMCA 時鐘協商與同步機制，徹底消除聲音抖動（Jitter）與飄移延遲。

### 2. 🔊 智慧立體聲配對與多房間群組 (Multi-Room)
- **自動辨識 HomePod 立體聲組 (Stereo Pair)**：自動將配對的左右聲道 HomePod 辨識為單一播放端點，享受身歷其境的立體環繞音場。
- **多裝置多房間廣播**：支援同時選取並串流多台 AirPlay 喇叭，讓全家不同房間同步聆聽。
- **動態加退群組**：串流進行中可隨時「＋ 加入播放」新喇叭或退出單一喇叭，無須中斷現有串流。
- **獨立與總體音量調節**：群組串流時提供總音量滑桿，並可針對每一台音響獨立微調個別音量。

### 3. 🎨 現代化 Fluent 浮動面板與狀態感知系統匣 (System Tray)
- **Fluent 懸浮面板**：基於 Rust `egui`/`eframe` 打造，完美融合 Windows 11 風格，點擊系統匣圖示即可優雅開啟。
- **狀態感知圖示**：系統匣圖示即時呈現連線狀態（待命灰環、連線中橘環、串流中綠環、異常紅環），並在連線被搶佔或異常時發出系統通知。
- **極致記憶體優化**：縮小至系統匣時自動休眠 UI 繪製並釋放 GPU/OpenGL 快取，背景記憶體佔用極低（約 10～20 MB）。
- **開機自動於背景啟動**：支援開機自啟動與靜默啟動參數（`--startup` / `--minimized`），開機時安靜駐留在系統匣。

### 4. 🎛️ 音訊分流模式 (Split Mode)
- **全系統混音擷取 (WASAPI Loopback)**：一鍵捕捉 Windows 電腦的所有發聲應用程式。
- **音訊來源端點自選**：可指定特定播放裝置進行側錄。例如：將 Spotify / Apple Music 獨立串流至 HomePod，而遊戲語音、Discord 或會議則保留在 PC 耳機播放。
- **一鍵開啟音量混音器**：內建捷徑按鈕直接開啟 Windows 原生應用程式音量混音設定。

### 5. ⚡ 智慧電源與輔助功能
- **串流時電腦喇叭自動靜音 (Auto-Mute)**：投播時自動將本機喇叭靜音，防止雙重發聲回音；中斷連線或結束程式時自動精準還原原始音量。
- **智慧防休眠機制**：僅在「有實際音訊播放」時自動阻止 Windows 進入休眠，靜音或暫停時自動釋放電源鎖。
- **低延遲模式開關**：可自由切換低延遲緩衝設定，適合影片播放與即時操作。
- **記住前次連線端點 (Auto-Connect)**：啟動時自動嘗試連線前次使用的音響設備。
- **全域快捷鍵**：支援自訂鍵盤快速鍵（預設 `Ctrl + Alt + H`，亦可選 `Alt + H`、`Win + Alt + H`、`F9`、`F10` 等），隨時一鍵切換串流開關。

### 6. 🌐 完整多國語系支援
- 內建 11 種語言界面：繁體中文、簡體中文、English、Français、Deutsch、Italiano、日本語、한국어、Português (Brasil)、Русский、Español。
- 可於「進階設定 ⚙」中透過下拉選單自由切換，預設自動依 Windows 系統語言自動適配。

---

## 📋 系統需求

1. **作業系統**：Windows 10 (1809 以上) 或 Windows 11 (64-bit)。
2. **網路環境**：電腦與 HomePod / AirPlay 喇叭必須連接至**同一個區域網路（Wi-Fi 或 LAN）**。
3. **Windows 防火牆設定**：  
   AirPlay 2 協定需要音響回連電腦的 RTSP / 時鐘校時連接埠。初次使用時請在系統防火牆放行入站連線，或以系統管理員身分在 PowerShell 執行：

```powershell
netsh advfirewall firewall add rule name="Win2Pod Bridge" dir=in action=allow program="C:\path\to\win2pod.exe" enable=yes profile=any
```

---

## 🚀 使用方式

1. 啟動 `win2pod.exe`（或從開始功能表啟動 MSIX 安裝版本）。
2. 點擊系統匣圖示開啟主控制面板，程式將自動透過 mDNS 掃描區域網路內的 AirPlay 設備。
3. 點選目標喇叭旁邊的「**連線**」，Windows 音訊便會立即開始串流至該音響！
4. 點擊右上角「**⚙**」進入進階設定，可調整音訊來源、自動靜音、低延遲模式、全域快捷鍵與介面語言。

---

## 🛠️ 從源碼編譯 (Build from Source)

本專案採用 Rust GNU 工具鏈與 MinGW-w64 編譯：

```powershell
# 1. 切換至 Rust GNU 工具鏈
rustup default stable-x86_64-pc-windows-gnu

# 2. 編譯可攜式獨立 Release 執行檔（輸出至 dist\win2pod.exe）
.\build.ps1

# 3. 或編譯並自簽 MSIX 安裝包（需安裝 Windows SDK makeappx / signtool）
.\build-msix.ps1
```

---

## 📜 授權與鳴謝 (Acknowledgements & License)

本專案基於 Lucas McGartland 的 [**airplay2-rs**](https://github.com/lmcgartland/airplay2-rs) 核心研究進行深度重構與擴充，新增了：
- 純 Rust Fluent 浮動 GUI 與系統匣管理（`crates/homepod-cast-gui`）
- Windows 原生 WASAPI Loopback 擷取管線與即時 ALAC 編碼器
- IEEE 802.1AS / gPTP 時鐘同步與多房間群組串流架構
- MSIX 打包支援、微軟商店（Microsoft Store）授權模組與 11 國語系

**開源授權**：依循上游專案採用 GPL-2.0 授權條款，詳見 [`LICENSE`](LICENSE)。
