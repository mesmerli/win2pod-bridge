# Win2Pod 視窗磁吸架構設計與踩坑復盤心得

> **作者**：Win2Pod Core Architecture Team  
> **日期**：2026-11-20  
> **標籤**：`Rust` `egui/eframe` `Win32 Subclassing` `Window Snapping` `Dock Tree Forest` `Lessons Learned`

---

## 1. 背景與設計目標

在打造現代音訊串流與播放工具 **Win2Pod** 的過程中，我們希望重現經典播放器（如 **Winamp**）最受玩家喜愛的靈魂功能——**多視窗磁力吸附與叢集一體化聯動（Magnetic Snapping & Docking Cluster）**。

Win2Pod 包含四個獨立用途的視窗：
1. **主控制視窗（Main Window）**：AirPlay 設備管理、設定、系統托盤常駐。
2. **迷你播放器（Mini Player）**：輕巧精緻的懸浮播放器，提供專輯封面、即時波形、VFD 液晶面板與控制鈕。
3. **10 頻段音訊等化器（Audio Equalizer）**：提供 DSP 等化調整、暖音增強與預設切換。
4. **即時視覺特效視窗（Visualizer）**：基於 WebView2 與 WebGL / Three.js 的全螢幕/視窗化 3D 頻譜動畫。

### 核心設計目標
- **自然貼齊（Magnetic Snapping）**：視窗靠近螢幕工作區邊緣、其他視窗邊緣（$\le 12\text{px}$）時產生磁吸咬合。
- **叢集聯動（Group Movement）**：合體為群組後，拖曳主導視窗可帶動所有相吸視窗一體平滑移動。
- **自由拆解（Natural Severance）**：使用者刻意向外拉拽子視窗（位移 $> 16\text{px}$）即可「撕開」磁吸，脫離隊伍獨立自由移動。
- **群組意識與動態子群組（Sub-cluster Autonomy）**：即便主視窗關閉或收至系統匣，桌面上殘留的副視窗（如迷你播放器與等化器）仍能自成體系、持續磁吸並相互帶動。

---

## 2. 磁吸架構演進：從一對一 Hardcode 到「有向擴展樹狀森林」

```mermaid
graph TD
    classDef main fill:#1e3a5f,stroke:#4a9eff,stroke-width:2px;
    classDef mini fill:#2d4a3e,stroke:#5cdb95,stroke-width:2px;
    classDef eq fill:#4a3b2c,stroke:#f39c12,stroke-width:2px;
    classDef vis fill:#3a2d4a,stroke:#9b59b6,stroke-width:2px;

    Main["主視窗 (Lv 1)"]:::main
    Mini["迷你播放器 (Lv 2)"]:::mini
    EQ["等化器視窗 (Lv 3)"]:::eq
    Vis["視覺特效 (Lv 4)"]:::vis

    Main -- "磁吸邊 (Right/Top)" --> Mini
    Mini -- "磁吸邊 (Bottom)" --> EQ
    Main -- "磁吸邊 (Right/Bottom)" --> Vis
```

### 階段一：一對一的死胡同
在最初期，系統只考慮「主視窗」與「視覺化視窗」的吸附，採用單一靜態變數 `DOCK_BOND: Mutex<DockBond>` 來記錄二者的相對偏移。
當引進第三個視窗（等化器）與第四個視窗（迷你播放器）後，狀態維護迅速崩潰：
- 關係從 1 對 1 暴增為多對多無向圖。
- 容易產生**環狀依賴（Cycle Loops）**：A 帶動 B、B 帶動 C、C 又把 A 拖回原點，導致座標振盪或畫面抖動。
- 無法表達「誰聽誰的」：兩個視窗合體時，移動其中一個到底誰該跟隨？

### 階段二：有向無環樹狀森林（Directed Dock Tree Forest）
我們重構為以圖論為基石的**有向無環擴展樹森林架構**：

1. **嚴格優先級層次（Priority Hierarchy）**：
   $$\text{Main (Level 1)} > \text{MiniPlayer (Level 2)} > \text{Equalizer (Level 3)} > \text{Visualizer (Level 4)}$$
2. **單一父節點不變量（Master-Slave Invariant）**：
   - 任何合體的從屬視窗（Slave）在 Dock Tree 中**至多只能有 1 個 Master（父節點）**。
   - 樹的拓撲結構保證是無環的（Directed Acyclic Graph），從數學上徹底消滅死循環。
3. **非對稱拉力（Asymmetric Pull Dynamics）**：
   - **向下擴散（Master $\to$ Slaves）**：當 Master 移動時，透過 `propagate_child_moves` 沿著樹狀分支遞迴更新所有子節點座標。
   - **向上剪枝（Slave $\to$ Master）**：當使用者拖曳 Slave 視窗時，Master **絕不會被帶動**；若位移超過容差門檻（$16\text{px}$），系統判定使用者意圖拆開，立即剪斷該邊（Sever edge）並調用 `rebuild_dock_tree()`。
4. **動態子群組根節點提升（Subcluster Root Promotion）**：
   若主視窗脫離或關閉，殘留在桌面上的副視窗群組會重新進行連通元件分解（Connected Components），由優先權最高者（如 Mini Player）自動升格為該子群組的根節點。

---

## 3. Win32 底層訊息攔截機制

`eframe` / `egui` 本身在跨平台上是定時渲染，無法提供 Windows 原生拖曳過程中的高頻同步。我們使用 Win32 **視窗子類化（Window Subclassing）** 作為核心引擎：

