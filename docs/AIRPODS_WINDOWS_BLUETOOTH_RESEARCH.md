# Windows 環境下 AirPods 藍牙連線、斷線與音路自動切換技術研究與總結

## 1. 緣起與目標 (Background & Goals)

在 Win2Pod Bridge 開發過程中，我們嘗試在主控制面板加入「可用的 AirPods」列表，並提供「⚡ 連線」與「中斷連線」按鈕，讓使用者無需開啟 Windows 10/11 的「設定 > 藍牙與裝置」或快捷面板，即可在應用程式內部一鍵喚醒並連線 AirPods。

然而，在實測中發現**連線行為極度不穩定（時好時壞、連線卡住、甚至造成 Windows 音訊端點失效）**。本文件記錄探索過程中所釐清的 Windows 藍牙架構底層機制、API 行為陷阱與連線失敗的根本原因，作為未來維護與藍牙功能設計的技術依據。

---

## 2. Windows 藍牙雙模架構與 AirPods 通訊原理

AirPods / Beats 等現代 Apple 音訊裝置在 Windows 上運作時，分為兩個截然不同的層次：

### 2.1 低功耗藍牙 (BLE - Bluetooth Low Energy)
* **協定**：Apple Continuity Protocol (AppleCP)，製造商自定義廣播資料 (`Manufacturer Data`, Company ID `0x004C`, Type `0x07`)。
* **特性**：
  * 即使 AirPods **尚未**與 Windows 建立音訊連線（只要在充電盒附近打開盒蓋、或戴在耳朵上），它就會持續以 BLE 發射廣播。
  * Win2Pod 利用 WinRT 的 `BluetoothLEAdvertisementWatcher` 可以在無連線狀態下被動監聽，即時獲取左右耳電量、充電狀態、充電盒電量、入耳/離耳狀態與盒蓋狀態。
* **穩定性**：**極高**。純被動接收廣播，無連線握手負擔，不佔用音訊通道。

### 2.2 傳統經典藍牙 (BR/EDR - Classic Bluetooth)
* **協定**：BR/EDR ACL 基頻連結，上層載入：
  * **A2DP (Advanced Audio Distribution Profile)**：立體聲音樂輸出 (`AUDIO_SINK_UUID: 0x110B`)。
  * **AVRCP (Audio/Video Remote Control)**：播放、暫停、音量同步控制 (`0x110E`)。
  * **HFP / HSP (Hands-Free / Headset Profile)**：語音通話與麥克風 (`HANDSFREE_UUID: 0x111E`)。
* **特性**：
  * 必須由中央端 (PC) 或週邊端 (AirPods) 建立 ACL (Asynchronous Connection-Less) 實體無線鏈路。
  * 兩端皆有配對金鑰 (Link Key)。

---

## 3. Windows API 連線與斷線的底層機制分析

Windows 系統對於藍牙音訊裝置的「發起連線」與「主動斷線」，提供的 API 具備高度不對稱性：

### 3.1 主動斷線 (Disconnect) — 穩定可靠
* **API**：`bthprops.cpl` 導出的 Win32 函式 `BluetoothDisconnectDevice(&address)`。
* **運作機制**：
  * 直接向本機藍牙適配器 (Host Controller Interface, HCI) 發送 HCI 中斷 ACL 鏈路指令 (`HCI_Disconnect`)。
  * **完全不影響** Windows 註冊表中的服務註冊狀態，亦不卸載裝置驅動。
  * 斷線迅速（耗時約 20~50ms），斷開後裝置維持乾淨的「已配對但未連線」狀態。
  * Windows 系統設置與後續再次手動連線完全不受影響。

### 3.2 程式化發起連線 (Connect) — 核心死穴與陷阱

Windows 作業系統**並未向第三方應用程式開放官方、穩定的 Classic Bluetooth Audio 主動連線 API**。

#### 為什麼 Windows 系統「設定」可以連線，第三方不行？
* Windows「設定」與「處理中心 / 控制中心」使用的是微軟內部的私有介面（例如 `DevicePairingBroker`、`Windows.Devices.Enumeration.DeviceInformation.Pairing` 的內部實作及 PnP IOCTL），直接要求藍牙無線電硬體執行 Page 搜尋指定的藍牙 MAC 位址。
* WinRT 公開的 `Windows.Devices.Bluetooth.BluetoothDevice` 僅支援 RFCOMM / SPP 序列埠，**不允許**透過程式碼發起 A2DP/HFP 音訊設定檔的連線請求。

#### 嘗試使用 `BluetoothSetServiceState` 的致命問題
在開源社群中，常見的連線變通作法是呼叫 `bthprops.cpl` 的 `BluetoothSetServiceState(hRadio, &deviceInfo, &guidService, flags)`：

