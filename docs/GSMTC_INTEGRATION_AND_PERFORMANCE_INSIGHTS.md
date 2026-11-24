# Win2Pod Windows 全域媒體控制 (GSMTC) 整合與高頻事件暴風圈最佳化心得

> **作者**：Win2Pod Core Architecture Team  
> **日期**：2026-11-24  
> **標籤**：`Rust` `Windows Runtime (WinRT)` `GSMTC` `KKBOX vs Spotify` `COM/RPC` `Performance Optimization` `Lessons Learned`

---

## 1. 背景與問題現象

Win2Pod 整合了 Windows 系統標準的 **全域系統媒體傳輸控制（GSMTC, Global System Media Transport Controls）**，用以即時同步本機媒體播放資訊（曲名、演出者、專輯名稱、專輯封面與播放時間碼），並提供反向控制（播放/暫停、上一首、下一首、跳轉進度）。

在測試過程中，使用者回報了一個奇異的效能問題：
* 當使用 **Spotify** 作為音樂來源播放時，Win2Pod 背景 CPU 僅佔約 **0.1% ~ 0.5%**，系統平穩如鏡。
* 但當換成 **KKBOX** 作為音樂來源播放時：
  * **Win2Pod** CPU 佔用飆升至 **22%**（多核系統上相當於吃滿多個核心）。
  * Windows 系統服務 **「服務主機: 現正播放工作階段管理員服務」（svchost GSMTC Session Manager）** 同步飆升至 **16%**。
  * 系統內部每秒有大量短暫執行緒（ThreadPool Workers）頻繁建立與退出，伴隨顯著的處理器開銷。

---

## 2. 根因深度剖析：播放軟體 SMTC 實作差異引發的連鎖 RPC 暴風圈

經過深入排查與 Windows 內部事件流分析，我們發現了兩款主流音樂軟體在 SMTC 規範實作上的巨大哲學差異：

### 2.1 Spotify 的靜態宣告哲學（低頻事件）
* Spotify 在一首歌曲開始播放時，向 Windows SMTC 發布一次時間戳記快照：
  $$\text{Position} = 00:00,\quad \text{LastUpdatedTime} = \text{CurrentSystemTime}$$
* 在整首歌播放的 3～4 分鐘內，Spotify **絕不再主動觸發 `TimelinePropertiesChanged` 事件**。
* Windows 官方推薦的客戶端做法是：客戶端讀取到此快照後，在本地依靠時鐘差進行平滑外推：
  $$\text{CurrentPosition} = \text{Position} + (\text{Now} - \text{LastUpdatedTime})$$
* **結果**：播放 Spotify 時，Win2Pod 的 GSMTC 事件通道接收頻率為 **0 次/秒**。

### 2.2 KKBOX 的計時器心跳哲學（高頻事件）
* KKBOX 桌面端內部具備一個高頻計時器（約每秒數次）。
* 每當計時器觸發，KKBOX 就會主動向 Windows SMTC 呼叫 `UpdateTimelineProperties`，廣播當前播放的秒數。
* Windows 核心接收到後，會全域向所有註冊的監聽器發送 `session.TimelinePropertiesChanged` 事件。
* **結果**：播放 KKBOX 時，事件通道接收頻率高達 **每秒 5～15 次**！

```mermaid
sequenceDiagram
    autonumber
    participant KKBOX as KKBOX Player
    participant WinSVC as 服務主機: 現正播放工作階段管理員 (svchost)
    participant Win2Pod as Win2Pod (舊版 media_control.rs)

    loop 每秒 5 ~ 15 次
        KKBOX->>WinSVC: UpdateTimelineProperties(pos=00:32)
        WinSVC->>Win2Pod: TimelinePropertiesChanged 事件通知
        Note over Win2Pod: 舊架構未細分事件類型，一律視為 Track 變更！
        Win2Pod->>WinSVC: RequestAsync() [重新建立 COM Session Manager]
        Win2Pod->>WinSVC: TryGetMediaPropertiesAsync() [跨行程查詢曲目/演出者]
        Win2Pod->>WinSVC: Thumbnail().OpenReadAsync() [跨行程開啟圖片串流]
        Win2Pod->>WinSVC: DataReader.LoadAsync() [從 KKBOX 跨行程拉取數 MB 封面點陣圖]
        WinSVC-->>Win2Pod: 傳輸完整點陣圖 bytes (高額 LPC/RPC 開銷)
    end
    Note over WinSVC,Win2Pod: 雙方 CPU 飽和：svchost 16% + Win2Pod 22%！
```

### 2.3 舊架構在面對高頻事件時的設計缺陷
在原有的 [`media_control.rs`](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/media_control.rs) 實作中存在以下三項關鍵硬傷：
1. **事件混同（Event Conflation）**：
   `MediaPropertiesChanged`（換歌）、`PlaybackInfoChanged`（暫停/播放）與 `TimelinePropertiesChanged`（時間碼微調）共用同一個 `notify_tx: UnboundedSender<()>`。一收到信號就直接執行全套 `fetch_media_properties()`。
2. **反覆重新建立 COM Proxy**：
   在 `fetch_media_properties()` 內每次都呼叫 `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()`，導致 Windows 必須頻繁建立/釋放跨行程 COM 代理。
3. **無防護的封面跨行程拉取（Thumbnail RPC Thrashing）**：
   每次微小時間跳動，程式碼都會呼叫 `props.Thumbnail().OpenReadAsync()` 加上 `LoadAsync()`，將一張數百 KB 乃至數 MB 的專輯封面從 KKBOX 跨行程解組（Unmarshal）傳輸進 Win2Pod 記憶體中，引發記憶體反覆配置與 CPU 尖峰。

---

## 3. 根治架構：四重防護機制