```mermaid
sequenceDiagram
    participant User as 使用者拖曳
    participant Win32 as Windows DWM / Win32
    participant Subclass as unified_window_subclass_proc
    participant Snapping as snapping.rs (Dock Engine)
    participant Egui as eframe (UI Rendering)

    User->>Win32: 滑鼠拖曳視窗標題列
    Win32->>Subclass: WM_MOVING
    Subclass->>Snapping: handle_window_moving()
    Note over Snapping: 檢查工作區邊界 & 磁吸候選者
    Snapping-->>Subclass: 修改 RECT 指標 (實時對齊吸附)
    Win32->>Subclass: WM_WINDOWPOSCHANGED
    Subclass->>Snapping: propagate_child_moves()
    Note over Snapping: 遞迴更新所有 Slave 視窗座標
    Snapping->>Win32: SetWindowPos(child_hwnd)
    Subclass->>Egui: 傳遞訊息維持 UI 更新
```

### 關鍵 Win32 訊息職責
- `WM_MOVING`：攔截即將發生的位移矩形指標（`lparam as *mut RECT`），原地修正座標達成即時吸附貼合。
- `WM_WINDOWPOSCHANGED`：當視窗位置確定改變且非縮小，發送 `propagate_child_moves` 帶動整個子樹。
- `WM_EXITSIZEMOVE`：拖曳放手瞬間校驗所有已吸附的邊，若視窗間隙已大於耐受值則斷開連接。
- `WM_MOUSEACTIVATE` / `WM_ACTIVATE`：調用 `bring_cluster_to_top`，透過 `SetWindowPos(top_tier)` 連鎖提升整個群組的 Z-Order，防止視窗夾在其他軟體中間。
- `WM_SYSCOMMAND (SC_CLOSE)`：攔截系統關閉指令，觸發自定義群組連帶關閉（Cluster Close）。

---

## 4. 視窗群組釘選與 Z-Order 管理規範（Window Pinning & Z-Order Specification）

在多視窗磁吸系統中，視窗除了在 $(X, Y)$ 平面上吸附聯動，在 $Z$ 軸深度（Z-Order）與系統置頂層級（Topmost Tier）上也必須具備嚴格的群組一致性。若缺乏體系化約束，容易引發「UI 按鈕顯示未釘選，視窗卻物理常駐最上層」等幽靈置頂現象。

```mermaid
flowchart TD
    classDef ssot fill:#1e3a5f,stroke:#4a9eff,stroke-width:2px;
    classDef top fill:#4a3b2c,stroke:#f39c12,stroke-width:2px;
    classDef notop fill:#2d4a3e,stroke:#5cdb95,stroke-width:2px;

    SSOT["全域原子核心: IS_ALWAYS_ON_TOP (SSOT)"]:::ssot
    UserToggle["使用者點擊 📌 釘選按鈕"] --> SSOT

    SSOT -->|is_pinned = true| PinCluster["set_cluster_pinned(true)"]:::top
    SSOT -->|is_pinned = false| UnpinCluster["set_cluster_pinned(false)"]:::notop

    subgraph TopmostTier ["置頂層級 (WS_EX_TOPMOST)"]
        PinCluster --> HWND_TOPMOST["SetWindowPos(HWND_TOPMOST)"]
        HWND_TOPMOST --> W1["主視窗"]
        HWND_TOPMOST --> W2["迷你播放器"]
        HWND_TOPMOST --> W3["等化器"]
        HWND_TOPMOST --> W4["視覺效果視窗"]
    end

    subgraph NormalTier ["普通層級 (非置頂)"]
        UnpinCluster --> HWND_NOTOPMOST["SetWindowPos(HWND_NOTOPMOST)"]
        HWND_NOTOPMOST -->|徹底清除 WS_EX_TOPMOST 旗標| NW1["全員回歸普通 Z 軸"]
    end
```

### 核心釘選規則體系

#### 規則 1：單一事實來源（Single Source of Truth, SSOT）
- **核心開關統一**：全系統視窗的釘選狀態以全域原子變數 `IS_ALWAYS_ON_TOP` 及 Master 視窗（`FlyoutApp.is_always_on_top`）為最高唯一事實來源。
- **禁止狀態分裂**：歷史設定中個別殘留的子視窗釘選值（如 `visualizer_pinned`）在視窗接入工作站群組時，**強制被 Master 視窗當前的釘選狀態覆寫同化**，嚴禁副視窗持有相衝突的獨立釘選狀態。

#### 規則 2：叢集原子連鎖釘選（Cluster Pin Atomicity）
- **一處釘選，全員置頂（Pin One, Pin All）**：當使用者在 Master 視窗（或任一依附節點）點擊釘選圖釘時，系統調用 `set_cluster_pinned(origin, true)`，沿著 Dock Tree 遍歷所有成員，原子級同步套用 `HWND_TOPMOST`。
- **一處解除，全員解脫（Unpin One, Unpin All）**：解除釘選時，遍歷全員調用 `HWND_NOTOPMOST`，並同步持久化設定。
- **獨立視窗豁免（Standalone Exemption）**：若使用者刻意撕開副視窗獨立使用（例如拉開視覺特效視窗至副螢幕），該視窗才享有獨立的釘選能力；但只要重新吸附回集群，立即同化回集群狀態。

#### 規則 3：Win32 Z-Order 階層降級鐵律（`HWND_NOTOPMOST` 徹底取代 `HWND_TOP`）
- **Win32 API 原生特性**：
  - `HWND_TOPMOST`：賦予視窗 `WS_EX_TOPMOST`，送入置頂層。
  - `HWND_NOTOPMOST`：**唯一能抹除 `WS_EX_TOPMOST`** 並將視窗降回普通層的指令。
  - `HWND_TOP`：僅將視窗移至**其當前所屬層級的頂部**。若視窗已具備置頂樣式，傳入 `HWND_TOP` **完全無法解除置頂**！
- **強制執行標準**：
  在非置頂狀態（`!is_topmost`）下，所有視窗維護動作（包括 `bring_cluster_to_top`、`layout_modular_workstation`、`dock_spawned_window` 滑入動畫、`bring_main_to_front` 托盤喚醒），**一律強制傳入 `HWND_NOTOPMOST`**，堅決封殺任何因使用 `HWND_TOP` 造成的幽靈置頂殘留。

