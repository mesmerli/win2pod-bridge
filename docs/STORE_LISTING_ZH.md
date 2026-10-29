# 微軟商店產品描述 - 中文（繁體）(Win2Pod Bridge)

## 1. 產品名稱 (Product Name)
```text
Win2Pod Bridge
```

---

## 2. 簡短描述 (Short Description - 270 字元限制內)
*(顯示於搜尋結果預覽、類別卡片與宣傳橫幅)*

```text
將 Windows 10 & 11 電腦音訊無損即時串流至 Apple HomePod、HomePod mini 及 AirPlay 2 智慧音響。支援立體聲配對、多房間同步廣播、低延遲 ALAC 編碼與現代化 Fluent 浮動控制面板。
```
*(字數約 110 字，遠低於 270 字元上限)*

---

## 3. 完整產品說明 (Full Description)
*(請直接複製並貼上至微軟合作夥伴中心 Partner Center 的「描述 / Description」欄位)*

```text
將您的 Windows PC 瞬間升級為強大的 AirPlay 2 高傳真發送端！

Win2Pod Bridge 是一款專為 Windows 10 與 Windows 11 設計的現代化、極致輕量高傳真音訊串流橋接工具。透過 Windows 原生底層 WASAPI Loopback 擷取技術，以 44.1 kHz / 16-bit 雙聲道無損 ALAC（Apple Lossless Audio Codec）格式，將電腦上的音樂、影片聲音與應用程式音效，即時、低延遲地傳輸至 Apple HomePod、HomePod mini、Apple TV 以及任何相容 AirPlay 2 的智慧音響。

Windows 原生無法發送 AirPlay 2 協定，且 Apple HomePod 嚴格要求 HomeKit Transient 暫態加密配對握手，導致市面上多數傳統工具無法支援。Win2Pod Bridge 完全採用純 Rust 語言從底層實現軟體級加密配對，無需改機、無需 Apple MFi 硬體授權晶片，更無需安裝龐大繁雜的背景輔助常駐程式，即可享受原汁原味的頂級聲學體驗！

============================================================
核心特色與重點功能
============================================================

🎵 專為 APPLE HOMEPOD 與 AIRPLAY 2 深度打造
• HomeKit 暫態加密配對：完整支援最新韌體版本的 HomePod 與 HomePod mini，純軟體級加密通訊。
• 無損 ALAC 即時壓縮編碼：以 44.1 kHz / 16-bit 雙聲道高保真 ALAC 即時封包傳輸，重現無損錄音室級原音。
• IEEE 802.1AS / gPTP 時鐘同步：採用 Mac 級別的 BMCA 協商讓出與高精度時鐘校時，徹底告別聲音抖動、雜音與時延漂移。

🔊 HOMEPOD 立體聲組與多房間同步廣播 (MULTI-ROOM)
• 自動識別立體聲組 (Stereo Pair)：智慧偵測並整合已配對的左右聲道 HomePod，一鍵開啟身歷其境的環繞立體音場。
• 多裝置多房間廣播：支援同時勾選多台不同房間的 AirPlay 音響，全家同步聆聽同一首樂曲。
• 動態加退群組：串流進行中可隨時「＋ 加入播放」新音響或退出單一音響，完全不影響正在播放中的串流。
• 獨立與總體音量調節：群組串流時提供總音量滑桿，並可針對每一台音響獨立微調專屬音量。

🎨 現代化 WINDOWS 11 FLUENT 浮動面板與系統匣
• Fluent 懸浮控制面板：採用純 Rust（egui / eframe）打造，完美契合 Windows 11 現代暗色美學。
• 狀態感知系統匣圖示：即時呈現待命（灰）、連線中（橘）、串流中（綠）、異常（紅）四種視覺狀態與系統通知。
• 極致記憶體優化：面板縮小至系統匣時自動休眠 UI 繪製並釋放 GPU/OpenGL 快取，背景記憶體佔用極低（僅約 10–20 MB RAM）。
• 靜默開機常駐：支援隨 Windows 開機自動於背景靜默啟動（--startup / --minimized），不打擾桌面操作。

🎛️ 音訊分流模式 (SPLIT MODE)
• 全系統混音擷取：一鍵側錄電腦的所有音效、瀏覽器影片與音樂播放器。
• 獨立端點分流：可自由指定特定播放裝置作為音訊擷取來源。例如：將 Spotify / Apple Music 獨立傳送至 HomePod，而 Discord 通話或遊戲語音依然保留在電腦耳機。
• 快速開啟音量混音器：內建按鈕直接喚起 Windows 原生應用程式音量混音設定。

⚡ 智慧電源與播放輔助
• 串流自動靜音電腦喇叭 (Auto-Mute)：投播時自動將 PC 本機喇叭靜音，防止雙重發聲回音與延遲干擾；中斷連線或結束程式時自動精準還原原始音量。
• 智慧防休眠機制：僅在「有實際音訊播放」時自動阻止 Windows 進入休眠，靜音或暫停時自動釋放電源鎖。
• 低延遲模式切換：可自由調整緩衝區設定，滿足即時反應與影音同步需求。
• 記住前次設備自動重連 (Auto-Connect)：程式啟動時自動嘗試連線前次使用的音響設備。
• 自訂全域快捷鍵：支援隨處以鍵盤快速鍵（預設 Ctrl + Alt + H）一鍵切換串流開關。

🌐 完整支援 11 國語系
內建繁體中文、簡體中文、English、Français、Deutsch、Italiano、日本語、한국어、Português (Brasil)、Русский、Español，並自動依據 Windows 系統語言自動切換。

============================================================
系統需求與設定提示
============================================================
• 作業系統：Windows 10 (1809 以上版本) 或 Windows 11 (64-bit)。
• 網路環境：Windows 電腦與 HomePod / AirPlay 喇叭必須連接至同一個區域網路（Wi-Fi 或有線 LAN）。
• Windows 防火牆設定：由於 AirPlay 2 協定需要音響回連電腦的 RTSP 與 PTP 時鐘校時連接埠，初次使用時請在系統防火牆放行入站連線。
```