1. **直接調用 `flags = 1 (ENABLE)` 無效**：
   * 若裝置先前已配對，該服務在 Windows 註冊表中本來就標記為「已啟用 (1)」。
   * 直接呼叫 `flags = 1` 時，Windows 判定狀態未變更，會直接返回錯誤代碼 `87 (ERROR_INVALID_PARAMETER)`，**根本不會向藍牙硬體發出連線封包**。

2. **採用「0 (DISABLE) -> 延遲 -> 1 (ENABLE)」強制重啟的災難性後果**：
   * **PnP 驅動節點遭卸載**：
     傳入 `0` 時，Windows 會立即在裝置管理員中卸載該音效端點驅動節點（Tear Down），發送 PnP 移除通知。此時 WASAPI 正在監聽的音訊端點會瞬間變成 `AUDCLNT_E_DEVICE_INVALIDATED (0x88890004)`。
   * **Hands-Free Profile (HFP) 導致 13 秒同步阻塞**：
     若對 `HANDSFREE_UUID (0x111E)` 執行 0 -> 1，Windows 音訊堆疊會嘗試建立 RFCOMM AT 終端與 SCO 語音鏈路。然而 AirPods 在未通話狀態下通常不接受 SCO 協商，每次呼叫 Windows 會同步阻塞 **6.5 秒** 等待超時！連同停用與重啟，直接凍結背景執行緒 **13 秒**。
   * **驅動堆疊狀態損壞 (Driver Invalidation)**：
     若在短時間內連續觸發（例如使用者以為沒反應重複點擊），Windows Bluetooth 驅動堆疊會陷入競態條件（Race Condition），導致裝置雖然顯示連線，但 Windows 音訊系統完全偵測不到音訊輸出設備，甚至必須「移除裝置重新配對」才能修復。

---

## 4. Win2Pod 音路架構與回授保護 (Anti-Feedback Shield) 交互影響

在 Win2Pod 中，音訊架構為了支援 DSP 效果器 (10-Band EQ, Sound Enhancer, Warmth Enhancer) 以及即時動態頻譜可視化：

1. **替身端點 (Stand-in Proxy Device) 機制**：
   * Win2Pod 會將 Windows 系統預設音訊端點設為實體喇叭（替身端點並靜音防回授）。
   * 所有系統軟體將音訊輸出至此替身端點，Win2Pod 透過 WASAPI Loopback 擷取其音訊，經由 DSP 處理後，再透過獨立的 `LocalAudioRenderer` 串流渲染至 AirPods 端點。

2. **音路切換與「頻譜在動但耳機沒聲音」的現象**：
   * 當連線剛完成，若 Windows 系統自動將預設播放裝置指派為 AirPods，此時：
     $$\text{Loopback 擷取來源 (Capture)} = \text{AirPods 端點} = \text{渲染目的地 (Render)}$$
   * Win2Pod 內建的 **Anti-Feedback Shield (音訊回授保護)** 為了防止無限循環自激震盪（Echo/Feedback Loop），會主動關閉本地渲染。
   * **結果**：頻譜顯示有訊號（因為 Loopback 有收到聲音），但 AirPods 端完全靜音，直到手動切換 Windows 預設裝置使兩者端點分離後才正常發聲。

---

## 5. 結論與後續架構決策

經過多次實驗與分析，結論如下：

1. **放棄主介面「手動連線 AirPods」按鈕**：
   * Windows 底層缺乏第三方可靠呼叫的 Classic 藍牙連線介面。
   * 任何強行操作 `BluetoothSetServiceState` 的手段都會引發驅動節點撕裂、PnP 衝突、13 秒線程阻塞與音訊失效問題，嚴重破壞穩定性。

2. **堅持「配戴狀態感應 (In-Ear Detection)」為唯一音路切換樞紐**：
   * **正常工作流**：
     1. 使用者自充電盒取出 AirPods 並配戴。
     2. AirPods 硬體自動與 Windows 建立經典藍牙連線（由耳機主動向電腦發起連線，Windows 驅動反應最穩定）。
     3. Win2Pod 透過 BLE 廣播秒級偵測到 `worn_count >= 1`（入耳狀態）。
     4. Win2Pod 自動平滑將音路切換至 AirPods，並啟動立體聲 DSP 與頻譜。
     5. 摘下耳機後，Win2Pod 自動無縫切回 HomePod 或電腦原播放裝置。
   * 若 AirPods 偶爾未自動連線，引導使用者使用 Windows 系統原生快捷鍵：
     * **`Win + K`**（開啟 Windows 投放/藍牙音訊快速連線列表）
     * 系統「設定 > 藍牙與裝置」

3. **保留中斷連線與恢復機制**：
   * 摘下耳機或關閉盒蓋時，Win2Pod 忠實維持音路復原。
   * 播放器卡片與狀態列僅在真正使用 AirPods 音路時顯示電量與耳機圖示；若已斷開則正確還原為 HomePod / AirPlay 2 或電腦本機裝置名稱。
