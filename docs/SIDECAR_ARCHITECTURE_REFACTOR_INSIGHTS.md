# Win2Pod Bridge Sidecar 架構重構與跨程序音訊串流實踐心得
# Win2Pod Sidecar Process Architecture Refactor & IPC Streaming Insights

**文件版本**：1.0.0  
**對應專案版本**：`0.2.0` (`feature/sidecar-architecture`)  
**撰寫時間**：2026-11-26  
**作者**：mesmerli & Antigravity  

---

## 1. 重構背景與核心挑戰 (Context & Motivation)

### 1.1 GPL 開源條款合規與商業保護矛盾
Win2Pod Bridge 底層參考與引用了開源社群優秀的 AirPlay 2 與 Apple Continuity 專案（如 `airplay2-rs`, `UxPlay`, `windows-airplay-homepod`, `AirPodsDesktop`）。上述專案皆採用 **GNU 通用公眾授權條款（GPL-2.0 / GPL-3.0）**。
- **原始單體架構（Monolithic）之痛**：所有開源 Crate 與專屬開發的 Fluent GUI（egui/DirectX 介面、自訂游標、磁吸叢集視窗、微軟市集 In-App Purchase 授權驗證等）皆靜態編譯於單一執行檔 `win2pod.exe` 中。
- **法律風險**：GPL 具備強傳染性（Copyleft），任何靜態連結 GPL 代碼的軟體，其整體均被視為衍生作品，面臨整體代碼必須開源的法律挑戰，限制了商業化與在微軟市集保護專屬核心智慧財產的可能性。

### 1.2 目標：Sidecar 隔離架構 (Out-of-Process Isolation)
依據自由軟體基金會（FSF）官方 GPL FAQ 的明確指引：若兩個程式執行於**獨立的作業系統行程（Separate Processes）**，僅透過標準 IPC（如具名管道、Sockets）傳遞一般性指令或結構化資料，則彼此在法律上構成獨立程式（Independent Programs），可分別適用不同授權。
- **前端 GUI (`win2pod.exe`)**：專屬商業保護授權（Proprietary EULA），保障原創 UI 與產品體驗。
- **IPC 協議橋樑 (`win2pod-ipc`)**：寬鬆無傳染授權（`MIT OR Apache-2.0`），作為數據傳輸契約。
- **後端引擎 (`win2pod-engine.exe`)**：無頭守護行程，採用 `GPL-3.0-or-later`，恪守開源回饋規範。

---

## 2. 關鍵排錯過程與技術根因分析 (Troubleshooting & Root Causes)

在將單體音訊串流與控制器切換至 Sidecar 驅動時，我們經歷了連線失敗、連線逾時、有頻譜無聲音等一系列複雜的跨行程與底層 WASAPI / RTSP 故障，以下記錄完整的根因與解法：

### 2.1 連線失敗：跨行程裝置資料還原 (Device Reconstruction)
- **問題現象**：GUI 發送 `IpcRequest::Connect` 後，後端引擎回應連線失敗。
- **根本原因**：早期引擎在收到目標裝置名稱後，試圖在後端重新發起 mDNS 廣播探索（耗時約 2 秒）。若區域網路內 mDNS 回應延遲，即引發逾時。
- **修復方案**：在 `ConnectTargetIpc` 與 `DeviceIpc` DTO 中完整封裝裝置的 IP 列表、連接埠、MAC ID、功能特徵碼（Features）與立體聲資訊。後端直接透過 `to_core_device()` 反序列化為 `airplay_core::Device`，達成**零延遲直接建連**。

