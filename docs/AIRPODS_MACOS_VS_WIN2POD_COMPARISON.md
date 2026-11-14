# macOS 原生 AirPods 音路自動切換與佩戴偵測機制 vs. Win2Pod 實作比較

本文件詳細比較 Apple macOS 原生環境下的 AirPods 音路自動切換、佩戴偵測與播放控制邏輯，以及 Win2Pod Bridge 在 Windows 系統上的現行實作規則、底層架構差異、防護機制與潛在優化空間。

---

## 1. 綜合架構與運作原理對照 (Architecture Overview)

| 比較維度 | macOS 原生環境 (Native Apple Ecosystem) | Win2Pod Bridge (Windows 10/11) |
| :--- | :--- | :--- |
| **通訊與狀態同步** | **Apple ID / iCloud 雲端接力 (APNs)**<br>Mac 與 iPhone/iPad 透過 iCloud Keychain 與藍牙 APNs 互傳活躍度與通話狀態。 | **本機被動監聽 (WinRT BLE Watcher)**<br>透過逆向 Apple Continuity 廣播協定 (`0x004C`, Type `0x07`) 即時解析封包。 |
| **音訊核心架構** | **CoreAudio & OS 內核藍牙驅動**<br>作業系統層直接控制藍牙 ACL 鏈路、音訊端點切換與麥克風輸入轉移。 | **WASAPI Loopback + AO 替身模式**<br>採用 Windows WASAPI 回錄系統聲音，經 DSP 渲染後透過獨立音訊串流輸出至 AirPods。 |
| **媒體控制機制** | **系統級媒體框架 (Now Playing)**<br>直接控制正在播放音訊的應用程式暫停與繼續。 | **Windows SMTC (System Media Transport Controls)**<br>透過發送虛擬媒體控制指令 (`MediaControlCmd::Pause` / `Play`) 控制播放器。 |
| **音路目標維度** | **AirPods $\longleftrightarrow$ Mac 內建揚聲器 / 螢幕喇叭** | **AirPods (AO 零延遲) $\longleftrightarrow$ Apple HomePod (AirPlay 2) / 本機喇叭** |

---

## 2. 核心規則逐項深入比較 (Detailed Feature Comparison)

### 2.1 跨裝置接力與優先權規則 (Cross-Device Handoff & Priority)

#### macOS 原生規則
1. **主動播放觸發轉移**：若佩戴連接 iPhone 的 AirPods，在 Mac 上直接點擊影片或音樂播放鍵，音路會自動無縫轉移到 Mac。
2. **通話最高優先**：Mac 播放時若 iPhone 有來電，強制優先切回 iPhone 供接聽；反之若 Mac 正在進行視訊會議（FaceTime、Zoom、Teams），iPhone 的媒體播放不會搶走耳機音路。
3. **系統提示音不搶線**：Mac 的系統通知聲或背景無聲廣告不會觸發搶線。

#### Win2Pod 現行實作與差距
* **根本差距（生態界限）**：Windows 是獨立封閉的第三方系統，**無法加入 Apple 帳號的 iCloud 跨裝置通道**，因此無法感知使用者的 iPhone 當前是否有電話打入或正在播放音樂。
* **防暴力奪取機制（Anti-Hijack Protection）**：
  * 當使用者戴上耳機（BLE 偵測到佩戴狀態），但 Windows 音訊系統找不到可用的 AirPods 端點時（代表耳機正連接著 iPhone 通話或聽音樂），Win2Pod **不會暴力嘗試搶奪藍牙連線**。
  * Win2Pod 會保留原本的音路（持續在 HomePod 播放或留在原喇叭），並在介面上跳出通知提示橫幅（行為類似 macOS Sonoma 的防搶保護），保護使用者在手機上的通話與聆聽體驗。

---

### 2.2 佩戴偵測與媒體播放控制 (In-Ear Detection & Playback Controls)

#### macOS 原生規則
1. **只拿下一邊耳機（單耳取下）**：
   * 音訊自動暫停。
   * 將耳機戴回耳中時，**自動恢復播放**。
2. **同時拿下兩邊耳機（雙耳取下）**：
   * 音訊完全停止。
   * 即使稍後將兩邊耳機重新戴上，**系統也不會自動續播**，使用者必須手動在播放器上按下播放鍵。