#### 規則 4：主從焦點防護與防逆向感染（Anti-Infection Shield）
- **病因背景**：副視窗（如 WebView2 視覺特效）獲取焦點（`WM_ACTIVATE` / `WM_SETFOCUS`）時，若在訊息迴圈中擅自執行 `SetWindowPos(main_hwnd, HWND_TOPMOST)`，會導致主視窗在 UI 按鈕未點亮的情況下被「暗中感染」為實體最上層。
- **防禦機制**：副視窗只有在處於「全螢幕展示」或「全域 `IS_ALWAYS_ON_TOP == true`」的明確條件下，才允許請求母視窗置頂；非置頂狀態下嚴禁向外傳播 `HWND_TOPMOST`。

#### 規則 5：全螢幕覆蓋特例（Fullscreen Override）
- 當視覺特效進入全螢幕時，自動提升為最上層覆蓋 Windows 工作列；此時若呼叫主控制介面（HUD），主視窗被暫時性提升至全螢幕視覺效果之上以供操作；一旦退出全螢幕，立即自動復原叢集原本的釘選層次。

---

## 5. 關鍵挑戰與深度踩坑復盤（Lessons Learned）

在本次多視窗磁吸架構落地的過程中，我們遭遇了五個極為隱蔽且深度的系統級難題，在此留下寶貴的排查經驗與防範心法：

### 坑點 1：WebView2 COM 物件銷毀重入崩潰（Panic `0xffeaecffffffffff` 與 Exit Code `-1`）
- **現象**：
  在主視窗與視覺特效視窗（WebView2）合體狀態下關閉主視窗，程式有機率發生致命崩潰：
  `FATAL PANIC ... misaligned pointer dereference: address must be a multiple of 0x8 but is 0xffeaecffffffffff`
  或直接以 Exit Code `0xffffffff` 瞬間閃退。
- **深度病因**：
  1. 視覺特效視窗是透過 `wry` 託管的 Edge WebView2。
  2. 當群組觸發 `close_window_cluster` 時，Windows 發送 `WM_CLOSE` 呼叫 `DestroyWindow`。
  3. 但此時 WebView2 內部的 `parent_subclass_proc` 仍在 Win32 訊息隊列中處理消息。
  4. 視窗句柄銷毀與 COM 物件（`ICoreWebView2Controller`）釋放發生微秒級別的競爭，導致已釋放的虛擬方法表指標（VTable）被嘗試解引用（產生負數無效指標 `0xffeaecffffffffff`），觸發 Chromium 底層的 `TerminateProcess(GetCurrentProcess(), -1)`。
- **解法心法**：
  - **切斷通訊在先**：在關閉群組時，第一步永遠是**先解散磁吸關係、切斷視窗間通訊**。
  - **原子鎖保護**：設立 `VISUALIZER_CLOSING` 原子旗標，並在調用銷毀前立刻將 `VISUALIZER_HWND` 歸零，使後續訊息直接由 `DefSubclassProc` 放行，阻斷重複銷毀。

---

### 坑點 2：`eframe` 單幀 Viewport 銷毀陷阱（The 1-Frame Viewport Teardown Trap）
- **現象**：
  拉開迷你播放器後關閉主視窗，迷你播放器沒有關閉（正確），但會**突然瞬間瞬移到螢幕正中央**。
- **深度病因**：
  1. 主視窗點擊 `X` 關閉時，在 [gui.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/gui.rs) 中攔截到 `close_req`，執行了收起群組後直接調用了 **`return;`** 提前退出當前幀的渲染函數。
  2. **`eframe` 的即時視窗生命週期機制**：任何副視窗（`show_viewport_immediate`）如果在**哪怕僅僅一幀**之內沒有被呼叫渲染，`eframe` 就會判定該視窗已不存在，直接呼叫 Win32 `DestroyWindow` 將其原生視窗銷毀！
  3. 下一幀中，由於 `mini_player_window_open` 仍為 true，`eframe` 又重新建立了一個全新的原生視窗。
  4. 新視窗初始化時走入「出生定位（Spawn Position）」邏輯；由於主視窗此時在托盤，無可吸附錨點，系統退回兜底邏輯（Fallback: Screen Center），導致視窗跳回螢幕正中央！
- **解法心法**：
  - **絕對不能在幀循環中無故早退**：移除 `return;`，改由主面板跳過機制（`CentralPanel` 依據 `is_hidden` 條件渲染）讓生命週期無縫延續。只要副視窗在每一幀都持續呼叫 `show_viewport_immediate`，原生 HWND 與座標就能永久固化。

---

### 坑點 3：全域托盤旗標污染副視窗（Global Tray Guard Poisoning）
- **現象**：
  迷你播放器與等化器磁吸合體後關閉主視窗，二者外觀看起來是一體、關閉時也會一併關閉，但是**拖曳迷你播放器時，等化器紋絲不動**。
- **深度病因**：
  1. 主視窗縮小至系統匣時，為了避免它被系統完全關閉，採取將座標移至離屏 `(-32000, -32000)` 的做法。
  2. 為了防止移至 -32000 時誤觸發吸附，開發者在 `unified_window_subclass_proc` 頂端加了一行：
     `if crate::tray::is_window_hidden_to_tray() { return DefSubclassProc(...); }`
  3. **這是一個全域判斷，套用到了所有視窗**！
  4. 當主視窗在托盤時，桌面上存活的迷你播放器和等化器在收到 `WM_MOVING` / `WM_WINDOWPOSCHANGED` 時，全部直接被提早 Return，負責帶動子節點的 `propagate_child_moves` 完全沒機會執行。
- **解法心法**：
  - **護盾必須精準對焦**：將過濾條件改為 `if kind == WindowKind::Main && is_window_hidden_to_tray()`。任何時候都必須意識到「主視窗收進托盤」並不等於「整個應用程式停止運作」，副視窗應享有獨立完整的事件泵。

---