---

## 4. 產品重點特色條目 (Product Features - 每條 120 字元內)
*(請將下列項目逐條填入 Partner Center 的「特色 / Features」欄位)*

1. `44.1 kHz / 16-bit 雙聲道高保真 ALAC（Apple Lossless Audio Codec）即時無損音訊串流`
2. `HomeKit Transient 暫態加密配對：免硬體 MFi 晶片，完美相容 Apple HomePod 與 HomePod mini`
3. `自動辨識 HomePod 立體聲組 (Stereo Pair)，一鍵實現左右雙聲道沉浸式音場`
4. `多房間同步廣播 (Multi-Room)：支援同時將 Windows 電腦聲音串流至多台 AirPlay 2 音響`
5. `動態群組管理：串流中隨時「＋ 加入播放」或退出音響，支援總音量與獨立音量滑桿`
6. `音訊分流模式 (Split Mode)：可將特定應用程式音樂獨立串流至音響，通話與遊戲音效保留在耳機`
7. `底層原生低延遲 WASAPI Loopback 系統音訊擷取管線`
8. `高精度 IEEE 802.1AS (gPTP) 讓出式時鐘同步機制，徹底消除聲音抖動與延遲漂移`
9. `現代化 Windows 11 Fluent 浮動控制面板與四態狀態感知系統匣圖示`
10. `極致輕量化記憶體優化：縮小至系統匣時背景佔用僅約 10–20 MB RAM`
11. `串流自動靜音本機喇叭 (Auto-Mute)，消除雙重發聲回音，中斷時自動恢復音量`
12. `智慧播放防休眠：僅在有實際音訊播放時阻止系統睡眠，安靜時自動釋放電源鎖`
13. `自訂全域快捷鍵（預設 Ctrl + Alt + H），隨時一鍵切換串流開關`
14. `智慧自動重連 (Auto-Connect)：啟動時自動嘗試連線前次使用的音響設備`
15. `內建 11 種國際語言，自動跟隨 Windows 系統語系，亦可隨時手動自選`

---

## 5. 搜尋關鍵字 (Search Terms - 至多 7 組，每組 30 字元內)
```text
1. HomePod
2. AirPlay
3. AirPlay Windows
4. HomePod Windows
5. 音訊串流
6. 多房間音訊
7. ALAC 無損
```

---

## 6. 著作權與商標免責聲明 (Copyright & Trademark Info)
```text
Copyright © 2026 mesmerli. 版權所有。
Apple、HomePod、HomePod mini、Apple TV 及 AirPlay 為 Apple Inc. 於美國及其他國家註冊之商標。Win2Pod Bridge 為獨立開源開發專案，與 Apple Inc. 無任何附屬、贊助、授權或背書關係。
```

---

## 7. 系統需求與補充資訊 (System Requirements Notes)
```text
• 電腦與 HomePod / AirPlay 喇叭必須處於同一個區域網路（Wi-Fi 或乙太網路）。
• 初次使用時，Windows 防火牆必須放行 Win2Pod Bridge 的入站連線，以確保音響可正常回連進行時鐘同步與音訊傳輸。
```

---

## 8. 版本發行說明 (Release Notes / What's New for v0.1.0)
```text
Win2Pod Bridge 首發版本 (v0.1.0)：
• 支援將 Windows 電腦音訊以無損 ALAC 串流至 Apple HomePod、HomePod mini 與 AirPlay 2 音響。
• 支援 HomePod 立體聲組自動辨識與多房間群組同步播放。
• 新增音訊分流模式 (Split Mode)，可自由指定音訊擷取端點。
• 現代化 Windows 11 Fluent 懸浮控制面板與狀態感知系統匣圖示。
• 內建串流自動靜音、智慧防休眠與自訂全域快捷鍵。
• 內建 11 國語系，啟動自動適配 Windows 系統語言。
```