### 2.2 連線逾時：Windows Defender 防火牆的「靜默封鎖」機制
- **問題現象**：`Connect` 階段卡在 RTSP/NTP 握手，最終報錯「連線逾時（目標裝置沒有回應）」。
- **根本原因**：
  1. Windows Defender 防火牆的安全警訊（允許存取網絡）**只會彈出給擁有前景互動桌面視窗的程式**。
  2. 後端 `win2pod-engine.exe` 是由 GUI 呼叫 `Command::spawn()` 喚起的無頭子行程（Child Process），被 Windows 防火牆判定為非互動式背景程式。為了防止後台惡意誘導，Windows 對其採取**「靜默攔截（Silent Block）」**，既阻擋 UDP/TCP 入站，也絕不向使用者彈出提示對話框。
- **修復方案（環境感知智慧防火牆輔助）**：
  實作 `firewall.rs` 模組：
  - **MSIX 容器環境**：透過 `is_packaged_app()` 偵測，若處於 MSIX 套件內，完全靜默跳過，由 Windows AppContainer 原生聲明的 `privateNetworkClientServer` 權限自動通關。
  - **綠色便攜版（Portable）**：自動查詢系統目前是否已有指向當前 `win2pod-engine.exe` 路徑的入站規則。若無，呼叫 Windows 原生 `ShellExecuteW(..., "runas", ...)` 喚起系統標準 UAC 提權授權。使用者只需點擊一次「是」，規則即在背景靜默生效，**徹底免去使用者開啟終端機手動敲打 `netsh` 的困擾**。

### 2.3 HomePod 連線成功但沒有聲音：解碼器提前 EOF 與隱蔽的邏輯判斷 Bug
這是本次重構中最關鍵、最深奧的故障排查：
- **問題現象**：HomePod 成功連線，GUI 介面顯示已連線，但 HomePod 完全靜音，且約 30 秒後被 HomePod 主動斷線。
- **深層日誌追蹤 (`engine.log`)**：
  ```text
  INFO win2pod_engine::airplay_sender: Sidecar WASAPI capture thread exiting cleanly
  DEBUG airplay_audio::live_decoder: Live decoder: channel disconnected, marking EOF
  INFO airplay_audio::streamer: Decoder EOF and buffer empty - stopping
  INFO airplay_audio::streamer: Sender thread: received Stop, exiting
  ```
- **三重根本原因發現**：
  1. **第一重：WASAPI 讀取拋錯直接中斷線程**：早期擷取代碼使用 `read_from_device_to_deque(&mut queue)?`，在啟動首輪無音訊時拋錯退出，導致 `LiveFrameSender` 被 drop，解碼器關閉。
  2. **第二重（致命邏輯 Bug）**：在重構擷取迴圈時，外層傳入 `running: Arc<AtomicBool>`（初始值為 `true`，代表運作中）。但在 `capture_loop` 內部，迴圈條件誤寫成了：
     ```rust
     while !stop.load(Ordering::Relaxed) // 此時 running 為 true，!true 立即為 false！
     ```
     這導致錄音線程在被 spawn 的第 0.0001 毫秒，就因為 `while false` 直接跳出迴圈退出！解碼器瞬間標記 EOF，RTP 音訊串流線程即刻終止。
  3. **第三重（30 秒超時斷開）**：AirPlay 2 規範要求發送端每隔約 2 秒向 HomePod 發送一次 RTSP `FEEDBACK` 請求。單體版本是在 GUI 主迴圈內發送，而 Sidecar 初期欠缺此背景維持任務，導致 HomePod 在 30 秒無心跳後判定通訊中斷（`os error 10054`）。
- **修復方案**：
  - 將錄音條件正名並更正為 `while running.load(Ordering::Relaxed)`。
  - 將 WASAPI 迴圈設計為具備重試容錯能力的架構，在緩衝區無音訊時持續發送 50ms 靜音保活影格（Silence Keepalive），確保解碼器與發送器緩衝永不 Underrun。
  - 在 `ActiveStreamSession` 內建立專屬 Tokio 定時任務，每 2 秒發送 RTSP Feedback，日誌顯示 `cseq=107..115` 連續 `200 OK`，徹底杜絕斷線。