### 坑點 4：吸附（Snap）與脫離（Sever）的滯後容差區間（Hysteresis）
- **現象**：
  若將「吸附距離」與「脫離距離」設定為相同數值（例如均為 12px），使用者在拉扯視窗時，視窗會在「被吸附」與「被脫離」之間高速劇烈跳動（抖動現象）。
- **解法心法**：
  - 引入**遲滯效應（Hysteresis）**：
    - **吸附門檻（Snap Distance）**：$12\text{px}$（只有靠得夠近才吸上去）。
    - **脫離門檻（Sever Distance）**：$16\text{px}$（必須刻意拉開超過 $16\text{px}$ 才會切斷）。
    - 兩者間的 $4\text{px}$ 緩衝區有效保障了手感的安定度。

---

### 坑點 5：Z-Order 狀態脫鉤與「幽靈置頂」逆向感染（Ghost Topmost & Reverse Pin Infection）
- **現象**：
  使用者在程式剛啟動時點擊「打開工作站配置」，標題列上的釘選按鈕呈現未激活的灰色（`is_always_on_top == false`），但所有視窗卻實體頑固浮在桌面最頂層，擋住其他桌面程式；使用者必須按一下釘選鈕（按鈕變金黃色，視窗維持置頂），再按一下（按鈕變灰），全體視窗才真正解除最上層。
- **深度病因（四重因素交織）**：
  1. **設定檔狀態分裂**：系統記錄了 `always_on_top`（主程式）與 `visualizer_pinned`（視覺特效）兩組開關；歷史執行殘留使視覺特效記錄為 `true`，主程式為 `false`。
  2. **副視窗逆向感染母視窗**：工作站組裝時視覺特效視窗啟動，在收到 `WM_ACTIVATE` / `WM_SETFOCUS` 時，因自身 `is_pinned == true` 為了防止擋住母視窗，主動呼叫 `SetWindowPos(main_hwnd, HWND_TOPMOST)`，將主視窗物理升格為置頂視窗。
  3. **集群連帶牽引**：工作站對齊與磁吸群組遍歷時，所有依附視窗順勢被掛在主視窗之後，全員被推入 Topmost Tier。
  4. **Win32 API 原生陷阱**：在非置頂邏輯中，系統原先使用 `top_tier = if is_topmost { HWND_TOPMOST } else { HWND_TOP }`。在 Windows 規範中，**`HWND_TOP` 僅將視窗排至其所屬層次頂部，根本無法剝除 `WS_EX_TOPMOST` 樣式**！只有顯式傳入 `HWND_NOTOPMOST` 才能將置頂視窗降級。
- **解法心法**：
  - **開關嚴格同化（SSOT）**：工作站組裝與視覺特效出生時，釘選狀態強制鎖定為 Master 狀態，禁止雙軌獨立。
  - **阻斷逆向感染**：副視窗只有在全域 `IS_ALWAYS_ON_TOP == true` 時才准許將母視窗提拔為置頂。
  - **全面以 `HWND_NOTOPMOST` 徹底降級**：在非置頂狀態下，全面使用 `HWND_NOTOPMOST` 執行 SetWindowPos，保證原子級剝除幽靈置頂樣式。

---

### 坑點 6：歷史遺留雙視窗硬編碼引發的「跨群組遠程幽靈吸引」（Cross-Cluster Ghost Attraction & Home Summon Severance）
- **現象**：
  在工作站群組拆分後，桌面由「迷你播放器 + 迷你等化器 + 視覺視窗」三者組成一個相吸群組，主視窗處於系統匣隱藏中。當使用者在**迷你播放器**上點擊「🏠（Home 房子按鈕）」希望召喚主視窗合體時，主視窗現身的一瞬間，**視覺視窗竟然如同被磁鐵吸住一般瞬間脫離原群組、被強行吸引到了主視窗身邊**，並與主視窗結合成獨立的一組；而迷你播放器和等化器則被孤立留在原處。
- **深度病因（歷史遺留代碼引發的拓撲撕裂）**：
  1. 在系統早期僅有「主視窗」與「視覺化視窗」雙視窗架構時，開發者在 `tray.rs` 的 `bring_main_to_front()` 尾端加入了一段「還原對齊視覺視窗」的邏輯：
     ```rust
     let bond = crate::snapping::get_dock_bond();
     if bond.is_bonded && !vis_hwnd.is_null() ... {
         let (new_vis_x, new_vis_y) = calc_bonded_visualizer_pos(&bond, &vis_rect, &main_rect);
         SetWindowPos(vis_hwnd, hwnd, new_vis_x, new_vis_y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
     }
     ```
  2. **盲目假設父節點必定為主視窗**：這段代碼僅僅檢查了 `bond.is_bonded == true`，**完全沒有校驗 `bond.parent == WindowKind::Main`**！
  3. 當視覺視窗是吸附在迷你播放器身上時，其 `is_bonded` 亦為 `true`（但母節點為 `MiniPlayer`）。當主視窗從托盤現身時，這段代碼將主視窗舊坐標作為 `master_rect` 傳入，瞬間強制以 `SetWindowPos` 將視覺視窗拉移數百像素至主視窗身側。
  4. 視覺視窗瞬間移位導致其與迷你播放器之間的物理間隔遠大於 24px，在緊接著觸發的 `rebuild_dock_tree()` 中，間隔過大的邊被依法剔除，視覺視窗正式與原群組決裂，轉而與主視窗結為新組。
- **解法心法**：
  - **杜絕任何繞過圖論引擎的私自移位**：現代的擴展樹森林（Dock Tree Forest）已由 `bring_fleet_to_top()` 中的 `restore_and_realign_all_children` 根據嚴密的母子拓撲統一維護。徹底刪除 `tray.rs` 中私自搬移視窗的遺留代碼。
  - **召喚加入以發起者（Anchor Kind）為基準**：主視窗受召喚現身時，本身只是一塊「等待加入群組」的新圖元，絕不能在抵達目標群組前擅自動搖桌面上任何現存視窗的位移。