#### Win2Pod 現行實作與差距
* **狀態即時監控**：Win2Pod 具備完整的光學入耳感測器解析能力（精確記錄 `left_in_ear`、`right_in_ear` 與佩戴數量 `worn_count`）。
* **單耳取下**：
  * 偵測到 `worn_count` 減少時（如 $2 \rightarrow 1$ 或 $1 \rightarrow 0$），發送 `MediaControlCmd::Pause` 暫停 Windows 媒體播放。
  * 啟動符合 Apple 規範的 **60 秒自動續播視窗 (Auto-Resume Window)**，60 秒內戴回會發送 `MediaControlCmd::Play` 自動續播。
* **雙耳取下（微小差距）**：
  * **現況**：目前程式碼中，即使兩耳皆拿下（$2 \rightarrow 0$），只要在 60 秒超時之前將耳機戴回，仍會觸發自動恢復播放。
  * **對齊建議**：若需 100% 嚴格對齊 macOS 原廠行為，僅需在 `current_worn_count == 0` 時將 `airpods_paused_at` 重設為 `None`，即可達成「雙耳脫離後戴回不自動續播」的邏輯。

---

### 2.3 音路轉移時機與防漏音隱私 (Audio Leakage & Fallback Behavior)

#### macOS 原生規則
1. **音訊路徑轉回揚聲器**：將雙耳取下置於桌上（非佩戴狀態），macOS 判定為閒置後會將預設輸出切回 Mac 內建揚聲器。
2. **放回充電盒蓋上盒蓋**：耳機與 Mac 藍牙立即完全中斷，所有音訊直接轉由 Mac 處理。

#### Win2Pod 現行實作與差距（隱私防護設計）
* **耳機取下置於桌上（防漏音保護）**：
  * 在 Windows 環境下，若耳機放在桌上就自動將音訊切回電腦外放喇叭，極易在辦公室、圖書館等安靜環境造成聲音外洩的尷尬。
  * 因此 Win2Pod 採取 **Apple 隱私防護原則（Apple Privacy Design）**：耳機脫離耳朵但未入盒關蓋前，**音路維持在 AirPods 端點（靜音/暫停狀態），絕不切回外放喇叭**。
* **放回充電盒並關閉盒蓋 (`is_stored_in_case_and_closed`)**：
  * Win2Pod 精準監聽盒蓋開關狀態 (`is_lid_opened`) 與雙耳入盒狀態 (`both_in_case`)。
  * 只有在觸發「入盒且關蓋」的邊緣事件時（Edge Trigger），Win2Pod 才會自動切回揚聲器或還原至 HomePod。

---

### 2.4 音路目標設備（Win2Pod 的專屬優勢：HomePod 串流快照）

#### macOS 原生規則
* 音路主要在 **AirPods $\longleftrightarrow$ Mac 內建喇叭/螢幕喇叭** 之間切換。

#### Win2Pod 現行實作
* Win2Pod 不僅是耳機控制器，更是 Windows 與 Apple HomePod 的 AirPlay 橋接核心。
* **路徑中斷快照機制 (`InterruptedRoute`)**：
  1. 當使用者正在透過 Win2Pod 將電腦聲音 AirPlay 串流至 HomePod 時：
  2. 一旦戴上 AirPods，Win2Pod 立即將中斷前的狀態記錄為 `InterruptedRoute::HomePod`，並切換至本地零延遲 AO 模式。
  3. 當使用者聽完將 AirPods 放回充電盒並蓋上盒蓋時，Win2Pod **會自動且無縫地將音路恢復串流至 HomePod**；若戴上前是本機實體喇叭，則恢復至本機喇叭。

---

### 2.5 麥克風與會議優先權 (Microphone & Conference Priority)

#### macOS 原生規則
* 取下耳機後，視訊會議軟體（Zoom、Teams、FaceTime）的麥克風輸入自動無縫切換回 Mac 內建麥克風，避免通話中斷。
* 通話中具備全域最高優先權，防止其他設備搶走音路。

#### Win2Pod 現行實作
* **不在範疇**：Win2Pod 定位為立體聲高傳真音訊輸出、DSP 音質渲染（10-Band EQ、Sound Enhancer、Warmth Enhancer）與 HomePod 串流工具。
* 軟體不管理 Windows 系統的麥克風錄音端點與會議軟體的輸入切換，麥克風由 Windows 系統原生機制自行處理。