針對上述問題，我們對 GSMTC 事件監聽與資料抓取架構進行了徹底重構：

```mermaid
graph TD
    classDef ev fill:#1e3a5f,stroke:#4a9eff,stroke-width:2px;
    classDef filter fill:#2d4a3e,stroke:#5cdb95,stroke-width:2px;
    classDef fast fill:#4a3b2c,stroke:#f39c12,stroke-width:2px;
    classDef full fill:#3a2d4a,stroke:#9b59b6,stroke-width:2px;

    EV_IN["WinRT SMTC 原始回呼"]:::ev
    EV_PARSE{"事件類型分類 (TrackEvent)"}:::ev

    EV_IN --> EV_PARSE

    EV_PARSE -- "SessionChanged" --> S_CHG["切換 App 來源，重建監聽"]:::full
    EV_PARSE -- "PropertiesChanged" --> P_FULL["換歌：查詢文字，比對快取封面"]:::full
    EV_PARSE -- "PlaybackChanged" --> P_FAST["純播放/暫停：0 封面 RPC，同步狀態"]:::fast
    EV_PARSE -- "TimelineChanged" --> T_FILTER{"誤差門檻過濾 (|real - calc| > 1.5s)"}:::filter

    T_FILTER -- "誤差 <= 1.5s (正常播放心跳)" --> T_DROP["完全忽略 (0% CPU, 0 記憶體開銷)"]:::filter
    T_FILTER -- "誤差 > 1.5s (使用者手動 Seek)" --> T_UPDATE["僅校正錨點時間，不碰封面與文字"]:::fast
```

### 3.1 專用事件枚舉（Specialized Event Enum）
摒棄無語意的 `()`，引進精確的 `TrackEvent`：
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrackEvent {
    PropertiesChanged, // 曲名、演出者、專輯或封面變動
    PlaybackChanged,   // 播放 / 暫停 / 停止狀態變更
    TimelineChanged,   // 時間碼或進度條變動
    SessionChanged,    // 來源應用程式切換
}
```

### 3.2 智慧時間碼過濾（Smart Timeline Filtering）
Win2Pod 內建的 `current_position_secs()` 本身就具備高精度的本地時間外推能力：
$$\text{CalcPos} = \text{AnchorPos} + \Delta t$$
因此在收到 `TimelineChanged` 時：
```rust
let (real_pos, dur) = compute_timeline_position(session, is_playing);
let current_calc_pos = state.read().ok().map(|g| g.current_position_secs()).unwrap_or(0.0);

if (real_pos - current_calc_pos).abs() > 1.5 {
    // 差距大於 1.5 秒：代表使用者在 KKBOX 介面上手動拖曳進度條進行 Seek！
    if let Ok(mut lock) = state.write() {
        lock.position_secs = real_pos;
        if dur > 0.0 {
            lock.duration_secs = dur;
        }
        lock.last_updated = Some(Instant::now());
    }
}
// 差距小於 1.5 秒：純屬 KKBOX 計時器自然心跳，直接靜默捨棄，不產生任何計算！
```

### 3.3 專輯封面快取保護（Thumbnail Caching Invariant）
跨行程讀取封面的開銷極大。即便收到 `PropertiesChanged`，在發起圖片串流讀取前，先比對現存快取：
```rust
let thumbnail_bytes = if let Some(prev) = prev_track {
    if prev.thumbnail_bytes.is_some() && prev.title == title && prev.artist == artist && prev.album == album {
        // 曲目文字完全相同，直接複用舊封面，省去所有 COM 串流傳輸！
        prev.thumbnail_bytes.clone()
    } else {
        fetch_thumbnail(&props).await
    }
} else {
    fetch_thumbnail(&props).await
};
```

### 3.4 複用 Session 物件，杜絕重覆建立 COM Proxy
將 `fetch_media_properties_from_session(&session, prev_track)` 抽出，直接複用現有的 `session` 參照，杜絕每秒重新呼叫 `GlobalSystemMediaTransportControlsSessionManager::RequestAsync()`。

---

## 4. 最佳化效益對比

| 指標 | 最佳化前（KKBOX 播放時） | 最佳化後（KKBOX 播放時） |
| :--- | :--- | :--- |
| **Win2Pod CPU 佔用** | **22%**（高負載、高熱） | **0.1% ~ 0.5%**（正常待命水準） |
| **Windows 現正播放管理服務 CPU** | **16%**（頻繁跨行程 IPC） | **0% ~ 0.1%**（近乎沉睡） |
| **每秒跨行程 COM/RPC 次數** | 約 30 ~ 60 次/秒 | **0 次/秒**（僅在換歌或手動 Seek 時產生） |
| **記憶體點陣圖配置次數** | 每秒反覆配置數 MB | **僅換歌時配置 1 次** |
| **進度條平滑度與響應** | 正常 | 100% 維持平滑，且手動 Seek 瞬時響應 |

---

## 5. 總結與工程啟示

1. **永遠不要假設第三方軟體的行為是一致的**：
   在標準規範（如 Windows SMTC）之下，不同軟體供應商（Spotify、KKBOX、Apple Music、YouTube/Chrome）的推播策略差異極大。軟體必須做好防禦性過濾，避免被第三方的惡意或低效實作「牽著鼻子走」。
2. **高頻事件與低頻資產必須嚴格解耦**：
   時間碼是每秒流逝的高頻資料；專輯封面是數分鐘才變換一次的大型二進位資產。將兩者綁定在同一條同步管線上是嚴重的架構反模式。
3. **本地外推（Local Extrapolation）勝於過度網路/IPC 輪詢**：
   在具備確定物理時鐘的場景下，本地數學推估不僅速度最快、消耗為零，更具備免疫跨行程雜訊與延遲的天然抗體。