---

### 坑點 7：全螢幕退出時的「主視窗殭屍復活」（Zombie Resuscitation on Fullscreen Exit）
- **現象**：
  使用者在視覺特效全螢幕模式下呼出 HUD 並點擊了主視窗的 ⛝（關閉／隱藏至系統匣）；然而當按下 `F` 或 `Escape` 切換退出全螢幕回到普通視窗時，**原本已收納至系統匣的主視窗竟然又神秘地自動彈回桌面**！
- **深度病因（全螢幕退出回調中的過度熱心）**：
  1. 在 `visualizer.rs` 的 `toggle_visualizer_fullscreen` 退出分支中，早期開發者為了確保使用者切回視窗化時能看到控制介面，寫了一段防護：
     ```rust
     if crate::tray::is_window_hidden_to_tray() {
         crate::tray::bring_main_to_front();
     } else {
         crate::snapping::bring_fleet_to_top();
     }
     ```
  2. **嚴重忽視使用者在全螢幕下主動關閉主視窗的意志**：當使用者在全螢幕中刻意關閉主視窗，主視窗已合法收合至托盤（`is_window_hidden_to_tray() == true`）。
  3. 一旦退出全螢幕，這段過時的邏輯看到 `is_window_hidden_to_tray() == true`，竟擅自調用 `bring_main_to_front()` 強制將主視窗從 `(-32000, -32000)` 離屏坐標抓回螢幕中央並還原視窗！
- **解法心法**：
  - **尊重使用者的隱藏意志，全面交由 `bring_fleet_to_top()` 統一過濾**：
    刪除強行喚醒邏輯，統一呼叫 `bring_fleet_to_top()`。在 `bring_fleet_to_top()` 內部已具備嚴密的托盤守衛：
    ```rust
    if kind == WindowKind::Main && crate::tray::is_window_hidden_to_tray() {
        continue; // 隱藏於托盤的主視窗老老實實留於托盤，絕不驚擾
    }
    ```
  - 確保視覺特效視窗切回普通模式時，僅調整自身樣式與可見視窗的層級，主視窗維持使用者所期望的隱藏狀態。

---

## 6. 極致視覺體驗：Mac 風格 Muelle Genie S-Curve 神燈最小化效果實踐

除了視窗間的剛性磁吸與聯動，視窗在隱藏與還原時的過渡動畫直接決定了應用的工藝質感。我們在 Windows 平台上以純 Win32 GDI + Layered Window 成功復刻了 macOS 最具辨識度的經典特效——**Muelle Genie Effect（阿拉丁神燈 S 曲線漏斗收縮動畫）**。

```mermaid
flowchart TD
    Capture["1. 原生 BitBlt 截取視窗當前點陣圖"] --> LayeredWin["2. 建立全螢幕 WS_EX_LAYERED 透明頂層畫布"]
    LayeredWin --> SMath["3. Muelle S-curve Hermite 三次樣條數學模型計算"]
    SMath --> Scanline["4. 逐掃描線 (Per-Scanline 1px) 次像素平滑光柵化"]
    Scanline --> TrayTarget["5. 動態托盤坐標 (Tray Rect) 精確命中"]
    TrayTarget --> UpdateLayer["6. UpdateLayeredWindow (ULW_ALPHA) 60 FPS 渲染"]
```

### 演進與技術攻堅復盤

#### 1. 拒絕粗糙階梯：從分塊 StretchBlt 到逐掃描線次像素光柵化
- **問題**：早期嘗試將視窗縱向切成 60 塊長條矩形執行 `StretchBlt`，在動畫播放時會出現明顯的「階梯狀橫條拉絲」與鋸齒斷層，完全喪失流體美感。
- **解法**：捨棄逐塊拉伸，改採 **逐掃描線（Per-Scanline 1px）次像素抗鋸齒平滑光柵化**（詳見 [genie_effect.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/genie_effect.rs)）：
  - 建立 32 位元預乘 Alpha DIBSection（`BI_RGB` 32bpp）。
  - 對畫布每一行掃描線（Y 坐標），精確計算 S 曲線的左右邊界邊緣 $[x_{\text{left}}, x_{\text{right}}]$。
  - 透過水平非線性縮放公式，將原始截圖像素平滑對應並線性混合同步寫入記憶體，徹底消除所有鋸齒與階梯感。

#### 2. Muelle S-Curve 數學模型
採用 macOS 經典的 Hermite 曲線模型，根據動畫進度 $t \in [0, 1]$ 與垂直位置 $u \in [0, 1]$ 計算漏斗形狀：
$$S(t) = 3t^2 - 2t^3$$
$$w(u) = (1 - u) \cdot W_{\text{orig}} + u \cdot W_{\text{tray}}$$
在收斂至托盤目標時，底部以優雅的指數收斂曲線逐漸緊貼托盤圖示中心點，形成拉絲漏斗般的流體視覺。

#### 3. 系統匣目標座標精準鎖定
- **問題**：Windows 系統工作列常駐眾多圖示（電池、網路、音量、自訂程式），早期粗略定位容易偏移至相鄰的電池膠囊或時鐘。
- **解法**：在 [tray.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/tray.rs) 中利用 `tray_icon.rect()` 即時獲取自身圖示的物理螢幕 Rect，將神燈漏斗的收縮終點精確綁定在 Win2Pod 圖示的幾何中心點，達成「自圖示中流出、吸回圖示中隱沒」的一體化沉浸感。

---

## 7. 靈動懸浮：視窗原生半透明化（Layered Opacity）與懸浮智慧窺視（Hover Peek）

在多視窗工作站生態中，「迷你播放器」與「視覺特效視窗」常被使用者長時間置頂於螢幕一隅，一邊寫代碼或瀏覽網頁、一邊享受背景音樂與律動。然而，全不透明的視窗會遮蔽後方的編輯器文字，而固定半透明又會導致想看清歌曲或操作按鈕時閱讀困難。