---

### 2.6 耳機名稱自訂與音訊端點比對機制 (Device Naming vs. BLE Advertisement)

#### BLE 狀態偵測：完全不依賴耳機名稱
* Win2Pod 在背景透過 WinRT 監聽低功耗藍牙廣播（Apple Continuity Protocol）。
* 封包解析直接讀取 Apple 原廠硬體資訊：
  * **Company ID**：`0x004C` (Apple Inc.)
  * **Packet Type**：`0x07` (Proximity Pairing)
  * **Model ID**：硬體晶片代碼（如 `0x200E` 為 AirPods Pro，`0x2014` 為 AirPods Pro 2）
* **結論**：無論使用者在 iPhone 或 Windows 上如何自訂耳機名稱（即使改成無關字串），**電量顯示、充電中狀態 ⚡、入耳/離耳偵測與充電盒開關蓋皆能 100% 精準識別**。

#### Windows 音路切換：需保留關鍵字
* 當使用者戴上耳機時，Win2Pod 必須在 Windows 的所有 WASAPI 播放裝置清單（Audio Render Endpoints）中，找出對應的耳機端點以完成音路切換。
* Windows 系統 API 僅提供裝置 FriendlyName 字串，Win2Pod 透過關鍵字比對（`name.contains("airpods") || name.contains("beats") || name.contains("pods")`）。
* **使用須知**：
  * 預設情況下，Apple 裝置配對名稱皆帶有「AirPods」或「Pods」，可自動無縫匹配。
  * **若使用者曾徹底自訂耳機名稱（如改為純英文字「My Headphone」或自訂中文「音樂天使」），請務必在名稱中保留「AirPods」或「Pods」關鍵字**，否則系統將無法在 Windows 聲音清單中辨識出該端點為 AirPods，導致無法自動轉移音路。

---

## 3. 規則差距對照矩陣 (Comparison Matrix)

| 功能項目 | macOS 原生行為 | Win2Pod 現行行為 | 差距性質 | 建議處置 |
| :--- | :--- | :--- | :--- | :--- |
| **iCloud 跨裝置通話搶線** | 來電自動強制轉移至 iPhone | 無法得知 iPhone 來電狀態 | 平台硬限制 | 維持現狀，保留防搶線提示 |
| **單耳拿下暫停** | 音樂暫停 | 發送 SMTC Pause | 完全一致 | 維持現狀 |
| **單耳戴回自動續播** | 戴回自動續播 | 60 秒內戴回自動續播 (Play) | 完全一致 | 維持現狀 |
| **雙耳同時拿下** | 播放停止，戴回**不自動續播** | 目前 60 秒內戴回仍會嘗試自動續播 | 微小差距 | 可在兩耳全離耳時清除續播記憶 |
| **拿下放桌上** | 判定閒置後切回內建喇叭 | 刻意維持耳機端點，防止喇叭漏音 | 刻意設計 | 建議維持 Win2Pod 防漏音防護 |
| **入盒關蓋** | 藍牙中斷，音路切回喇叭 | 偵測入盒關蓋邊緣事件還原音路 | 行為等效 | 維持現狀 |
| **音路復原目標** | 復原至 Mac 內建喇叭 | 復原至 HomePod 串流或本機喇叭 | Win2Pod 更強 | 維持現狀 (`InterruptedRoute`) |
| **通話麥克風無縫切換** | 麥克風切回 Mac 內建輸入 | 不託管 Windows 錄音設備 | 系統分工 | 維持現狀，由 Windows 處理 |

---

## 4. 後續可對齊之調整建議 (Actionable Alignment)

若後續評估希望在行為上更緊密貼合 macOS 原生習慣，可進行以下程式碼微調：

1. **對齊「雙耳脫離不自動續播」**：
   在 `crates/homepod-cast-gui/src/controller.rs` 處理離耳事件時，當 `current_worn_count == 0`（雙耳皆脫離）：
   ```rust
   if current_worn_count == 0 {
       // macOS 規則：雙耳皆取下視為完全停止，戴回時不自動續播
       airpods_paused_at = None;
   }
   ```
   如此一來，使用者拿下一邊時仍可享有戴回自動續播的便利；但當雙耳都拿下時，戴回就不會自動重啟音樂，需手動點擊播放。