### 2.4 頻譜儀 (Visualizer) 沒反應：跨行程架構下的 Tap 脫鉤
- **問題現象**：電腦播放音樂，GUI 介面頻譜條與峰值計維持在 0。
- **根本原因**：單體架構中，DSP 分析器是在 GUI 行程內直接透過指針掛載在音訊串流線上（`set_post_dsp_hook`）。改為 Sidecar 後，音訊流轉移至 `win2pod-engine.exe`，GUI 端不再收到音訊 Frame。
- **修復方案**：
  在 `controller.rs` 的 `SidecarStreaming` 模式中，實作並啟動專屬的輕量級 WASAPI 本地監聽 Tap（`spawn_visualizer_tap_loop`）。
  - Windows WASAPI 允許多個客戶端同時對預設音訊端點進行 Loopback 擷取。
  - 同時加入 **Zero-CPU 智慧休眠機制**：當 Win2Pod 介面縮小至系統匣或無任何視覺化視窗顯示時，Tap 自動暫停 FFT 計算，節省系統運算資源。

---

## 3. 授權架構與版本演進決策 (Licensing & Versioning)

### 3.1 版本號正式跨入 `0.2.0`
依據語意化版本（Semantic Versioning）規則：
- `0.1.x`：代表第 1 階段的「單體 Monolithic」歷史架構。
- `0.2.0`：標誌著專案正式跨入「Sidecar 程序隔離、雙行程 IPC、商業授權分離」的全新世代。
- 全工作區 17 個 Crates 與 MSIX 清單同步升級為 `0.2.0` / `0.2.0.0`。

### 3.2 方案 A（商業專屬保護型分離式授權架構）
透過行程物理邊界，確立合法合規的複合授權體系：
1. **前端 GUI (`crates/homepod-cast-gui`)**：
   - 宣告：`Proprietary`（專屬商業授權），版權所有 © 2024-2026 mesmerli。
   - 保障微軟商店付費機制、自訂游標、流暢磁吸介面與視覺化專利體驗。
2. **IPC 協定橋樑 (`crates/win2pod-ipc`)**：
   - 宣告：`MIT OR Apache-2.0`。
   - 純資料結構體（DTO），不含業務邏輯，具備最高的相容與寬鬆性。
3. **後端引擎 (`crates/win2pod-engine`, `crates/airplay-*`)**：
   - 宣告：`GNU GPL-3.0-or-later`。
   - 保留上游開源貢獻者鳴謝（Lucas McGartland, Yang Liu, antimof, SpriteOvO 等），源碼隨本專案公開。

---

## 4. Git 雙軌維護策略提案 (Branch Maintenance Strategy)

為了確保舊版使用者的支援，同時讓新架構全力迭代，確立以下 Git 分支方針：

- **`master` 分支（主線 0.2.x+）**：
  - 承接 `feature/sidecar-architecture` 成果。
  - 所有未來新功能、新 DSP 音效、新版 UI 皆併入此處。
- **`release/0.1.x` 分支（舊單體 LTS 維護線）**：
  - 從 `v0.1.24` 切出，進行**功能凍結（Feature Freeze）**。
  - 僅在遭遇重大相容性 Bug 時出 Hotfix 修正（`0.1.25`, `0.1.26`），修復內容可透過 `git cherry-pick` 挑選同步至主線。

---

## 5. 結論與成果 (Conclusion)

本次重構成功達成：
1. **商業/開源法律風險完全化解**：乾淨切斷 GPL 傳染性，符合全球軟體分發規範。
2. **音質零損耗與穩定度躍升**：保留 44.1kHz ALAC、250ms/500ms 緩衝管理與 NTP/PTP 自動切換，消除了單體架構下 GUI 卡頓導致音訊抖動的隱患。
3. **完全無感的極致使用者體驗**：後端強制啟用 `CREATE_NO_WINDOW` 絕對無視窗背景運行，搭配智慧 UAC 防火牆自動配置，使用者開箱即用、一鍵串流！