我們為此量身打造了**「原生層疊半透明 + 智慧懸浮窺視（Hover Peek）」**雙軌機制：

```mermaid
stateDiagram-v2
    [*] --> SemiTransparent: 使用者設定透明度 (20% ~ 90%)
    SemiTransparent --> Hovered: 滑鼠游標移入 (MouseEnter)
    Hovered --> Solid: 毫秒級自動過渡至 100% 不透明 (Hover Peek)
    Solid --> MouseLeave: 滑鼠離開視窗 (MouseLeave)
    MouseLeave --> SemiTransparent: 自動漸變回目標透明度
    
    note right of Solid: 確保歌名、進度條、按鈕高對比清晰可讀
    note right of SemiTransparent: 柔和退居背景，不干擾主畫面工作
```

### 關鍵機制與技術細節

#### 1. Win32 原生 `WS_EX_LAYERED` 硬體加速混合
- 在 [snapping.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/snapping.rs#L107-L135) 中實作 `apply_native_window_opacity(hwnd, opacity)`：
  - 透過 `GetWindowLongW` / `SetWindowLongW` 附加 `WS_EX_LAYERED` 延伸樣式。
  - 呼叫 Win32 原生 API `SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA)`，由 Windows DWM 桌面視窗管理員在 GPU 合成階段以零 CPU 開銷完成 Alpha 混合。
- **性能優化（動態樣式剝除）**：
  當透明度為 $100\%$（`opacity >= 0.999`）時，系統主動以 `SetWindowLongW` 剝除 `WS_EX_LAYERED` 樣式並觸發 `SWP_FRAMECHANGED`，使視窗完整回退至標準原生渲染通道，完全避免分層視窗帶來的微量記憶體與 DWM 轉譯開銷。

#### 2. 安全下限不變量（Safety Clamping Invariant: 20% ~ 100%）
- **踩坑點**：若允許透明度一路降至 $0\%$，視窗會在畫面上徹底「人間蒸發」。使用者既看不見也無法點擊，會誤以為程式崩潰或丟失視窗。
- **架構規範**：嚴格強制夾取（Clamp）透明度區間在 $[0.20, 1.00]$。最低 $20\%$ 保證了在任何暗色或淺色桌布上，視窗輪廓與圖示始終隱約可辨，絕不丟失。

#### 3. 智慧懸浮窺視（Hover Peek / Intelligent Auto-Reveal）
- 在 [visualizer.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/visualizer.rs#L535-L560) 與迷你播放器中：
  - 平時維持安靜半透明（例如 $40\%$），宛如毛玻璃雕刻般懸浮於背景。
  - 只要使用者的滑鼠游標掠過或移入視窗範圍（Hover），系統立刻無感切換至 $100\%$ 不透明，所有歌詞、曲名、光柱與按鈕瞬間立體飽滿；移出後無痕還原，兼顧工作專注與操作便利。

#### 4. Alt + 滑鼠滾輪無級微調（Fast Gesture Tuning）
- 使用者無需深入層層設定選單，只要游標懸浮在迷你播放器或視覺視窗上，按住鍵盤 `Alt` 鍵並滾動滑鼠滾輪，即可即時以 $5\%$ 步進流暢升降透明度，手感極度直覺。

#### 5. 叢集同化與設定持久化
- 每個視窗的半透明數值（`mini_player_opacity`, `visualizer_opacity`）與 Hover Peek 開關（`hover_peek_enabled`）皆寫入設定檔（`controller.rs`）。
- 當副視窗組裝吸附至群組時（`assimilate_cluster_to_anchor`），視窗的樣式狀態會平滑調和，重啟後亦能百分之百原樣復原。

---

## 8. 分工明確的標題列控制：視窗角色徽章、叢集釘選與單獨脫隊最小化（Cluster vs Solo Controls）

在多個視窗合體為一個大叢集時，若每個視窗都保留各自的「🗕 最小化、📌 置頂、⛝ 關閉」，會引發嚴重的操作認知混亂：
- 使用者點擊附屬視窗的 🗕，到底該收起自己、還是收起整組？
- 點擊 📌 釘選，附屬視窗置頂了、但母視窗在底下，視窗會不會被其他外部程式夾在中間「切成千層糕」？

我們為此重構了**「視窗角色自適應與標題列按鈕動態分工」**機制：

```mermaid
flowchart LR
    subgraph Cluster ["磁吸群組 (Cluster)"]
        Master["Master 視窗 🧲<br/>(藍色微光邊框)"]
        Slave1["Slave 從屬視窗 📎<br/>(無縫緊湊邊框)"]
        Slave2["Slave 從屬視窗 📎<br/>(無縫緊湊邊框)"]
        Master --> Slave1
        Slave1 --> Slave2
    end

    subgraph Actions ["標題列按鈕操作策略"]
        direction TB
        BtnMin["🗕 最小化按鈕<br/>- 預設點擊：整組最小化 (Cluster Minimize)<br/>- Shift+點擊：脫隊單獨最小化 (Solo Minimize)"]
        BtnPin["📌 釘選按鈕<br/>- 預設點擊：全群組原子級同步置頂 (Cluster Pinning)<br/>- HUD 呼出：臨時提拔，關閉時精確還原自癒"]
        BtnClose["⛝ 關閉按鈕<br/>- 預設點擊：整組關閉<br/>- Shift+點擊：脫隊單獨關閉"]
    end

    Cluster -.-> Actions
```

### 關鍵機制與技術細節

#### 1. 三態視窗角色（WindowRole）與動態標題列適配
在 [snapping.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/snapping.rs#L435-L460) 中定義三種角色：
- **`WindowRole::ClusterMaster`（叢集母艦 🧲）**：
  - 角色：圖論連通分量中優先權最高（Level 1）的根節點。
  - 外觀：標題列顯示「🧲」徽章，四周帶有 `rgb(70, 140, 215)` 的科技藍主控微光邊框。
  - 職責：擁有全套控制權，掌管全群組的移動、置頂、收納與關閉。
- **`WindowRole::ClusterSlave`（從屬僚機 📎）**：
  - 角色：依附於 Master 之後的子節點。
  - 外觀：標題列顯示「📎」吸附徽章，邊框轉為緊湊暗色 `rgb(45, 50, 60)`。
  - **標題列動態精簡（`set_window_caption_buttons`）**：在 [snapping.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/snapping.rs#L947) 中，系統為 Slave 視窗自動精簡或隱藏重複的系統按鈕，避免畫面雜亂。
- **`WindowRole::Standalone`（獨立個體）**：
  - 未與任何人吸附，外框呈標準中性灰 `rgb(52, 58, 70)`，標題列無徽章，擁有標準單視窗控制邏輯。

#### 2. 最小化按鈕雙模態：全體進退 vs 單獨脫隊
- **預設點擊 🗕（Cluster Minimize）**：
  - 當使用者點擊群組視窗上的最小化按鈕時，系統透過圖論遞迴遍歷整個連通塊，將全員一併收縮最小化，確保工作站整體狀態不被破壞。
- **Shift + 點擊 🗕（Solo Minimize）**：
  - 使用者情境：只想暫時收起「視覺特效」或「等化器」，留下「迷你播放器」繼續聽歌。
  - 實現邏輯（[snapping.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/snapping.rs#L1740-L1760) `minimize_window_solo`）：
    1. 主動調用 `break_dock_relation(target)`，乾淨俐落地將該視窗從有向樹中剪斷（Sever edge）。
    2. 調用 Win32 `ShowWindow(hwnd, SW_MINIMIZE)` 僅最小化該視窗。
    3. 留在桌面上的其餘視窗毫髮無損，系統自動觸發 `rebuild_dock_tree()` 重新推舉剩餘群組的 Master，完成無縫自治過渡。

#### 3. 釘選置頂原子級全域傳播（Cluster Pinning Propagation）
- **核心痛點**：Windows 作業系統底層並不知曉「視窗群組」概念。若群組中有的視窗是 `HWND_TOPMOST`，有的不是，當使用者切換至第三方瀏覽器時，非置頂視窗會被沉入瀏覽器背後，而置頂視窗仍浮在前方，群組實體被「橫向攔腰切斷」。
- **解法心法**：
  - 在 [snapping.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/snapping.rs#L165-L220) 中實作 `toggle_window_pinned(origin)`：
  - 點擊任一合體視窗的 📌 釘選按鈕時，系統取得該群組全體成員，**原子級連帶將全員同步設為置頂或非置頂**（全面傳入 `HWND_TOPMOST` 或 `HWND_NOTOPMOST`）。
  - 若使用者在副視窗上按住 `Shift + 點擊 📌`，則自動剪枝脫離群組，僅單獨切換自身置頂。

#### 4. 全螢幕 HUD 釘選備份與退出自癒還原（HUD Pinned State Backup & Restore）
- 在 [visualizer.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/visualizer.rs#L20-L45) 中：
  - 當使用者在全螢幕視覺特效下按 `W` 呼出控制中心 HUD 時，系統必須暫時將主視窗等視窗提升至 `HWND_TOPMOST` 才能顯示在全螢幕之上。
  - **狀態備份機制**：呼出 HUD 前，系統先記憶全體成員原先的真實釘選偏好（`WAS_PINNED` 暫存簿）。
  - **退出自癒**：當關閉 HUD 或退出全螢幕時，系統忠實依照暫存簿還原各視窗的原本置頂屬性，絕不破壞使用者的桌面日常設定。

---

## 9. 新使用者體驗：模組化工作站呼吸光環與智慧導引氣泡（Onboarding Beacon & Balloon）

Win2Pod 雖然具備極致的「四視窗磁吸合體」與「3 欄模組化工作站」能力，但新使用者首次啟動應用程式時，看到的僅是一面精緻簡約的主視窗，極易錯過「點擊標題列左上角 App 圖示即可一鍵展開四重視窗合體」的靈魂功能。

為了在**不強迫彈窗、不打擾老玩家**的前提下達成「一眼即懂」，我們在 [gui.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/gui.rs#L2760-L2920) 實作了漸進式發現引導體系：

```mermaid
flowchart TD
    Launch["使用者啟動應用程式"] --> Check{"是否曾組裝過工作站？<br/>(workstation_discovered)"}
    Check -- "是 (已探索過)" --> NormalState["正常安靜渲染 App 圖示，無任何打擾"]
    Check -- "否 (首次使用且工作站未開啟)" --> GuideState["啟動新使用者專屬引導機制"]
    
    subgraph Guide ["引導呈現機制"]
        Beacon["1. 正弦波呼吸動態光圈 (Pulsing Halo Beacon)<br/>以 3.2 rad/s 平滑泛出科技藍光暈"]
        Balloon["2. 半透明毛玻璃懸浮導引氣泡 (Floating Balloon)<br/>介紹工作站合體功能 + 立即體驗 / 知道了"]
    end
    
    GuideState --> Beacon
    GuideState --> Balloon
    
    Balloon -- "點擊『立即體驗』" --> Assemble["觸發 assemble_modular_workstation()<br/>標記永久已探索 (持久化至 config)"]
    Balloon -- "點擊『知道了』" --> Dismiss["關閉氣泡<br/>標記永久已探索 (持久化至 config)"]
```

### 關鍵機制與技術細節

#### 1. 正弦波呼吸微光光環（Pulsing Halo Beacon）
- 在 [gui.rs](file:///C:/Users/mesme/workspace/win2pod-bridge/crates/homepod-cast-gui/src/gui.rs#L2776-L2790) 中：
  - 根據全局即時時間戳計算連續正弦波強度：
    $$\text{pulse} = \frac{\sin(\text{time} \times 3.2) + 1}{2} \in [0.0, 1.0]$$
  - 在 App 圖示外圍繪製半徑與透明度動態呼吸擴散的圓角藍光環（擴展 $1.5 \sim 4.7\text{px}$，Alpha 動態介於 $45 \sim 185$）。
  - 既能第一時間捕捉新使用者的視覺焦點，又完全不發出聲音或強奪輸入焦點。

#### 2. 半透明毛玻璃導引氣泡卡片（Onboarding Balloon Card）
- 依託 `egui::Area` 錨定於標題列 App 圖示下方，具備深色毛玻璃底色、天藍細邊框與柔和擴散陰影（Blur 16px）。
- 支援完整的全語系本地化（繁中、簡中、英文、日文等）：
  - 標題：💡 探索模組化工作站（Discover Modular Workstation）
  - 說明：一鍵展開並自動拼裝：主視窗、迷你播放器、等化器與視覺效果！
  - 雙按鈕互動：
    - **「立即體驗」（藍色高亮按鈕）**：點擊後立即啟動 `assemble_modular_workstation()`，透過階梯式彈簧阻尼動畫（Staggered Spring）平滑將全體 4 個視窗依序拼裝到位。
    - **「知道了」（低調暗色按鈕）**：點擊後安靜收起氣泡。

#### 3. 單一事實來源與開關持久化（State Self-Healing & Persistence）
- 不論使用者是點擊「立即體驗」、「知道了」、抑或是直接點擊了 App 圖示，系統一律觸發：
  ```rust
  self.workstation_onboarding_dismissed = true;
  crate::controller::save_workstation_discovered(true);
  ```
- 該狀態永久寫入使用者本機設定檔（`workstation_discovered: true`），往後啟動永不重複彈出，徹底杜絕了讓進階使用者感到煩躁的「牛皮癬提示」。

---

## 10. 架構核心成果與檢驗

經過此番體系化梳理與重構，Win2Pod 達成如下標準行為：

| 操作情境 | 行為表現 | 核心保證機制 |
| :--- | :--- | :--- |
| **四視窗任意拼裝** | 支援右側排列、頂部拼裝、底部等化器多重組合 | BFS 樹遍歷 + 依賴驗證 |
| **拖曳母視窗** | 全體相連子節點以 60 FPS 幀率鎖定同步移動 | `propagate_child_moves` |
| **拖曳子視窗** | 脫離當前母視窗，母視窗不動，子視窗獨立移出 | `remove_dock_edge` 向上剪枝 |
| **關閉主視窗** | 觸發 Mac 風格 S 曲線神燈特效吸入托盤，副視窗持續成組運作 | Muelle Genie Effect + 精準托盤防護 |
| **按副視窗 🏠 召喚** | 主視窗優雅平滑飛回並完美融入群組，絕不破壞既有子群組 | 徹底消除硬編碼搬移 + 拓撲一致性 |
| **點擊任一合體視窗** | 群組內所有視窗原子級同步置頂，不被第三方視窗穿插 | `bring_cluster_to_top` Z-Order 秩序 |
| **點擊任一合體視窗 📌** | 全叢集視窗原子級同步切換置頂／非置頂，杜絕幽靈置頂 | `set_cluster_pinned` + `HWND_NOTOPMOST` 規範 |
| **Shift + 點擊 🗕** | 脫隊單獨最小化特定視窗，剩餘叢集保持原位並自癒推舉 Master | `minimize_window_solo` 剪枝與重構 |
| **半透明視窗懸浮互動** | 平時安靜半透明，滑鼠移入自動 100% 窺視，Alt+滾輪微調 | `apply_native_window_opacity` + `Hover Peek` |
| **新使用者首次啟動** | 標題列按鈕呼吸微光 + 懸浮導引氣泡，體驗後永久隱藏 | `workstation_discovered` 狀態持久化 |
| **點擊任一合體視窗 X** | 連鎖關閉該連通群組中的所有合體視窗 | `close_window_cluster` 遍歷關閉 |

---

## 11. 結語

視窗磁吸吸附表面上看似只是「計算座標貼齊」，但一旦引入**多視窗自由組合、主從聯動、生命週期異步化與跨架構渲染框架（Win32 + egui + wry）**，其複雜度便會呈幾何級數上升。

本次實踐的關鍵體悟在於：
1. **嚴謹的資料結構先行**（樹狀森林與單一父節點約束，徹底杜絕死循環）。
2. **通訊生命週期大於一切**（在視窗銷毀與重繪交界處，精準的狀態原子隔離是防止閃退的不二法門）。
3. **全局狀態需極度克制**（避免全域旗標污染獨立副視窗的事件泵）。
4. **Z 軸與平面享有同等嚴謹的階層規範**（單一事實來源、杜絕狀態分裂與逆向感染，並善用 Win32 `HWND_NOTOPMOST` 清除非預期置頂）。
5. **歷史代碼必須及時隨架構升級而重構**（早期雙視窗時代的便宜行事代碼，在多視窗拓撲體系下會成為難以察覺的「幽靈撕裂者」，唯有堅持 SSOT 原則方能維護系統純潔性）。
6. **細節成就非凡質感**（從多塊鋸齒拉絲到逐掃描線次像素光柵化的 Muelle S-curve，數學模型的嚴謹性決定了使用者體驗的精緻上限）。
7. **互動的人性化細節（Affordance）**（半透明視窗搭配 20% 安全下限、智慧 Hover Peek 與手勢滾輪無級微調，讓靈活性與可用性達到最佳平衡）。
8. **功能的發現性與非侵入式引導（Discoverability without Annoyance）**（善用動態正弦波光圈與單次觸達氣泡卡片，使隱蔽而強大的進階功能自然傳遞給新使用者）。

這份經驗不僅讓 Win2Pod 成功重現經典 Winamp 的迷人操作手感與 macOS 的靈動神燈視效，更為未來進一步擴充更多模組視窗立下了堅實可靠的架構基礎。
