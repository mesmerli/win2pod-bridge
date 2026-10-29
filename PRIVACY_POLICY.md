# Privacy Policy for Win2Pod Bridge

**Effective Date:** October 29, 2026  
**Last Updated:** October 29, 2026  
**Developer:** mesmerli  
**Contact:** mesmerli@outlook.com (or via GitHub Issues)  

---

## 1. Overview
Win2Pod Bridge ("the Application", "we", "us", or "our") is designed to stream Windows desktop audio to Apple HomePod and AirPlay-compatible audio receivers over your local home network.

We respect your privacy and are committed to protecting it. **Win2Pod Bridge does NOT collect, store, transmit, or share any personal information, telemetry, usage statistics, or audio recordings.**

---

## 2. Information Handling and Permissions

### 2.1 Audio Data (WASAPI Loopback Capture)
- **Purpose:** The Application captures real-time desktop audio output solely to encode (ALAC) and transmit it directly to your selected AirPlay / HomePod speakers.
- **Privacy Guarantee:** All audio processing occurs entirely in local volatile memory (RAM). Audio is **never recorded, never saved to disk, and never transmitted to external cloud servers or third parties**.

### 2.2 Local Network Discovery & Communication (mDNS, RTSP, RTP)
- **Purpose:** The Application scans your local area network (LAN) using standard multicast DNS (mDNS / Bonjour) to discover AirPlay devices, and establishes a direct, encrypted local network connection (AirPlay RTSP / RTP) to stream audio.
- **Privacy Guarantee:** All network communication is strictly confined to your local network between your PC and your designated audio receiver. No network traffic is routed through or monitored by any external server.

### 2.3 Microsoft Store Licensing & In-App Purchases
- **Purpose:** If installed via the Microsoft Store, the Application queries the official Windows Store API (`Windows.Services.Store`) to verify trial status and license ownership.
- **Privacy Guarantee:** All transaction, payment, and billing details are handled directly and exclusively by Microsoft Corporation in accordance with the [Microsoft Privacy Statement](https://privacy.microsoft.com/). The Application never receives or accesses your credit card, billing address, or personal Microsoft account credentials.

### 2.4 Local Configuration Storage
- **Purpose:** The Application saves your user preferences (e.g., UI language, buffer duration, speaker volume, auto-mute preference) locally on your device in `%APPDATA%\win2pod\config.json`.
- **Privacy Guarantee:** This configuration file contains no personally identifiable information (PII) and never leaves your computer.

---

## 3. Third-Party Services & Tracking
- **No Analytics / No Tracking:** The Application contains no advertising SDKs, analytics tracking (e.g., Google Analytics, Firebase), or telemetry frameworks.
- **No Remote Servers:** We do not operate or connect to any remote backend server.

---

## 4. Children's Privacy
Because Win2Pod Bridge does not collect any personal data, it does not knowingly collect or solicit personal information from children under the age of 13 (or applicable age in your jurisdiction).

---

## 5. Changes to This Privacy Policy
We may update this Privacy Policy from time to time. Any changes will be posted on this page with an updated effective date.

---

## 6. Contact Us
If you have any questions, concerns, or feedback regarding this Privacy Policy, please contact us via:
- GitHub: [https://github.com/mesmerli/win2pod-bridge](https://github.com/mesmerli/win2pod-bridge)
- Email: mesmerli (via GitHub repository contact)

---

# 隱私權政策 (繁體中文版)

**生效日期：** 2026 年 10 月 29 日  
**最後更新：** 2026 年 10 月 29 日  
**開發者：** mesmerli  

---

## 1. 簡介
Win2Pod Bridge（以下簡稱「本應用程式」）旨在透過本機區域網路，將 Windows 電腦音訊串流傳輸至 Apple HomePod 及支援 AirPlay 的音訊設備。

我們非常重視您的隱私權。**Win2Pod Bridge 不會收集、儲存、上傳或分享任何個人身分資料、遙測數據、使用記錄或音訊錄音。**

---

## 2. 資料處理與權限說明

### 2.1 音訊資料處理 (WASAPI 音訊擷取)
- **目的：** 本應用程式僅擷取系統即時音訊輸出，並即時編碼（ALAC）傳送給您指定的 HomePod 或 AirPlay 喇叭。
- **隱私保障：** 所有音訊資料均僅在本機記憶體（RAM）中作即時暫存與串流處理，**絕不錄製、絕不寫入硬碟儲存，亦絕不傳送至任何外部伺服器或第三方**。

### 2.2 區域網路搜尋與通訊 (mDNS、RTSP、RTP)
- **目的：** 本應用程式使用標準 mDNS（Bonjour）在您的區域網路（LAN / Wi-Fi）內搜尋可用的 AirPlay 裝置，並建立點對點加密串流連線。
- **隱私保障：** 所有通訊皆嚴格限制於您的本機內部網路（PC 與音響之間），絕不透過外部伺服器轉發，亦無任何外聯監控行為。

### 2.3 微軟商店授權與購買驗證
- **目的：** 透過微軟商店 API（`Windows.Services.Store`）查詢軟體的試用期與授權狀態。
- **隱私保障：** 所有付款、結帳與微軟帳號驗證均由微軟官方依據 [微軟隱私權聲明](https://privacy.microsoft.com/) 處理。本應用程式完全無法取得您的信用卡號、帳單地址或個人帳戶機密。

### 2.4 本機偏好設定
- **目的：** 本應用程式會在本機路徑（`%APPDATA%\win2pod\config.json`）儲存您的個人偏好（如介面語言、音量、緩衝時間、連線時自動靜音等）。
- **隱私保障：** 該設定檔僅存放本機偏好參數，不含任何個人資訊，且不會上傳至網路。

---

## 3. 第三方服務與追蹤
- 本應用程式**完全沒有**置入任何第三方廣告 SDK、資料分析工具（如 Google Analytics、Firebase）或遙測追蹤模組。
- 開發者並未建置任何收集用戶資料的遠端伺服器。

---

## 4. 兒童隱私保護
本應用程式不收集任何個人資料，因此絕不會收集或索取 13 歲以下兒童的個人資訊。

---

## 5. 聯絡我們
若您對本隱私權政策有任何疑問，請透過 GitHub 專案頁面與開發者聯繫：  
GitHub: [https://github.com/mesmerli/win2pod-bridge](https://github.com/mesmerli/win2pod-bridge)
