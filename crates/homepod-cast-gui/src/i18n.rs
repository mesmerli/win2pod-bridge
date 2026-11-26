//! Internationalization (i18n) module for Win2Pod Bridge.
//! Supports 11 languages:
//! - Chinese (Simplified)
//! - Chinese (Traditional)
//! - English (United States)
//! - French
//! - German
//! - Italian
//! - Japanese
//! - Korean
//! - Portuguese (Brazil)
//! - Russian
//! - Spanish

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    SimplifiedChinese,   // zh-Hans
    TraditionalChinese,  // zh-Hant
    English,             // en-US
    French,              // fr
    German,              // de
    Italian,             // it
    Japanese,            // ja
    Korean,              // ko
    PortugueseBrazil,    // pt-BR
    PortuguesePortugal,  // pt-PT
    Indonesian,          // id-ID
    Vietnamese,          // vi-VN
    Russian,             // ru
    Spanish,             // es
}

pub const ALL_LANGUAGES: &[Language] = &[
    Language::TraditionalChinese,
    Language::SimplifiedChinese,
    Language::English,
    Language::French,
    Language::German,
    Language::Italian,
    Language::Japanese,
    Language::Korean,
    Language::PortugueseBrazil,
    Language::PortuguesePortugal,
    Language::Indonesian,
    Language::Vietnamese,
    Language::Russian,
    Language::Spanish,
];

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "zh-Hant",
            Self::SimplifiedChinese => "zh-Hans",
            Self::English => "en-US",
            Self::French => "fr",
            Self::German => "de",
            Self::Italian => "it",
            Self::Japanese => "ja",
            Self::Korean => "ko",
            Self::PortugueseBrazil => "pt-BR",
            Self::PortuguesePortugal => "pt-PT",
            Self::Indonesian => "id-ID",
            Self::Vietnamese => "vi-VN",
            Self::Russian => "ru",
            Self::Spanish => "es",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code.trim().to_lowercase().as_str() {
            "zh-hant" | "zh-tw" | "zh-hk" => Some(Self::TraditionalChinese),
            "zh-hans" | "zh-cn" | "zh-sg" | "zh" => Some(Self::SimplifiedChinese),
            "en-us" | "en" | "en-gb" => Some(Self::English),
            "fr" | "fr-fr" | "fr-ca" => Some(Self::French),
            "de" | "de-de" | "de-at" | "de-ch" => Some(Self::German),
            "it" | "it-it" => Some(Self::Italian),
            "ja" | "ja-jp" => Some(Self::Japanese),
            "ko" | "ko-kr" => Some(Self::Korean),
            "pt-pt" => Some(Self::PortuguesePortugal),
            "pt-br" | "pt" => Some(Self::PortugueseBrazil),
            "id" | "id-id" | "in" | "in-id" => Some(Self::Indonesian),
            "vi" | "vi-vn" => Some(Self::Vietnamese),
            "ru" | "ru-ru" => Some(Self::Russian),
            "es" | "es-es" | "es-mx" => Some(Self::Spanish),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "繁體中文 (Chinese Traditional)",
            Self::SimplifiedChinese => "简体中文 (Chinese Simplified)",
            Self::English => "English (United States)",
            Self::French => "Français (French)",
            Self::German => "Deutsch (German)",
            Self::Italian => "Italiano (Italian)",
            Self::Japanese => "日本語 (Japanese)",
            Self::Korean => "한국어 (Korean)",
            Self::PortugueseBrazil => "Português do Brasil (Portuguese Brazil)",
            Self::PortuguesePortugal => "Português de Portugal (Portuguese Portugal)",
            Self::Indonesian => "Bahasa Indonesia (Indonesian)",
            Self::Vietnamese => "Tiếng Việt (Vietnamese)",
            Self::Russian => "Русский (Russian)",
            Self::Spanish => "Español (Spanish)",
        }
    }

    /// Auto-detect Windows system UI language via Win32 API.
    pub fn detect_system_language() -> Self {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetUserDefaultUILanguage() -> u16;
        }

        let langid = unsafe { GetUserDefaultUILanguage() };
        let primary = langid & 0x03FF;
        let sub = langid >> 10;

        match primary {
            0x04 => {
                if sub == 0x02 || sub == 0x04 {
                    Self::SimplifiedChinese
                } else {
                    Self::TraditionalChinese
                }
            }
            0x09 => Self::English,
            0x0C => Self::French,
            0x07 => Self::German,
            0x10 => Self::Italian,
            0x11 => Self::Japanese,
            0x12 => Self::Korean,
            0x16 => {
                if sub == 0x02 {
                    Self::PortugueseBrazil
                } else {
                    Self::PortuguesePortugal
                }
            }
            0x21 => Self::Indonesian,
            0x2A => Self::Vietnamese,
            0x19 => Self::Russian,
            0x0A => Self::Spanish,
            _ => Self::English,
        }
    }

    // -------------------------------------------------------------
    // UI String Translations
    // -------------------------------------------------------------

    pub fn btn_refresh(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => " ⟳ 重新整理 ",
            Self::SimplifiedChinese => " ⟳ 刷新 ",
            Self::English => " ⟳ Refresh ",
            Self::French => " ⟳ Actualiser ",
            Self::German => " ⟳ Aktualisieren ",
            Self::Italian => " ⟳ Aggiorna ",
            Self::Japanese => " ⟳ 再読み込み ",
            Self::Korean => " ⟳ 새로고침 ",
            Self::PortugueseBrazil => " ⟳ Atualizar ",
            Self::PortuguesePortugal => " ⟳ Atualizar ",
            Self::Indonesian => " ⟳ Segarkan ",
            Self::Vietnamese => " ⟳ Làm mới ",
            Self::Russian => " ⟳ Обновить ",
            Self::Spanish => " ⟳ Actualizar ",
        }
    }

    pub fn btn_back(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⬅ 返回",
            Self::SimplifiedChinese => "⬅ 返回",
            Self::English => "⬅ Back",
            Self::French => "⬅ Retour",
            Self::German => "⬅ Zurück",
            Self::Italian => "⬅ Indietro",
            Self::Japanese => "⬅ 戻る",
            Self::Korean => "⬅ 뒤로",
            Self::PortugueseBrazil => "⬅ Voltar",
            Self::PortuguesePortugal => "⬅ Voltar",
            Self::Indonesian => "⬅ Kembali",
            Self::Vietnamese => "⬅ Quay lại",
            Self::Russian => "⬅ Назад",
            Self::Spanish => "⬅ Volver",
        }
    }

    pub fn tooltip_close_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關閉此獨立視窗",
            Self::SimplifiedChinese => "关闭此独立窗口",
            Self::English => "Close this standalone window",
            Self::French => "Fermer cette fenêtre",
            Self::German => "Dieses Fenster schließen",
            Self::Italian => "Chiudi questa finestra",
            Self::Japanese => "このウィンドウを閉じる",
            Self::Korean => "이 창 닫기",
            Self::PortugueseBrazil => "Fechar esta janela",
            Self::PortuguesePortugal => "Fechar esta janela",
            Self::Indonesian => "Tutup jendela ini",
            Self::Vietnamese => "Đóng cửa sổ này",
            Self::Russian => "Закрыть это окно",
            Self::Spanish => "Cerrar esta ventana",
        }
    }

    pub fn badge_cluster_master(&self) -> &'static str {
        "🧲"
    }

    pub fn tooltip_cluster_master(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "磁吸組合主控視窗（移動、最小化或關閉時連動全體組合成員）",
            Self::SimplifiedChinese => "磁吸组合主控窗口（移动、最小化或关闭时联动全体成员）",
            Self::English => "Master window of docked cluster (moving, minimizing, or closing affects all members)",
            Self::French => "Fenêtre principale du groupe ancré (déplacer, réduire ou fermer affecte tous les membres)",
            Self::German => "Hauptfenster der angedockten Gruppe (Verschieben, Minimieren oder Schließen betrifft alle)",
            Self::Italian => "Finestra principale del gruppo agganciato (spostamento, riduzione o chiusura influisce su tutti)",
            Self::Japanese => "ドッキンググループのマスターウィンドウ（移動・最小化・終了時に全メンバー連動）",
            Self::Korean => "도킹 그룹의 마스터 창 (이동, 최소화, 닫기 시 전체 멤버 연동)",
            Self::PortugueseBrazil => "Janela principal do grupo fixado (mover, minimizar ou fechar afeta todos)",
            Self::PortuguesePortugal => "Janela principal do grupo fixado (mover, minimizar ou fechar afeta todos)",
            Self::Indonesian => "Jendela utama dari grup yang terpasang (memindahkan, memperkecil, atau menutup memengaruhi semua anggota)",
            Self::Vietnamese => "Cửa sổ chính của nhóm liên kết (di chuyển, thu nhỏ hoặc đóng sẽ ảnh hưởng đến tất cả các thành viên)",
            Self::Russian => "Главное окно группы окон (перемещение, сворачивание или закрытие влияет на всю группу)",
            Self::Spanish => "Ventana principal del grupo acoplado (mover, minimizar o cerrar afecta a todos)",
        }
    }

    pub fn badge_cluster_slave(&self) -> &'static str {
        "📎"
    }

    pub fn tooltip_cluster_slave(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "磁吸附屬視窗（跟隨主控視窗移動；關閉僅脫離並關閉自身）",
            Self::SimplifiedChinese => "磁吸附属窗口（跟随主控窗口移动；关闭仅脱离并关闭自身）",
            Self::English => "Slave window of docked cluster (follows master; closing only closes itself)",
            Self::French => "Fenêtre rattachée (suit la principale ; fermer ferme uniquement celle-ci)",
            Self::German => "Verbundenes Fenster (folgt dem Hauptfenster; Schließen schließt nur dieses)",
            Self::Italian => "Finestra agganciata (segue la principale; la chiusura chiude solo questa)",
            Self::Japanese => "ドッキングサブウィンドウ（マスターに追従；閉じると本ウィンドウのみ終了）",
            Self::Korean => "도킹 보조 창 (마스터를 따름; 닫을 때 자신만 닫힘)",
            Self::PortugueseBrazil => "Janela vinculada (segue a principal; fechar fecha apenas a si mesma)",
            Self::PortuguesePortugal => "Janela vinculada (segue a principal; fechar fecha apenas a si mesma)",
            Self::Indonesian => "Jendela terikat (mengikuti jendela utama; menutup hanya menutup jendela ini)",
            Self::Vietnamese => "Cửa sổ phụ thuộc (đi theo cửa sổ chính; đóng chỉ đóng cửa sổ này)",
            Self::Russian => "Дочернее окно (следует за главным; закрытие закрывает только его)",
            Self::Spanish => "Ventana vinculada (sigue a la principal; cerrar solo cierra esta ventana)",
        }
    }

    pub fn tooltip_minimize_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "最小化視窗",
            Self::SimplifiedChinese => "最小化窗口",
            Self::English => "Minimize window",
            Self::French => "Réduire la fenêtre",
            Self::German => "Fenster minimieren",
            Self::Italian => "Riduci a icona la finestra",
            Self::Japanese => "ウィンドウを最小化",
            Self::Korean => "창 최소화",
            Self::PortugueseBrazil => "Minimizar janela",
            Self::PortuguesePortugal => "Minimizar janela",
            Self::Indonesian => "Perkecil jendela",
            Self::Vietnamese => "Thu nhỏ cửa sổ",
            Self::Russian => "Свернуть окно",
            Self::Spanish => "Minimizar ventana",
        }
    }

    pub fn tooltip_minimize_cluster(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "最小化模組化工作站全體群組 (Shift+點擊僅最小化此視窗)",
            Self::SimplifiedChinese => "最小化模块化工作站全体群组 (Shift+点击仅最小化此窗口)",
            Self::English => "Minimize workstation cluster (Shift+Click to minimize solo)",
            Self::French => "Réduire tout le groupe de la station (Maj+Clic pour réduire seul)",
            Self::German => "Gesamte Workstation-Gruppe minimieren (Umschalt+Klick für Einzelfenster)",
            Self::Italian => "Riduci l'intero gruppo workstation (Shift+Clic per ridurre singola)",
            Self::Japanese => "モジュラーワークステーション全体を最小化 (Shift+クリックで単独最小化)",
            Self::Korean => "워크스테이션 전체 그룹 최소화 (Shift+클릭 시 단독 최소화)",
            Self::PortugueseBrazil => "Minimizar todo o grupo da estação (Shift+clique para janela individual)",
            Self::PortuguesePortugal => "Minimizar todo o grupo da estação (Shift+clique para janela individual)",
            Self::Indonesian => "Perkecil seluruh grup workstation (Shift+Klik untuk jendela tunggal)",
            Self::Vietnamese => "Thu nhỏ toàn bộ nhóm không gian làm việc (Shift+Nhấp cho cửa sổ đơn)",
            Self::Russian => "Свернуть всю группу рабочей станции (Shift+клик для одного окна)",
            Self::Spanish => "Minimizar todo el grupo de la estación (Shift+clic para ventana individual)",
        }
    }

    pub fn tooltip_minimize_cluster_all(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "僅最小化此視窗 (自工作站群組脫離)",
            Self::SimplifiedChinese => "仅最小化此窗口 (自工作站群组脱离)",
            Self::English => "Minimize this window only (detach from cluster)",
            Self::French => "Réduire uniquement cette fenêtre (détacher du groupe)",
            Self::German => "Nur dieses Fenster minimieren (aus Gruppe lösen)",
            Self::Italian => "Riduci solo questa finestra (separa dal gruppo)",
            Self::Japanese => "このウィンドウのみ最小化 (グループから分離)",
            Self::Korean => "이 창만 최소화 (그룹에서 분리)",
            Self::PortugueseBrazil => "Minimizar apenas esta janela (desacoplar do grupo)",
            Self::PortuguesePortugal => "Minimizar apenas esta janela (desacoplar do grupo)",
            Self::Indonesian => "Hanya perkecil jendela ini (lepas dari grup)",
            Self::Vietnamese => "Chỉ thu nhỏ cửa sổ này (tách khỏi nhóm)",
            Self::Russian => "Свернуть только это окно (отсоединить от группы)",
            Self::Spanish => "Minimizar solo esta ventana (separar del grupo)",
        }
    }

    pub fn tooltip_close_cluster(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關閉模組化工作站全體群組 (Shift+點擊僅關閉此視窗)",
            Self::SimplifiedChinese => "关闭模块化工作站全体群组 (Shift+点击仅关闭此窗口)",
            Self::English => "Close workstation cluster (Shift+Click to close solo)",
            Self::French => "Fermer tout le groupe de la station (Maj+Clic pour fermer seul)",
            Self::German => "Gesamte Workstation-Gruppe schließen (Umschalt+Klick für Einzelfenster)",
            Self::Italian => "Chiudi l'intero gruppo workstation (Shift+Clic per chiudere singola)",
            Self::Japanese => "モジュラーワークステーション全体を閉じる (Shift+クリックで単独閉じる)",
            Self::Korean => "워크스테이션 전체 그룹 닫기 (Shift+클릭 시 단독 닫기)",
            Self::PortugueseBrazil => "Fechar todo o grupo da estação (Shift+clique para janela individual)",
            Self::PortuguesePortugal => "Fechar todo o grupo da estação (Shift+clique para janela individual)",
            Self::Indonesian => "Tutup seluruh grup workstation (Shift+Klik untuk jendela tunggal)",
            Self::Vietnamese => "Đóng toàn bộ nhóm không gian làm việc (Shift+Nhấp cho cửa sổ đơn)",
            Self::Russian => "Закрыть всю группу рабочей станции (Shift+клик для одного окна)",
            Self::Spanish => "Cerrar todo el grupo de la estación (Shift+clic para ventana individual)",
        }
    }

    pub fn tooltip_close_cluster_all(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "僅關閉此視窗 (自工作站群組脫離)",
            Self::SimplifiedChinese => "仅关闭此窗口 (自工作站群组脱离)",
            Self::English => "Close this window only (detach from cluster)",
            Self::French => "Fermer uniquement cette fenêtre (détacher du groupe)",
            Self::German => "Nur dieses Fenster schließen (aus Gruppe lösen)",
            Self::Italian => "Chiudi solo questa finestra (separa dal gruppo)",
            Self::Japanese => "このウィンドウのみ閉じる (グループから分離)",
            Self::Korean => "이 창만 닫기 (그룹에서 분리)",
            Self::PortugueseBrazil => "Fechar apenas esta janela (desacoplar do grupo)",
            Self::PortuguesePortugal => "Fechar apenas esta janela (desacoplar do grupo)",
            Self::Indonesian => "Hanya tutup jendela ini (lepas dari grup)",
            Self::Vietnamese => "Chỉ đóng cửa sổ này (tách khỏi nhóm)",
            Self::Russian => "Закрыть только это окно (отсоединить от группы)",
            Self::Spanish => "Cerrar solo esta ventana (separar del grupo)",
        }
    }

    pub fn tooltip_modular_workstation(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "呈現三欄模組化工作站（自動開啟並齊平磁吸：主視窗、迷你播放器、等化器與視覺效果）",
            Self::SimplifiedChinese => "呈现三栏模块化工作站（自动开启并齐平磁吸：主窗口、迷你播放器、均衡器与视觉效果）",
            Self::English => "3-Column Modular Workstation (open and snap: Main, MiniPlayer, Equalizer & Visualizer)",
            Self::French => "Station modulaire à 3 colonnes (ouvrir et ancrer : Principal, MiniPlayer, Égaliseur et Visualiseur)",
            Self::German => "3-Spalten-Workstation (Öffnen und Andocken: Hauptfenster, MiniPlayer, Equalizer & Visualizer)",
            Self::Italian => "Workstation modulare a 3 colonne (apri e aggancia: Principale, MiniPlayer, Equalizzatore e Visualizzatore)",
            Self::Japanese => "3列モジュラーワークステーション（メイン、ミニプレイヤー、イコライザー、ビジュアライザーを一括整列）",
            Self::Korean => "3열 모듈형 워크스테이션 (메인 창, 미니 플레이어, 이퀄라이저 및 시각화 창 일괄 정렬)",
            Self::PortugueseBrazil => "Estação de trabalho modular em 3 colunas (abrir e fixar: Principal, MiniPlayer, Equalizador e Visualizador)",
            Self::PortuguesePortugal => "Estação de trabalho modular em 3 colunas (abrir e fixar: Principal, MiniPlayer, Equalizador e Visualizador)",
            Self::Indonesian => "Workstation Modular 3 Kolom (buka dan pasang: Utama, MiniPlayer, Equalizer & Visualizer)",
            Self::Vietnamese => "Trạm làm việc mô-đun 3 cột (mở và gắn: Cửa sổ chính, MiniPlayer, Bộ chỉnh âm & Trình hiển thị)",
            Self::Russian => "3-колоночная модульная станция (открыть и прикрепить: Главное окно, MiniPlayer, Эквалайзер и Визуализатор)",
            Self::Spanish => "Estación de trabajo modular de 3 columnas (abrir y acoplar: Principal, MiniPlayer, Ecualizador y Visualizador)",
        }
    }

    pub fn onboarding_workstation_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 推薦探索：模組化工作站",
            Self::SimplifiedChinese => "💡 推荐探索：模块化工作站",
            Self::English => "💡 Discover Modular Workstation",
            Self::French => "💡 Découvrir la station modulaire",
            Self::German => "💡 Entdecken Sie die modulare Workstation",
            Self::Italian => "💡 Scopri la workstation modulare",
            Self::Japanese => "💡 おすすめ機能：モジュラーワークステーション",
            Self::Korean => "💡 추천 기능: 모듈형 워크스테이션",
            Self::PortugueseBrazil | Self::PortuguesePortugal => "💡 Descubra a estação de trabalho modular",
            Self::Indonesian => "💡 Temukan Workstation Modular",
            Self::Vietnamese => "💡 Khám phá Trạm làm việc mô-đun",
            Self::Russian => "💡 Рекомендуем: Модульная станция",
            Self::Spanish => "💡 Descubre la estación de trabajo modular",
        }
    }

    pub fn onboarding_workstation_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "一鍵展開並自動磁吸：主控台 + 迷你播放器 + 等化器 + 音訊視覺特效。",
            Self::SimplifiedChinese => "一键展开并自动磁吸：主控台 + 迷你播放器 + 均衡器 + 音频视觉特效。",
            Self::English => "One-click open & snap: Main + MiniPlayer + Equalizer + Visualizer.",
            Self::French => "Un clic pour ouvrir et ancrer : Principal + MiniPlayer + Égaliseur + Visualiseur.",
            Self::German => "Mit einem Klick öffnen & andocken: Hauptfenster + MiniPlayer + Equalizer + Visualizer.",
            Self::Italian => "Un clic per aprire e agganciare: Principale + MiniPlayer + Equalizzatore + Visualizzatore.",
            Self::Japanese => "ワンクリックで一括展開・磁気吸着：メイン + ミニプレイヤー + イコライザー + 視覚効果。",
            Self::Korean => "원클릭으로 일괄 열기 및 자석 정렬: 메인 + 미니 플레이어 + 이퀄라이저 + 시각 효과.",
            Self::PortugueseBrazil | Self::PortuguesePortugal => "Um clique para abrir e fixar: Principal + MiniPlayer + Equalizador + Visualizador.",
            Self::Indonesian => "Buka & pasang sekali klik: Utama + MiniPlayer + Equalizer + Visualizer.",
            Self::Vietnamese => "Mở và gắn chỉ bằng một cú nhấp: Cửa sổ chính + MiniPlayer + Bộ chỉnh âm + Trình hiển thị.",
            Self::Russian => "Один клик для открытия и стыковки: Главное окно + MiniPlayer + Эквалайзер + Визуализатор.",
            Self::Spanish => "Un clic para abrir y acoplar: Principal + MiniPlayer + Ecualizador + Visualizador.",
        }
    }

    pub fn onboarding_workstation_btn_try(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "◫ 立即展開",
            Self::SimplifiedChinese => "◫ 立即展开",
            Self::English => "◫ Open Now",
            Self::French => "◫ Ouvrir",
            Self::German => "◫ Jetzt öffnen",
            Self::Italian => "◫ Apri ora",
            Self::Japanese => "◫ 今すぐ展開",
            Self::Korean => "◫ 지금 열기",
            Self::PortugueseBrazil | Self::PortuguesePortugal => "◫ Abrir agora",
            Self::Indonesian => "◫ Buka Sekarang",
            Self::Vietnamese => "◫ Mở ngay",
            Self::Russian => "◫ Открыть сейчас",
            Self::Spanish => "◫ Abrir ahora",
        }
    }

    pub fn onboarding_workstation_btn_dismiss(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "✕ 稍後",
            Self::SimplifiedChinese => "✕ 稍后",
            Self::English => "✕ Later",
            Self::French => "✕ Plus tard",
            Self::German => "✕ Später",
            Self::Italian => "✕ Più tardi",
            Self::Japanese => "✕ あとで",
            Self::Korean => "✕ 나중에",
            Self::PortugueseBrazil | Self::PortuguesePortugal => "✕ Mais tarde",
            Self::Indonesian => "✕ Nanti",
            Self::Vietnamese => "✕ Để sau",
            Self::Russian => "✕ Позже",
            Self::Spanish => "✕ Más tarde",
        }
    }

    pub fn tooltip_settings(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟進階設定",
            Self::SimplifiedChinese => "打开高级设置",
            Self::English => "Open Advanced Settings",
            Self::French => "Ouvrir les paramètres avancés",
            Self::German => "Erweiterte Einstellungen öffnen",
            Self::Italian => "Apri impostazioni avanzate",
            Self::Japanese => "詳細設定を開く",
            Self::Korean => "고급 설정 열기",
            Self::PortugueseBrazil => "Abrir configurações avançadas",
            Self::PortuguesePortugal => "Abrir definições avançadas",
            Self::Indonesian => "Buka Pengaturan Lanjutan",
            Self::Vietnamese => "Mở cài đặt nâng cao",
            Self::Russian => "Открыть расширенные настройки",
            Self::Spanish => "Abrir configuración avanzada",
        }
    }

    pub fn settings_section_audio(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊來源與輸出",
            Self::SimplifiedChinese => "音频来源与输出",
            Self::English => "Audio Source & Output",
            Self::French => "Source audio et sortie",
            Self::German => "Audioquelle & Ausgabe",
            Self::Italian => "Sorgente audio e uscita",
            Self::Japanese => "音声ソースと出力",
            Self::Korean => "오디오 소스 및 출력",
            Self::PortugueseBrazil => "Fonte de áudio e saída",
            Self::PortuguesePortugal => "Fonte de áudio e saída",
            Self::Indonesian => "Sumber & Output Audio",
            Self::Vietnamese => "Nguồn & Đầu ra âm thanh",
            Self::Russian => "Источник звука и вывод",
            Self::Spanish => "Fuente de audio y salida",
        }
    }

    pub fn settings_section_streaming(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "串流與電源",
            Self::SimplifiedChinese => "串流与电源",
            Self::English => "Streaming & Power",
            Self::French => "Diffusion et alimentation",
            Self::German => "Streaming & Energie",
            Self::Italian => "Streaming e alimentazione",
            Self::Japanese => "ストリーミングと電源",
            Self::Korean => "스트리밍 및 전원",
            Self::PortugueseBrazil => "Transmissão e energia",
            Self::PortuguesePortugal => "Transmissão e energia",
            Self::Indonesian => "Streaming & Daya",
            Self::Vietnamese => "Truyền phát & Nguồn điện",
            Self::Russian => "Трансляция и питание",
            Self::Spanish => "Transmisión y energía",
        }
    }

    pub fn settings_section_general(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "系統與快速鍵",
            Self::SimplifiedChinese => "系统与快捷键",
            Self::English => "System & Shortcuts",
            Self::French => "Système et raccourcis",
            Self::German => "System & Tastenkürzel",
            Self::Italian => "Sistema e scorciatoie",
            Self::Japanese => "システムとショートカット",
            Self::Korean => "시스템 및 단축키",
            Self::PortugueseBrazil => "Sistema e atalhos",
            Self::PortuguesePortugal => "Sistema e atalhos",
            Self::Indonesian => "Sistem & Pintasan",
            Self::Vietnamese => "Hệ thống & Phím tắt",
            Self::Russian => "Система и горячие клавиши",
            Self::Spanish => "Sistema y atajos",
        }
    }

    pub fn top_hint_minimize(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "✕ 關閉視窗會最小化至工作列常駐",
            Self::SimplifiedChinese => "✕ 关闭窗口将最小化到托盘常驻",
            Self::English => "✕ Closing minimizes to system tray",
            Self::French => "✕ Fermer minimise dans la zone de notification",
            Self::German => "✕ Schließen minimiert in den Infobereich",
            Self::Italian => "✕ La chiusura riduce a icona nella barra",
            Self::Japanese => "✕ 閉じるとタスクトレイに常駐します",
            Self::Korean => "✕ 창을 닫으면 시스템 트레이로 최소화됩니다",
            Self::PortugueseBrazil => "✕ Fechar minimiza para a bandeja do sistema",
            Self::PortuguesePortugal => "✕ Fechar minimiza para a área de notificação",
            Self::Indonesian => "✕ Menutup akan meminimalkan ke baki sistem",
            Self::Vietnamese => "✕ Đóng sẽ thu nhỏ xuống khay hệ thống",
            Self::Russian => "✕ Закрытие сворачивает в системный трей",
            Self::Spanish => "✕ Al cerrar se minimiza a la bandeja del sistema",
        }
    }

    pub fn btn_quit_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⏻ 結束程式",
            Self::SimplifiedChinese => "⏻ 退出程序",
            Self::English => "⏻ Exit App",
            Self::French => "⏻ Quitter",
            Self::German => "⏻ Beenden",
            Self::Italian => "⏻ Esci",
            Self::Japanese => "⏻ 終了",
            Self::Korean => "⏻ 앱 종료",
            Self::PortugueseBrazil => "⏻ Sair do app",
            Self::PortuguesePortugal => "⏻ Sair da aplicação",
            Self::Indonesian => "⏻ Keluar dari Aplikasi",
            Self::Vietnamese => "⏻ Thoát ứng dụng",
            Self::Russian => "⏻ Выйти",
            Self::Spanish => "⏻ Salir de la app",
        }
    }

    pub fn btn_quitting_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⏻ 正在結束...",
            Self::SimplifiedChinese => "⏻ 正在退出...",
            Self::English => "⏻ Quitting...",
            Self::French => "⏻ Fermeture...",
            Self::German => "⏻ Wird beendet...",
            Self::Italian => "⏻ Chiusura...",
            Self::Japanese => "⏻ 終了中...",
            Self::Korean => "⏻ 종료 중...",
            Self::PortugueseBrazil => "⏻ Encerrando...",
            Self::PortuguesePortugal => "⏻ A encerrar...",
            Self::Indonesian => "⏻ Sedang keluar...",
            Self::Vietnamese => "⏻ Đang thoát...",
            Self::Russian => "⏻ Завершение...",
            Self::Spanish => "⏻ Saliendo...",
        }
    }

    pub fn tooltip_refresh(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "重新搜尋區網內 AirPlay 裝置",
            Self::SimplifiedChinese => "重新搜索局域网内 AirPlay 设备",
            Self::English => "Rescan for AirPlay devices on the local network",
            Self::French => "Rechercher les appareils AirPlay sur le réseau local",
            Self::German => "Nach AirPlay-Geräten im lokalen Netzwerk suchen",
            Self::Italian => "Cerca dispositivi AirPlay nella rete locale",
            Self::Japanese => "ローカルネットワーク内のAirPlayデバイスを再検索",
            Self::Korean => "로컬 네트워크의 AirPlay 기기 다시 검색",
            Self::PortugueseBrazil => "Buscar dispositivos AirPlay na rede local",
            Self::PortuguesePortugal => "Procurar dispositivos AirPlay na rede local",
            Self::Indonesian => "Pindai ulang perangkat AirPlay di jaringan lokal",
            Self::Vietnamese => "Quét lại các thiết bị AirPlay trên mạng cục bộ",
            Self::Russian => "Поиск устройств AirPlay в локальной сети",
            Self::Spanish => "Buscar dispositivos AirPlay en la red local",
        }
    }

    pub fn tooltip_auto_preamp(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🛡 自動前級衰減補償：根據當前 EQ 最高提升頻段自動調整 Headroom，徹底防止音波破音與數位裁切失真。",
            Self::SimplifiedChinese => "🛡 自动前级衰减补偿：根据当前 EQ 最高提升频段自动调整 Headroom，彻底防止音波破音与数字削波失真。",
            Self::English => "🛡 Auto Pre-Amp Headroom: Automatically compensates for the highest EQ boost band to prevent clipping distortion.",
            Self::French => "🛡 Pré-ampli automatique : compense le gain maximal de l'égaliseur pour éviter la distorsion numérique.",
            Self::German => "🛡 Auto-Vorverstärkung: Gleicht das maximale EQ-Boost-Band automatisch aus, um Clipping-Verzerrungen zu verhindern.",
            Self::Italian => "🛡 Pre-amp automatico: compensa automaticamente il guadagno massimo dell'equalizzatore per evitare distorsioni.",
            Self::Japanese => "🛡 自動プリアンプ減衰補正：現在のEQ最大ブースト量に合わせてヘッドルームを自動調整し、音割れやデジタルクリッピング歪みを完全に防止します。",
            Self::Korean => "🛡 자동 프리앰프 감쇄 보정: 현재 EQ 최대 부스트에 맞춰 헤드룸을 자동 확보하여 음 왜곡 및 디지털 클리핑을 방지합니다.",
            Self::PortugueseBrazil => "🛡 Pré-amplificador automático: compensa o ganho máximo do equalizador para evitar distorção por corte.",
            Self::PortuguesePortugal => "🛡 Pré-amplificador automático: compensa o ganho máximo do equalizador para evitar distorção por corte.",
            Self::Indonesian => "🛡 Ruang Kepala Pra-Amp Otomatis: Mengompensasi penguatan EQ tertinggi secara otomatis untuk mencegah distorsi kliping.",
            Self::Vietnamese => "🛡 Khoảng đệm tiền khuếch đại tự động: Tự động bù dải tăng EQ cao nhất để chống méo tiếng vỡ âm (clipping).",
            Self::Russian => "🛡 Авто-предусиление: автоматически компенсирует максимальное усиление эквалайзера для предотвращения клиппинга.",
            Self::Spanish => "🛡 Preamplificador automático: compensa la ganancia máxima del ecualizador para evitar la distorsión por saturación.",
        }
    }

    pub fn status_idle(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "待機中 (尚未連線)",
            Self::SimplifiedChinese => "待机中 (未连接)",
            Self::English => "Idle (Not connected)",
            Self::French => "En attente (Non connecté)",
            Self::German => "Bereit (Nicht verbunden)",
            Self::Italian => "In attesa (Non connesso)",
            Self::Japanese => "待機中 (未接続)",
            Self::Korean => "대기 중 (연결되지 않음)",
            Self::PortugueseBrazil => "Em espera (Não conectado)",
            Self::PortuguesePortugal => "Em espera (Não ligado)",
            Self::Indonesian => "Siaga (Tidak terhubung)",
            Self::Vietnamese => "Chờ (Chưa kết nối)",
            Self::Russian => "Ожидание (Не подключено)",
            Self::Spanish => "En espera (No conectado)",
        }
    }

    pub fn status_connecting(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("正在連線至 {name}..."),
            Self::SimplifiedChinese => format!("正在连接至 {name}..."),
            Self::English => format!("Connecting to {name}..."),
            Self::French => format!("Connexion à {name}..."),
            Self::German => format!("Verbindung zu {name} wird hergestellt..."),
            Self::Italian => format!("Connessione a {name}..."),
            Self::Japanese => format!("{name} に接続中..."),
            Self::Korean => format!("{name}에 연결 중..."),
            Self::PortugueseBrazil => format!("Conectando a {name}..."),
            Self::PortuguesePortugal => format!("A ligar a {name}..."),
            Self::Indonesian => format!("Menghubungkan ke {name}..."),
            Self::Vietnamese => format!("Đang kết nối tới {name}..."),
            Self::Russian => format!("Подключение к {name}..."),
            Self::Spanish => format!("Conectando a {name}..."),
        }
    }

    pub fn status_streaming(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("現正串流：{name}"),
            Self::SimplifiedChinese => format!("正在串流：{name}"),
            Self::English => format!("Streaming: {name}"),
            Self::French => format!("Lecture en cours : {name}"),
            Self::German => format!("Streaming: {name}"),
            Self::Italian => format!("In riproduzione: {name}"),
            Self::Japanese => format!("ストリーミング中：{name}"),
            Self::Korean => format!("스트리밍 중: {name}"),
            Self::PortugueseBrazil => format!("Transmitindo: {name}"),
            Self::PortuguesePortugal => format!("A transmitir: {name}"),
            Self::Indonesian => format!("Streaming: {name}"),
            Self::Vietnamese => format!("Đang truyền phát: {name}"),
            Self::Russian => format!("Трансляция: {name}"),
            Self::Spanish => format!("Transmitiendo: {name}"),
        }
    }

    pub fn friendly_model(&self, model: &str) -> &'static str {
        match model {
            "AudioAccessory1,1" | "AudioAccessory1,2" => match self {
                Self::TraditionalChinese => "HomePod (第 1 代)",
                Self::SimplifiedChinese => "HomePod (第 1 代)",
                Self::English => "HomePod (1st gen)",
                Self::French => "HomePod (1re gén.)",
                Self::German => "HomePod (1. Gen.)",
                Self::Italian => "HomePod (1ª gen.)",
                Self::Japanese => "HomePod (第1世代)",
                Self::Korean => "HomePod (1세대)",
                Self::PortugueseBrazil => "HomePod (1ª ger.)",
                Self::PortuguesePortugal => "HomePod (1ª ger.)",
                Self::Indonesian => "HomePod (Gen ke-1)",
                Self::Vietnamese => "HomePod (Thế hệ 1)",
                Self::Russian => "HomePod (1-го пок.)",
                Self::Spanish => "HomePod (1.ª gen.)",
            },
            "AudioAccessory5,1" => "HomePod mini",
            "AudioAccessory6,1" => match self {
                Self::TraditionalChinese => "HomePod (第 2 代)",
                Self::SimplifiedChinese => "HomePod (第 2 代)",
                Self::English => "HomePod (2nd gen)",
                Self::French => "HomePod (2e gén.)",
                Self::German => "HomePod (2. Gen.)",
                Self::Italian => "HomePod (2ª gen.)",
                Self::Japanese => "HomePod (第2世代)",
                Self::Korean => "HomePod (2세대)",
                Self::PortugueseBrazil => "HomePod (2ª ger.)",
                Self::PortuguesePortugal => "HomePod (2ª ger.)",
                Self::Indonesian => "HomePod (Gen ke-2)",
                Self::Vietnamese => "HomePod (Thế hệ 2)",
                Self::Russian => "HomePod (2-го пок.)",
                Self::Spanish => "HomePod (2.ª gen.)",
            },
            m if m.starts_with("AppleTV") => "Apple TV",
            m if m.starts_with("Mac") => "Mac",
            _ => "",
        }
    }

    pub fn stereo_pair_model_desc(&self, friendly_model: &str, count: usize) -> String {
        match self {
            Self::TraditionalChinese => format!("立體聲配對 ({count} 部 • {friendly_model})"),
            Self::SimplifiedChinese => format!("立体声组合 ({count} 台 • {friendly_model})"),
            Self::English => format!("Stereo Pair ({count} units • {friendly_model})"),
            Self::French => format!("Paire stéréo ({count} appareils • {friendly_model})"),
            Self::German => format!("Stereopaar ({count} Geräte • {friendly_model})"),
            Self::Italian => format!("Coppia stereo ({count} unità • {friendly_model})"),
            Self::Japanese => format!("ステレオペア ({count} 台 • {friendly_model})"),
            Self::Korean => format!("스테레오 페어 ({count}대 • {friendly_model})"),
            Self::PortugueseBrazil => format!("Par estéreo ({count} aparelhos • {friendly_model})"),
            Self::PortuguesePortugal => format!("Par estéreo ({count} aparelhos • {friendly_model})"),
            Self::Indonesian => format!("Pasangan Stereo ({count} unit • {friendly_model})"),
            Self::Vietnamese => format!("Cặp âm thanh nổi ({count} thiết bị • {friendly_model})"),
            Self::Russian => format!("Стереопара ({count} устр. • {friendly_model})"),
            Self::Spanish => format!("Par estéreo ({count} unidades • {friendly_model})"),
        }
    }

    pub fn status_error(&self, err: &str) -> String {
        let trimmed = err.trim();
        if trimmed.contains("470") || trimmed.contains("Authorization Required") {
            return match self {
                Self::TraditionalChinese => "連線失敗：470 未授權（請至 Apple TV「設定 > AirPlay 與 HomeKit > 允許取用」設為「所有人」，並關閉密碼要求）".to_string(),
                Self::SimplifiedChinese => "连接失败：470 未授权（请至 Apple TV「设置 > 隔空播放与家庭 > 允许访问」设为「所有人」，并关闭密码要求）".to_string(),
                Self::English => "Connection failed: 470 Authorization Required (Please set Apple TV \"Settings > AirPlay & HomeKit > Allow Access\" to \"Everyone\" and turn off password)".to_string(),
                Self::French => "Échec de connexion : 470 Autorisation requise (Veuillez régler sur l'Apple TV « Réglages > AirPlay et HomeKit > Autoriser l'accès » sur « Tout le monde » et désactiver le mot de passe)".to_string(),
                Self::German => "Verbindung fehlgeschlagen: 470 Autorisierung erforderlich (Bitte auf Apple TV „Einstellungen > AirPlay und HomeKit > Zugriff erlauben“ auf „Jeder“ setzen und Passwort deaktivieren)".to_string(),
                Self::Italian => "Connessione fallita: 470 Autorizzazione richiesta (Impostare su Apple TV \"Impostazioni > AirPlay e HomeKit > Consenti accesso\" su \"Tutti\" e disattivare la password)".to_string(),
                Self::Japanese => "接続失敗：470 未認証（Apple TVの「設定 > AirPlayとHomeKit > アクセスを許可」を「全員」に設定し、パスワードを無効にしてください）".to_string(),
                Self::Korean => "연결 실패: 470 인증 필요 (Apple TV \"설정 > AirPlay 및 HomeKit > 접근 허용\"을 \"모든 사람\"으로 설정하고 암호 요구를 끄십시오)".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: 470 Autorização necessária (Defina na Apple TV \"Ajustes > AirPlay e HomeKit > Permitir Acesso\" como \"Todos\" e desative a senha)".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: 470 Autorização necessária (Defina na Apple TV \"Definições > AirPlay e HomeKit > Permitir Acesso\" como \"Todos\" e desative a palavra-passe)".to_string(),
                Self::Indonesian => "Koneksi gagal: 470 Otorisasi Diperlukan (Harap atur Apple TV \"Pengaturan > AirPlay & HomeKit > Izinkan Akses\" ke \"Semua Orang\" dan matikan kata sandi)".to_string(),
                Self::Vietnamese => "Kết nối thất bại: 470 Yêu cầu cấp quyền (Vui lòng đặt Apple TV \"Cài đặt > AirPlay & HomeKit > Cho phép truy cập\" thành \"Mọi người\" và tắt mật khẩu)".to_string(),
                Self::Russian => "Ошибка подключения: 470 Требуется авторизация (Установите на Apple TV «Настройки > AirPlay и HomeKit > Доступ» в значение «Все» и отключите пароль)".to_string(),
                Self::Spanish => "Error de conexión: 470 Autorización requerida (Configure en Apple TV \"Ajustes > AirPlay y HomeKit > Permitir acceso\" en \"Todos\" y desactive la contraseña)".to_string(),
            };
        }

        if trimmed.contains("403") || trimmed.contains("Forbidden") {
            return match self {
                Self::TraditionalChinese => "連線失敗：403 拒絕存取（接收端拒絕連線，請檢查接收端的 AirPlay 允許取用權限）".to_string(),
                Self::SimplifiedChinese => "连接失败：403 拒绝访问（接收端拒绝连接，请检查接收端的 AirPlay 允许访问权限）".to_string(),
                Self::English => "Connection failed: 403 Forbidden (Receiver refused connection, please check AirPlay access permissions)".to_string(),
                Self::French => "Échec de connexion : 403 Refusé (Le récepteur a refusé la connexion, vérifiez les autorisations AirPlay)".to_string(),
                Self::German => "Verbindung fehlgeschlagen: 403 Verboten (Empfänger hat die Verbindung abgelehnt, bitte AirPlay-Zugriffsberechtigungen prüfen)".to_string(),
                Self::Italian => "Connessione fallita: 403 Vietato (Il ricevitore ha rifiutato la connessione, verificare le autorizzazioni AirPlay)".to_string(),
                Self::Japanese => "接続失敗：403 アクセス拒否（受信機に接続を拒否されました。AirPlayアクセス許可設定を確認してください）".to_string(),
                Self::Korean => "연결 실패: 403 거부됨 (수신 기기가 연결을 거부했습니다. AirPlay 접근 권한 설정을 확인하십시오)".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: 403 Proibido (O receptor recusou a conexão, verifique as permissões do AirPlay)".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: 403 Proibido (O recetor recusou a ligação, verifique as permissões do AirPlay)".to_string(),
                Self::Indonesian => "Koneksi gagal: 403 Dilarang (Penerima menolak koneksi, periksa izin akses AirPlay)".to_string(),
                Self::Vietnamese => "Kết nối thất bại: 403 Bị cấm (Thiết bị nhận đã từ chối kết nối, vui lòng kiểm tra quyền truy cập AirPlay)".to_string(),
                Self::Russian => "Ошибка подключения: 403 Запрещено (Приемник отклонил подключение, проверьте разрешения AirPlay)".to_string(),
                Self::Spanish => "Error de conexión: 403 Prohibido (El receptor rechazó la conexión, verifique los permisos de AirPlay)".to_string(),
            };
        }

        if trimmed.contains("401") || trimmed.contains("Unauthorized") {
            return match self {
                Self::TraditionalChinese => "連線失敗：401 需密碼（接收端需要 AirPlay 存取密碼）".to_string(),
                Self::SimplifiedChinese => "连接失败：401 需密码（接收端需要 AirPlay 访问密码）".to_string(),
                Self::English => "Connection failed: 401 Unauthorized (Receiver requires an AirPlay password)".to_string(),
                Self::French => "Échec de connexion : 401 Non autorisé (Le récepteur nécessite un mot de passe AirPlay)".to_string(),
                Self::German => "Verbindung fehlgeschlagen: 401 Nicht autorisiert (Empfänger erfordert ein AirPlay-Passwort)".to_string(),
                Self::Italian => "Connessione fallita: 401 Non autorizzato (Il ricevitore richiede una password AirPlay)".to_string(),
                Self::Japanese => "接続失敗：401 認証が必要（受信機にAirPlayパスワードが必要です）".to_string(),
                Self::Korean => "연결 실패: 401 인증 필요 (수신 기기에 AirPlay 암호가 필요합니다)".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: 401 Não autorizado (O receptor requer uma senha do AirPlay)".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: 401 Não autorizado (O recetor requer uma palavra-passe do AirPlay)".to_string(),
                Self::Indonesian => "Koneksi gagal: 401 Tidak Diotorisasi (Penerima memerlukan kata sandi AirPlay)".to_string(),
                Self::Vietnamese => "Kết nối thất bại: 401 Chưa xác thực (Thiết bị nhận yêu cầu mật khẩu AirPlay)".to_string(),
                Self::Russian => "Ошибка подключения: 401 Не авторизован (Приемник требует пароль AirPlay)".to_string(),
                Self::Spanish => "Error de conexión: 401 No autorizado (El receptor requiere una contraseña de AirPlay)".to_string(),
            };
        }

        if trimmed.contains("ConnectionRefused") || trimmed.contains("Connection refused") || trimmed.contains("refused") {
            return match self {
                Self::TraditionalChinese => "連線失敗：連線被拒絕（無法連線至目標裝置，請確認裝置已開機並在相同區域網路）".to_string(),
                Self::SimplifiedChinese => "连接失败：连接被拒绝（无法连接至目标设备，请确认设备已开机并在相同局域网）".to_string(),
                Self::English => "Connection failed: Connection refused (Cannot connect to target device, please verify power and Wi-Fi)".to_string(),
                Self::French => "Échec de connexion : Connexion refusée (Impossible de se connecter à l'appareil, vérifiez l'alimentation et le Wi-Fi)".to_string(),
                Self::German => "Verbindung fehlgeschlagen: Verbindung abgelehnt (Zielgerät nicht erreichbar, bitte Strom und WLAN prüfen)".to_string(),
                Self::Italian => "Connessione fallita: Connessione rifiutata (Impossibile connettersi al dispositivo, verificare alimentazione e Wi-Fi)".to_string(),
                Self::Japanese => "接続失敗：接続拒否（ターゲット機器に接続できません。電源とWi-Fi接続を確認してください）".to_string(),
                Self::Korean => "연결 실패: 연결 거부 (대상 기기에 연결할 수 없습니다. 전원 및 Wi-Fi 연결을 확인하십시오)".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: Conexão recusada (Não foi possível conectar ao dispositivo, verifique energia e Wi-Fi)".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: Ligação recusada (Não foi possível ligar ao dispositivo, verifique energia e Wi-Fi)".to_string(),
                Self::Indonesian => "Koneksi gagal: Koneksi ditolak (Tidak dapat terhubung ke perangkat target, periksa daya dan Wi-Fi)".to_string(),
                Self::Vietnamese => "Kết nối thất bại: Kết nối bị từ chối (Không thể kết nối đến thiết bị đích, vui lòng kiểm tra nguồn điện và Wi-Fi)".to_string(),
                Self::Russian => "Ошибка подключения: В соединении отказано (Не удалось подключиться к устройству, проверьте питание и Wi-Fi)".to_string(),
                Self::Spanish => "Error de conexión: Conexión rechazada (No se pudo conectar al dispositivo objetivo, compruebe alimentación y Wi-Fi)".to_string(),
            };
        }

        if trimmed.contains("Timeout") || trimmed.contains("TimedOut") || trimmed.contains("timed out") {
            return match self {
                Self::TraditionalChinese => "連線失敗：連線逾時（目標裝置沒有回應，請檢查網路連線品質）".to_string(),
                Self::SimplifiedChinese => "连接失败：连接超时（目标设备没有响应，请检查网络连接质量）".to_string(),
                Self::English => "Connection failed: Connection timed out (Target device did not respond, please check network quality)".to_string(),
                Self::French => "Échec de connexion : Délai d'attente dépassé (L'appareil ne répond pas, vérifiez le réseau)".to_string(),
                Self::German => "Verbindung fehlgeschlagen: Zeitüberschreitung (Zielgerät antwortet nicht, bitte Netzwerkverbindung prüfen)".to_string(),
                Self::Italian => "Connessione fallita: Timeout di connessione (Il dispositivo non risponde, verificare la rete)".to_string(),
                Self::Japanese => "接続失敗：接続タイムアウト（ターゲット機器から応答がありません。ネットワークを確認してください）".to_string(),
                Self::Korean => "연결 실패: 연결 시간 초과 (대상 기기가 응답하지 않습니다. 네트워크 품질을 확인하십시오)".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: Tempo limite esgotado (O dispositivo não respondeu, verifique a rede)".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: Tempo limite esgotado (O dispositivo não respondeu, verifique a qualidade da rede)".to_string(),
                Self::Indonesian => "Koneksi gagal: Waktu koneksi habis (Perangkat target tidak merespons, periksa kualitas jaringan)".to_string(),
                Self::Vietnamese => "Kết nối thất bại: Quá thời gian chờ kết nối (Thiết bị đích không phản hồi, vui lòng kiểm tra chất lượng mạng)".to_string(),
                Self::Russian => "Ошибка подключения: Время ожидания истекло (Устройство не отвечает, проверьте сеть)".to_string(),
                Self::Spanish => "Error de conexión: Tiempo de espera agotado (El dispositivo no responde, verifique la red)".to_string(),
            };
        }

        if trimmed.contains("IPv4") || trimmed.contains("找不到") || trimmed.contains("No available IPv4") {
            return match self {
                Self::TraditionalChinese => "連線失敗：找不到選取裝置的可用 IPv4 位址".to_string(),
                Self::SimplifiedChinese => "连接失败：找不到选定设备的可用 IPv4 地址".to_string(),
                Self::English => "Connection failed: No usable IPv4 address found for the selected device".to_string(),
                Self::French => "Échec de connexion : Aucune adresse IPv4 utilisable trouvée pour l'appareil sélectionné".to_string(),
                Self::German => "Verbindung fehlgeschlagen: Keine nutzbare IPv4-Adresse für das ausgewählte Gerät gefunden".to_string(),
                Self::Italian => "Connessione fallita: Nessun indirizzo IPv4 utilizzabile trovato per il dispositivo selezionato".to_string(),
                Self::Japanese => "接続失敗：選択したデバイスの有効なIPv4アドレスが見つかりませんでした".to_string(),
                Self::Korean => "연결 실패: 선택한 기기의 유효한 IPv4 주소를 찾을 수 없습니다".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: Nenhum endereço IPv4 utilizável encontrado para o dispositivo selecionado".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: Nenhum endereço IPv4 utilizável encontrado para o dispositivo selecionado".to_string(),
                Self::Indonesian => "Koneksi gagal: Tidak ada alamat IPv4 yang dapat digunakan untuk perangkat yang dipilih".to_string(),
                Self::Vietnamese => "Kết nối thất bại: Không tìm thấy địa chỉ IPv4 khả dụng cho thiết bị đã chọn".to_string(),
                Self::Russian => "Ошибка подключения: Не найден доступный адрес IPv4 для выбранного устройства".to_string(),
                Self::Spanish => "Error de conexión: No se encontró una dirección IPv4 utilizable para el dispositivo seleccionado".to_string(),
            };
        }

        if trimmed.contains("PTP") {
            return match self {
                Self::TraditionalChinese => "連線失敗：PTP 時鐘同步失敗，請重試或重新整理裝置清單".to_string(),
                Self::SimplifiedChinese => "连接失败：PTP 时钟同步失败，请重试或刷新设备列表".to_string(),
                Self::English => "Connection failed: PTP clock synchronization failed, please retry or refresh devices".to_string(),
                Self::French => "Échec de connexion : Échec de synchronisation de l'horloge PTP, veuillez réessayer".to_string(),
                Self::German => "Verbindung fehlgeschlagen: PTP-Uhrensynchronisierung fehlgeschlagen, bitte erneut versuchen".to_string(),
                Self::Italian => "Connessione fallita: Sincronizzazione dell'orologio PTP non riuscita, riprovare".to_string(),
                Self::Japanese => "接続失敗：PTPクロック同期に失敗しました。再試行またはデバイス一覧を更新してください".to_string(),
                Self::Korean => "연결 실패: PTP 시계 동기화에 실패했습니다. 다시 시도하거나 기기 목록을 새로고침하십시오".to_string(),
                Self::PortugueseBrazil => "Falha na conexão: Falha na sincronização do relógio PTP, tente novamente".to_string(),
                Self::PortuguesePortugal => "Falha na ligação: Falha na sincronização do relógio PTP, tente novamente ou atualize os dispositivos".to_string(),
                Self::Indonesian => "Koneksi gagal: Sinkronisasi jam PTP gagal, silakan coba lagi atau segarkan perangkat".to_string(),
                Self::Vietnamese => "Kết nối thất bại: Đồng bộ hóa xung nhịp PTP thất bại, vui lòng thử lại hoặc làm mới danh sách thiết bị".to_string(),
                Self::Russian => "Ошибка подключения: Сбой синхронизации часов PTP, повторите попытку".to_string(),
                Self::Spanish => "Error de conexión: Error de sincronización del reloj PTP, intente nuevamente".to_string(),
            };
        }

        match self {
            Self::TraditionalChinese => format!("連線失敗：{trimmed}"),
            Self::SimplifiedChinese => format!("连接失败：{trimmed}"),
            Self::English => format!("Connection failed: {trimmed}"),
            Self::French => format!("Échec de connexion : {trimmed}"),
            Self::German => format!("Verbindung fehlgeschlagen: {trimmed}"),
            Self::Italian => format!("Connessione fallita: {trimmed}"),
            Self::Japanese => format!("接続失敗：{trimmed}"),
            Self::Korean => format!("연결 실패: {trimmed}"),
            Self::PortugueseBrazil => format!("Falha na conexão: {trimmed}"),
            Self::PortuguesePortugal => format!("Falha na ligação: {trimmed}"),
            Self::Indonesian => format!("Koneksi gagal: {trimmed}"),
            Self::Vietnamese => format!("Kết nối thất bại: {trimmed}"),
            Self::Russian => format!("Ошибка подключения: {trimmed}"),
            Self::Spanish => format!("Error de conexión: {trimmed}"),
        }
    }

    pub fn btn_disconnect(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "中斷連線",
            Self::SimplifiedChinese => "断开连接",
            Self::English => "Disconnect",
            Self::French => "Déconnecter",
            Self::German => "Trennen",
            Self::Italian => "Disconnetti",
            Self::Japanese => "切断",
            Self::Korean => "연결 해제",
            Self::PortugueseBrazil => "Desconectar",
            Self::PortuguesePortugal => "Desligar",
            Self::Indonesian => "Putuskan",
            Self::Vietnamese => "Ngắt kết nối",
            Self::Russian => "Отключить",
            Self::Spanish => "Desconectar",
        }
    }

    pub fn preempted_banner_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音響連線已被接管",
            Self::SimplifiedChinese => "音响连接已被接管",
            Self::English => "Speaker connection taken over",
            Self::French => "Connexion de l'enceinte reprise",
            Self::German => "Lautsprecherverbindung übernommen",
            Self::Italian => "Connessione altoparlante occupata",
            Self::Japanese => "スピーカー接続が乗っ取られました",
            Self::Korean => "스피커 연결이 다른 기기에서 가져감",
            Self::PortugueseBrazil => "Conexão do alto-falante assumida",
            Self::PortuguesePortugal => "Ligação do altifalante assumida",
            Self::Indonesian => "Koneksi speaker diambil alih",
            Self::Vietnamese => "Kết nối loa đã bị chiếm quyền",
            Self::Russian => "Подключение колонки перехвачено",
            Self::Spanish => "Conexión del altavoz transferida",
        }
    }

    pub fn available_devices_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "可用的 AirPlay 音響裝置",
            Self::SimplifiedChinese => "可用的 AirPlay 音响设备",
            Self::English => "Available AirPlay Audio Devices",
            Self::French => "Appareils audio AirPlay disponibles",
            Self::German => "Verfügbare AirPlay-Audiogeräte",
            Self::Italian => "Dispositivi audio AirPlay disponibili",
            Self::Japanese => "利用可能な AirPlay オーディオ機器",
            Self::Korean => "사용 가능한 AirPlay 오디오 기기",
            Self::PortugueseBrazil => "Dispositivos de áudio AirPlay disponíveis",
            Self::PortuguesePortugal => "Dispositivos de áudio AirPlay disponíveis",
            Self::Indonesian => "Perangkat Audio AirPlay yang Tersedia",
            Self::Vietnamese => "Thiết bị âm thanh AirPlay khả dụng",
            Self::Russian => "Доступные аудиоустройства AirPlay",
            Self::Spanish => "Dispositivos de audio AirPlay disponibles",
        }
    }

    pub fn available_airpods_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "可用的 AirPods",
            Self::SimplifiedChinese => "可用的 AirPods",
            Self::English => "Available AirPods",
            Self::French => "AirPods disponibles",
            Self::German => "Verfügbare AirPods",
            Self::Italian => "AirPods disponibili",
            Self::Japanese => "利用可能な AirPods",
            Self::Korean => "사용 가능한 AirPods",
            Self::PortugueseBrazil => "AirPods disponíveis",
            Self::PortuguesePortugal => "AirPods disponíveis",
            Self::Indonesian => "AirPods yang Tersedia",
            Self::Vietnamese => "AirPods khả dụng",
            Self::Russian => "Доступные AirPods",
            Self::Spanish => "AirPods disponibles",
        }
    }

    pub fn no_airpods_found(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "未偵測到已配對的 AirPods",
            Self::SimplifiedChinese => "未检测到已配对的 AirPods",
            Self::English => "No paired AirPods detected",
            Self::French => "Aucun AirPods jumelé détecté",
            Self::German => "Keine gekoppelten AirPods erkannt",
            Self::Italian => "Nessun AirPods associato rilevato",
            Self::Japanese => "ペアリングされた AirPods が見つかりません",
            Self::Korean => "페어링된 AirPods를 찾을 수 없습니다",
            Self::PortugueseBrazil => "Nenhum AirPods emparelhado detectado",
            Self::PortuguesePortugal => "Nenhum AirPods emparelhado detetado",
            Self::Indonesian => "Tidak ada AirPods terpasang yang terdeteksi",
            Self::Vietnamese => "Không phát hiện AirPods đã ghép đôi nào",
            Self::Russian => "Сопряженные AirPods не обнаружены",
            Self::Spanish => "No se detectaron AirPods emparejados",
        }
    }

    pub fn no_airpods_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "首次使用請先完成 Windows 藍牙配對，耳機名稱須保留「AirPods」或「Pods」。",
            Self::SimplifiedChinese => "首次使用请先完成 Windows 蓝牙配对，耳机名称须保留“AirPods”或“Pods”。",
            Self::English => "Pair via Windows Bluetooth settings first; device name must retain 'AirPods' or 'Pods'.",
            Self::French => "Jumelez d'abord via Windows Bluetooth ; le nom doit contenir « AirPods » ou « Pods ».",
            Self::German => "Zuerst über Windows Bluetooth koppeln; Gerätename muss „AirPods“ oder „Pods“ enthalten.",
            Self::Italian => "Associa prima tramite Bluetooth di Windows; il nome deve contenere \"AirPods\" o \"Pods\".",
            Self::Japanese => "初回は Windows の Bluetooth 設定からペアリングしてください。名前に「AirPods」または「Pods」を含める必要があります。",
            Self::Korean => "처음 사용 시 Windows Bluetooth 설정에서 페어링하세요. 이름에 'AirPods' 또는 'Pods'가 포함되어야 합니다.",
            Self::PortugueseBrazil => "Emparelhe pelo Bluetooth do Windows primeiro; o nome deve conter 'AirPods' ou 'Pods'.",
            Self::PortuguesePortugal => "Emparelhe através das definições de Bluetooth do Windows primeiro; o nome deve conter \"AirPods\" ou \"Pods\".",
            Self::Indonesian => "Pasangkan melalui pengaturan Bluetooth Windows terlebih dahulu; nama perangkat harus mempertahankan \"AirPods\" atau \"Pods\".",
            Self::Vietnamese => "Ghép đôi qua cài đặt Bluetooth của Windows trước; tên thiết bị phải giữ từ \"AirPods\" hoặc \"Pods\".",
            Self::Russian => "Сначала выполните сопряжение в Windows Bluetooth; имя должно содержать «AirPods» или «Pods».",
            Self::Spanish => "Empareje primero en Bluetooth de Windows; el nombre debe incluir 'AirPods' o 'Pods'.",
        }
    }

    pub fn airpods_in_use_status_tag(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "使用中",
            Self::SimplifiedChinese => "使用中",
            Self::English => "In Use",
            Self::French => "En cours d'utilisation",
            Self::German => "In Verwendung",
            Self::Italian => "In uso",
            Self::Japanese => "使用中",
            Self::Korean => "사용 중",
            Self::PortugueseBrazil => "Em uso",
            Self::PortuguesePortugal => "Em utilização",
            Self::Indonesian => "Sedang Digunakan",
            Self::Vietnamese => "Đang sử dụng",
            Self::Russian => "Используется",
            Self::Spanish => "En uso",
        }
    }

    pub fn airpods_standby_status_tag(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "待命中",
            Self::SimplifiedChinese => "待命中",
            Self::English => "Standby",
            Self::French => "En attente",
            Self::German => "Bereit",
            Self::Italian => "In attesa",
            Self::Japanese => "待機中",
            Self::Korean => "대기 중",
            Self::PortugueseBrazil => "Em espera",
            Self::PortuguesePortugal => "Em espera",
            Self::Indonesian => "Siaga",
            Self::Vietnamese => "Chế độ chờ",
            Self::Russian => "Ожидание",
            Self::Spanish => "En espera",
        }
    }

    pub fn airpods_connected_status_tag(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "已連線",
            Self::SimplifiedChinese => "已连接",
            Self::English => "Connected",
            Self::French => "Connecté",
            Self::German => "Verbunden",
            Self::Italian => "Connesso",
            Self::Japanese => "接続済み",
            Self::Korean => "연결됨",
            Self::PortugueseBrazil => "Conectado",
            Self::PortuguesePortugal => "Ligado",
            Self::Indonesian => "Terhubung",
            Self::Vietnamese => "Đã kết nối",
            Self::Russian => "Подключено",
            Self::Spanish => "Conectado",
        }
    }

    pub fn airpods_disconnected_status_tag(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "未連線",
            Self::SimplifiedChinese => "未连接",
            Self::English => "Disconnected",
            Self::French => "Déconnecté",
            Self::German => "Getrennt",
            Self::Italian => "Disconnesso",
            Self::Japanese => "未接続",
            Self::Korean => "연결 안 됨",
            Self::PortugueseBrazil => "Desconectado",
            Self::PortuguesePortugal => "Desligado",
            Self::Indonesian => "Terputus",
            Self::Vietnamese => "Đã ngắt kết nối",
            Self::Russian => "Отключено",
            Self::Spanish => "Desconectado",
        }
    }

    pub fn airpods_wear_to_connect_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "打開 AirPods 充電盒即會自動連線",
            Self::SimplifiedChinese => "打开 AirPods 充电盒即会自动连接",
            Self::English => "Open AirPods charging case to connect automatically",
            Self::French => "Ouvrez le boîtier de charge des AirPods pour vous connecter automatiquement",
            Self::German => "AirPods-Ladecase öffnen, um automatisch zu verbinden",
            Self::Italian => "Apri la custodia di ricarica degli AirPods per connetterti automaticamente",
            Self::Japanese => "AirPods の充電ケースを開けると自動的に接続されます",
            Self::Korean => "AirPods 충전 케이스를 열면 자동으로 연결됩니다",
            Self::PortugueseBrazil => "Abra o estojo de recarga dos AirPods para conectar automaticamente",
            Self::PortuguesePortugal => "Abra a caixa de carregamento dos AirPods para ligar automaticamente",
            Self::Indonesian => "Buka wadah pengisi daya AirPods untuk menghubungkan otomatis",
            Self::Vietnamese => "Mở hộp sạc AirPods để tự động kết nối",
            Self::Russian => "Откройте зарядный футляр AirPods для автоматического подключения",
            Self::Spanish => "Abre el estuche de carga de los AirPods para conectar automáticamente",
        }
    }

    pub fn btn_open_windows_bluetooth_short(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚙ 開啟 Windows 藍牙設定",
            Self::SimplifiedChinese => "⚙ 打开 Windows 蓝牙设置",
            Self::English => "⚙ Open Windows Bluetooth Settings",
            Self::French => "⚙ Ouvrir les paramètres Bluetooth",
            Self::German => "⚙ Windows-Bluetooth-Einstellungen öffnen",
            Self::Italian => "⚙ Apri impostazioni Bluetooth Windows",
            Self::Japanese => "⚙ Windows の Bluetooth 設定を開く",
            Self::Korean => "⚙ Windows Bluetooth 설정 열기",
            Self::PortugueseBrazil => "⚙ Abrir configurações de Bluetooth",
            Self::PortuguesePortugal => "⚙ Abrir Definições de Bluetooth",
            Self::Indonesian => "⚙ Buka Pengaturan Bluetooth Windows",
            Self::Vietnamese => "⚙ Mở Cài đặt Bluetooth của Windows",
            Self::Russian => "⚙ Открыть параметры Bluetooth",
            Self::Spanish => "⚙ Abrir configuración de Bluetooth",
        }
    }

    pub fn multi_room_label(&self, count: usize) -> String {
        match self {
            Self::TraditionalChinese => format!("(多房間同播 • {count} 部)"),
            Self::SimplifiedChinese => format!("(多房间同播 • {count} 部)"),
            Self::English => format!("(Multi-room sync • {count} devices)"),
            Self::French => format!("(Multi-pièces • {count} appareils)"),
            Self::German => format!("(Multiroom • {count} Geräte)"),
            Self::Italian => format!("(Multi-stanza • {count} dispositivi)"),
            Self::Japanese => format!("(マルチルーム同期 • {count}台)"),
            Self::Korean => format!("(멀티룸 동기화 • {count}대)"),
            Self::PortugueseBrazil => format!("(Multiambiente • {count} disp.)"),
            Self::PortuguesePortugal => format!("(Multi-divisão • {count} disp.)"),
            Self::Indonesian => format!("(Sinkronisasi multi-ruangan • {count} perangkat)"),
            Self::Vietnamese => format!("(Đồng bộ đa phòng • {count} thiết bị)"),
            Self::Russian => format!("(Мультирум • {count} устр.)"),
            Self::Spanish => format!("(Multisala • {count} disp.)"),
        }
    }

    pub fn no_devices_found(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "搜尋中 / 未找到支援的 AirPlay 裝置",
            Self::SimplifiedChinese => "搜索中 / 未找到支持的 AirPlay 设备",
            Self::English => "Searching / No supported AirPlay devices found",
            Self::French => "Recherche en cours / Aucun appareil AirPlay trouvé",
            Self::German => "Suche läuft / Keine AirPlay-Geräte gefunden",
            Self::Italian => "Ricerca in corso / Nessun dispositivo AirPlay trovato",
            Self::Japanese => "検索中 / サポートされているAirPlayデバイスが見つかりません",
            Self::Korean => "검색 중 / 지원되는 AirPlay 기기를 찾을 수 없음",
            Self::PortugueseBrazil => "Procurando / Nenhum dispositivo AirPlay encontrado",
            Self::PortuguesePortugal => "A procurar / Nenhum dispositivo AirPlay encontrado",
            Self::Indonesian => "Mencari / Tidak ada perangkat AirPlay yang didukung ditemukan",
            Self::Vietnamese => "Đang tìm kiếm / Không tìm thấy thiết bị AirPlay tương thích",
            Self::Russian => "Поиск / Устройства AirPlay не найдены",
            Self::Spanish => "Buscando / No se encontraron dispositivos AirPlay",
        }
    }

    pub fn stereo_pair_tag(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "立體聲組",
            Self::SimplifiedChinese => "立体声组",
            Self::English => "Stereo Pair",
            Self::French => "Paire stéréo",
            Self::German => "Stereopaar",
            Self::Italian => "Coppia stereo",
            Self::Japanese => "ステレオペア",
            Self::Korean => "스테레오 페어",
            Self::PortugueseBrazil => "Par estéreo",
            Self::PortuguesePortugal => "Par estéreo",
            Self::Indonesian => "Pasangan Stereo",
            Self::Vietnamese => "Cặp âm thanh nổi",
            Self::Russian => "Стереопара",
            Self::Spanish => "Par estéreo",
        }
    }

    pub fn stereo_pair_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "已偵測到 HomePod 立體聲配對，雙聲道串流將自動同步至左右聲道",
            Self::SimplifiedChinese => "已检测到 HomePod 立体声配对，双声道串流将自动同步至左右声道",
            Self::English => "HomePod stereo pair detected; stereo stream will sync automatically to left & right channels",
            Self::French => "Paire stéréo HomePod détectée ; synchronisation automatique gauche/droite",
            Self::German => "HomePod-Stereopaar erkannt; Stereo-Stream wird automatisch synchronisiert",
            Self::Italian => "Coppia stereo HomePod rilevata; sincronizzazione automatica canali sinistro e destro",
            Self::Japanese => "HomePodステレオペアが検出されました。左右チャンネルに自動同期されます",
            Self::Korean => "HomePod 스테레오 페어가 감지되었습니다. 좌우 채널에 자동 동기화됩니다",
            Self::PortugueseBrazil => "Par estéreo HomePod detectado; áudio sincronizado automaticamente nos canais E/D",
            Self::PortuguesePortugal => "Par estéreo HomePod detetado; fluxo estéreo sincronizado automaticamente nos canais E/D",
            Self::Indonesian => "Pasangan stereo HomePod terdeteksi; streaming stereo akan disinkronkan otomatis ke saluran kiri & kanan",
            Self::Vietnamese => "Đã phát hiện cặp âm thanh nổi HomePod; luồng âm thanh nổi sẽ tự động đồng bộ sang kênh trái & phải",
            Self::Russian => "Обнаружена стереопара HomePod; стереопоток автоматически синхронизируется",
            Self::Spanish => "Par estéreo HomePod detectado; sincronización automática a los canales izquierdo y derecho",
        }
    }

    pub fn btn_leave(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "退出",
            Self::SimplifiedChinese => "退出",
            Self::English => "Leave",
            Self::French => "Quitter",
            Self::German => "Verlassen",
            Self::Italian => "Esci",
            Self::Japanese => "離脱",
            Self::Korean => "제외",
            Self::PortugueseBrazil => "Sair",
            Self::PortuguesePortugal => "Sair",
            Self::Indonesian => "Tinggalkan",
            Self::Vietnamese => "Rời khỏi",
            Self::Russian => "Выйти",
            Self::Spanish => "Salir",
        }
    }

    pub fn btn_add_to_group(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "＋ 加入播放",
            Self::SimplifiedChinese => "＋ 加入播放",
            Self::English => "+ Add to Playback",
            Self::French => "+ Ajouter à la lecture",
            Self::German => "+ Zur Wiedergabe hinzufügen",
            Self::Italian => "+ Aggiungi alla riproduzione",
            Self::Japanese => "+ 再生に追加",
            Self::Korean => "+ 재생에 추가",
            Self::PortugueseBrazil => "+ Adicionar à reprodução",
            Self::PortuguesePortugal => "+ Adicionar à reprodução",
            Self::Indonesian => "+ Tambahkan ke Pemutaran",
            Self::Vietnamese => "+ Thêm vào danh sách phát",
            Self::Russian => "+ Добавить к трансляции",
            Self::Spanish => "+ Agregar a reproducción",
        }
    }

    pub fn btn_connect(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線",
            Self::SimplifiedChinese => "连接",
            Self::English => "Connect",
            Self::French => "Connecter",
            Self::German => "Verbinden",
            Self::Italian => "Connetti",
            Self::Japanese => "接続",
            Self::Korean => "연결",
            Self::PortugueseBrazil => "Conectar",
            Self::PortuguesePortugal => "Ligar",
            Self::Indonesian => "Hubungkan",
            Self::Vietnamese => "Kết nối",
            Self::Russian => "Подключить",
            Self::Spanish => "Conectar",
        }
    }

    pub fn tooltip_individual_vol(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "獨立微調此音響音量",
            Self::SimplifiedChinese => "独立微调此音响音量",
            Self::English => "Fine-tune this speaker's volume individually",
            Self::French => "Ajuster individuellement le volume de cette enceinte",
            Self::German => "Lautstärke dieses Lautsprechers individuell anpassen",
            Self::Italian => "Regola singolarmente il volume di questo altoparlante",
            Self::Japanese => "このスピーカーの音量を個別に微調整",
            Self::Korean => "이 스피커의 볼륨을 개별적으로 미세 조정",
            Self::PortugueseBrazil => "Ajustar volume individual deste alto-falante",
            Self::PortuguesePortugal => "Ajustar volume individual deste altifalante",
            Self::Indonesian => "Sesuaikan volume speaker ini secara individual",
            Self::Vietnamese => "Tinh chỉnh riêng âm lượng cho loa này",
            Self::Russian => "Индивидуальная настройка громкости этой колонки",
            Self::Spanish => "Ajustar individualmente el volumen de este altavoz",
        }
    }

    pub fn batch_connect_btn(&self, count: usize) -> String {
        match self {
            Self::TraditionalChinese => format!("▶ 同時連線選取的 {count} 部音響 (多房間群組)"),
            Self::SimplifiedChinese => format!("▶ 同时连接选取的 {count} 部音响 (多房间群组)"),
            Self::English => format!("▶ Connect selected {count} speakers (Multi-room group)"),
            Self::French => format!("▶ Connecter les {count} enceintes sélectionnées (Multi-pièces)"),
            Self::German => format!("▶ {count} ausgewählte Lautsprecher verbinden (Multiroom)"),
            Self::Italian => format!("▶ Connetti i {count} altoparlanti selezionati (Multi-stanza)"),
            Self::Japanese => format!("▶ 選択した{count}台のスピーカーを同時接続 (マルチルーム)"),
            Self::Korean => format!("▶ 선택한 {count}대 스피커 동시 연결 (멀티룸 그룹)"),
            Self::PortugueseBrazil => format!("▶ Conectar os {count} alto-falantes selecionados (Multiambiente)"),
            Self::PortuguesePortugal => format!("▶ Ligar os {count} altifalantes selecionados (Grupo multi-divisão)"),
            Self::Indonesian => format!("▶ Hubungkan {count} speaker terpilih (Grup multi-ruangan)"),
            Self::Vietnamese => format!("▶ Kết nối {count} loa đã chọn (Nhóm đa phòng)"),
            Self::Russian => format!("▶ Подключить выбранные {count} колонок (Мультирум)"),
            Self::Spanish => format!("▶ Conectar los {count} altavoces seleccionados (Multisala)"),
        }
    }

    pub fn volume_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音量與靜音控制",
            Self::SimplifiedChinese => "音量与静音控制",
            Self::English => "Volume & Mute Controls",
            Self::French => "Contrôle du volume et sourdine",
            Self::German => "Lautstärke- und Stummschaltung",
            Self::Italian => "Controllo volume e muto",
            Self::Japanese => "音量とミュート設定",
            Self::Korean => "볼륨 및 음소거 설정",
            Self::PortugueseBrazil => "Controle de volume e mudo",
            Self::PortuguesePortugal => "Controlos de volume e sem som",
            Self::Indonesian => "Kontrol Volume & Bisu",
            Self::Vietnamese => "Điều khiển âm lượng & Tắt tiếng",
            Self::Russian => "Управление громкостью и звуком",
            Self::Spanish => "Control de volumen y silencio",
        }
    }

    pub fn vol_speaker_muted(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("🔇 {name} 已靜音"),
            Self::SimplifiedChinese => format!("🔇 {name} 已静音"),
            Self::English => format!("🔇 {name} is muted"),
            Self::French => format!("🔇 {name} est muet"),
            Self::German => format!("🔇 {name} ist stumm"),
            Self::Italian => format!("🔇 {name} è disattivato"),
            Self::Japanese => format!("🔇 {name} はミュート中"),
            Self::Korean => format!("🔇 {name} 음소거됨"),
            Self::PortugueseBrazil => format!("🔇 {name} silenciado"),
            Self::PortuguesePortugal => format!("🔇 {name} sem som"),
            Self::Indonesian => format!("🔇 {name} dibisukan"),
            Self::Vietnamese => format!("🔇 {name} đã tắt tiếng"),
            Self::Russian => format!("🔇 {name} без звука"),
            Self::Spanish => format!("🔇 {name} silenciado"),
        }
    }

    pub fn vol_speaker_playing(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("🔊 {name} 現正播放中"),
            Self::SimplifiedChinese => format!("🔊 {name} 正在播放中"),
            Self::English => format!("🔊 {name} is playing"),
            Self::French => format!("🔊 {name} en lecture"),
            Self::German => format!("🔊 {name} wird wiedergegeben"),
            Self::Italian => format!("🔊 {name} in riproduzione"),
            Self::Japanese => format!("🔊 {name} 再生中"),
            Self::Korean => format!("🔊 {name} 재생 중"),
            Self::PortugueseBrazil => format!("🔊 {name} reproduzindo"),
            Self::PortuguesePortugal => format!("🔊 {name} em reprodução"),
            Self::Indonesian => format!("🔊 {name} sedang memutar"),
            Self::Vietnamese => format!("🔊 {name} đang phát"),
            Self::Russian => format!("🔊 {name} играет"),
            Self::Spanish => format!("🔊 {name} reproduciendo"),
        }
    }

    pub fn vol_connecting_muted(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⏳ 連線中 (靜音模式)",
            Self::SimplifiedChinese => "⏳ 连接中 (静音模式)",
            Self::English => "⏳ Connecting (Muted)",
            Self::French => "⏳ Connexion (Muet)",
            Self::German => "⏳ Verbinden (Stumm)",
            Self::Italian => "⏳ Connessione (Muto)",
            Self::Japanese => "⏳ 接続中 (ミュート)",
            Self::Korean => "⏳ 연결 중 (음소거)",
            Self::PortugueseBrazil => "⏳ Conectando (Mudo)",
            Self::PortuguesePortugal => "⏳ A ligar (Sem som)",
            Self::Indonesian => "⏳ Menghubungkan (Dibisukan)",
            Self::Vietnamese => "⏳ Đang kết nối (Đã tắt tiếng)",
            Self::Russian => "⏳ Подключение (Без звука)",
            Self::Spanish => "⏳ Conectando (Silenciado)",
        }
    }

    pub fn vol_connecting(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⏳ 連線中...",
            Self::SimplifiedChinese => "⏳ 连接中...",
            Self::English => "⏳ Connecting...",
            Self::French => "⏳ Connexion...",
            Self::German => "⏳ Verbinden...",
            Self::Italian => "⏳ Connessione...",
            Self::Japanese => "⏳ 接続中...",
            Self::Korean => "⏳ 연결 중...",
            Self::PortugueseBrazil => "⏳ Conectando...",
            Self::PortuguesePortugal => "⏳ A ligar...",
            Self::Indonesian => "⏳ Menghubungkan...",
            Self::Vietnamese => "⏳ Đang kết nối...",
            Self::Russian => "⏳ Подключение...",
            Self::Spanish => "⏳ Conectando...",
        }
    }

    pub fn vol_idle_muted(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔇 靜音模式 (待機中 / 尚未連線)",
            Self::SimplifiedChinese => "🔇 静音模式 (待机中 / 尚未连接)",
            Self::English => "🔇 Muted (Idle / Not connected)",
            Self::French => "🔇 Muet (En attente / Non connecté)",
            Self::German => "🔇 Stummgeschaltet (Bereit / Nicht verbunden)",
            Self::Italian => "🔇 Silenzioso (In attesa / Non connesso)",
            Self::Japanese => "🔇 ミュート中 (待機中 / 未接続)",
            Self::Korean => "🔇 음소거 모드 (대기 중 / 연결되지 않음)",
            Self::PortugueseBrazil => "🔇 Silenciado (Em espera / Não conectado)",
            Self::PortuguesePortugal => "🔇 Sem som (Em espera / Não ligado)",
            Self::Indonesian => "🔇 Dibisukan (Siaga / Tidak terhubung)",
            Self::Vietnamese => "🔇 Đã tắt tiếng (Chờ / Chưa kết nối)",
            Self::Russian => "🔇 Без звука (Ожидание / Не подключено)",
            Self::Spanish => "🔇 Silenciado (En espera / No conectado)",
        }
    }

    pub fn vol_idle(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔊 待機中 (尚未連線)",
            Self::SimplifiedChinese => "🔊 待机中 (尚未连接)",
            Self::English => "🔊 Idle (Not connected)",
            Self::French => "🔊 En attente (Non connecté)",
            Self::German => "🔊 Bereit (Nicht verbunden)",
            Self::Italian => "🔊 In attesa (Non connesso)",
            Self::Japanese => "🔊 待機中 (未接続)",
            Self::Korean => "🔊 대기 중 (연결되지 않음)",
            Self::PortugueseBrazil => "🔊 Em espera (Não conectado)",
            Self::PortuguesePortugal => "🔊 Em espera (Não ligado)",
            Self::Indonesian => "🔊 Siaga (Tidak terhubung)",
            Self::Vietnamese => "🔊 Chờ (Chưa kết nối)",
            Self::Russian => "🔊 Ожидание (Не подключено)",
            Self::Spanish => "🔊 En espera (No conectado)",
        }
    }

    pub fn vol_error_muted(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔇 靜音模式 (連線異常)",
            Self::SimplifiedChinese => "🔇 静音模式 (连接异常)",
            Self::English => "🔇 Muted (Connection error)",
            Self::French => "🔇 Muet (Erreur de connexion)",
            Self::German => "🔇 Stumm (Verbindungsfehler)",
            Self::Italian => "🔇 Muto (Errore di connessione)",
            Self::Japanese => "🔇 ミュート中 (接続異常)",
            Self::Korean => "🔇 음소거 모드 (연결 오류)",
            Self::PortugueseBrazil => "🔇 Silenciado (Erro de conexão)",
            Self::PortuguesePortugal => "🔇 Sem som (Erro de ligação)",
            Self::Indonesian => "🔇 Dibisukan (Kesalahan koneksi)",
            Self::Vietnamese => "🔇 Đã tắt tiếng (Lỗi kết nối)",
            Self::Russian => "🔇 Без звука (Ошибка соединения)",
            Self::Spanish => "🔇 Silenciado (Error de conexión)",
        }
    }

    pub fn vol_error(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔊 待機 (連線異常)",
            Self::SimplifiedChinese => "🔊 待机 (连接异常)",
            Self::English => "🔊 Idle (Connection error)",
            Self::French => "🔊 En attente (Erreur de connexion)",
            Self::German => "🔊 Bereit (Verbindungsfehler)",
            Self::Italian => "🔊 In attesa (Errore di connessione)",
            Self::Japanese => "🔊 待機 (接続異常)",
            Self::Korean => "🔊 대기 (연결 오류)",
            Self::PortugueseBrazil => "🔊 Em espera (Erro de conexão)",
            Self::PortuguesePortugal => "🔊 Em espera (Erro de ligação)",
            Self::Indonesian => "🔊 Siaga (Kesalahan koneksi)",
            Self::Vietnamese => "🔊 Chờ (Lỗi kết nối)",
            Self::Russian => "🔊 Ожидание (Ошибка соединения)",
            Self::Spanish => "🔊 En espera (Error de conexión)",
        }
    }

    pub fn btn_unmute(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔊 取消靜音",
            Self::SimplifiedChinese => "🔊 取消静音",
            Self::English => "🔊 Unmute",
            Self::French => "🔊 Rétablir",
            Self::German => "🔊 Laut",
            Self::Italian => "🔊 Riattiva",
            Self::Japanese => "🔊 解除",
            Self::Korean => "🔊 해제",
            Self::PortugueseBrazil => "🔊 Som",
            Self::PortuguesePortugal => "🔊 Ativar som",
            Self::Indonesian => "🔊 Bunyikan",
            Self::Vietnamese => "🔊 Bật tiếng",
            Self::Russian => "🔊 Звук",
            Self::Spanish => "🔊 Sonido",
        }
    }

    pub fn btn_mute(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔇 一鍵靜音",
            Self::SimplifiedChinese => "🔇 一键静音",
            Self::English => "🔇 Mute",
            Self::French => "🔇 Muet",
            Self::German => "🔇 Stumm",
            Self::Italian => "🔇 Muto",
            Self::Japanese => "🔇 ミュート",
            Self::Korean => "🔇 음소거",
            Self::PortugueseBrazil => "🔇 Mudo",
            Self::PortuguesePortugal => "🔇 Sem som",
            Self::Indonesian => "🔇 Bisukan",
            Self::Vietnamese => "🔇 Tắt tiếng",
            Self::Russian => "🔇 Без звука",
            Self::Spanish => "🔇 Silencio",
        }
    }

    pub fn group_volume_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "群組總音量:",
            Self::SimplifiedChinese => "群组总音量:",
            Self::English => "Group Volume:",
            Self::French => "Volume du groupe :",
            Self::German => "Gesamtlautstärke:",
            Self::Italian => "Volume di gruppo:",
            Self::Japanese => "グループ全体音量:",
            Self::Korean => "그룹 전체 볼륨:",
            Self::PortugueseBrazil => "Volume do grupo:",
            Self::PortuguesePortugal => "Volume do grupo:",
            Self::Indonesian => "Volume Grup:",
            Self::Vietnamese => "Âm lượng nhóm:",
            Self::Russian => "Общая громкость:",
            Self::Spanish => "Volumen del grupo:",
        }
    }

    pub fn group_volume_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連動等量調整所有已連線音響，各音響亦可在上方列表獨立微調相對音量",
            Self::SimplifiedChinese => "联动等量调整所有已连接音响，各音响亦可在上方列表独立微调相对音量",
            Self::English => "Adjusts all connected speakers together; individual volume can be adjusted above",
            Self::French => "Ajuste toutes les enceintes ensemble ; ajustements individuels possibles ci-dessus",
            Self::German => "Passt alle verbundenen Lautsprecher an; individuelle Anpassung oben möglich",
            Self::Italian => "Regola tutti gli altoparlanti collegati; volume individuale regolabile sopra",
            Self::Japanese => "接続中のすべてのスピーカーを連動調整します。個別調整は上記リストで行えます",
            Self::Korean => "연결된 모든 스피커 볼륨을 함께 조절합니다. 개별 볼륨은 상단 목록에서 조절 가능합니다",
            Self::PortugueseBrazil => "Ajusta todos os alto-falantes conectados; ajustes individuais acima",
            Self::PortuguesePortugal => "Ajusta todos os altifalantes ligados em conjunto; volume individual pode ser ajustado acima",
            Self::Indonesian => "Menyesuaikan semua speaker yang terhubung bersama-sama; volume individu dapat disesuaikan di atas",
            Self::Vietnamese => "Điều chỉnh đồng thời tất cả các loa đang kết nối; âm lượng từng loa có thể chỉnh ở trên",
            Self::Russian => "Регулирует громкость всех колонок; индивидуальная настройка доступна выше",
            Self::Spanish => "Ajusta todos los altavoces conectados; el volumen individual se puede ajustar arriba",
        }
    }

    pub fn volume_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音量:",
            Self::SimplifiedChinese => "音量:",
            Self::English => "Volume:",
            Self::French => "Volume :",
            Self::German => "Lautstärke:",
            Self::Italian => "Volume:",
            Self::Japanese => "音量:",
            Self::Korean => "볼륨:",
            Self::PortugueseBrazil => "Volume:",
            Self::PortuguesePortugal => "Volume:",
            Self::Indonesian => "Volume:",
            Self::Vietnamese => "Âm lượng:",
            Self::Russian => "Громкость:",
            Self::Spanish => "Volumen:",
        }
    }

    pub fn slider_muted_suffix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => " (靜音中)",
            Self::SimplifiedChinese => " (静音中)",
            Self::English => " (Muted)",
            Self::French => " (Muet)",
            Self::German => " (Stumm)",
            Self::Italian => " (Muto)",
            Self::Japanese => " (ミュート中)",
            Self::Korean => " (음소거됨)",
            Self::PortugueseBrazil => " (Mudo)",
            Self::PortuguesePortugal => " (Sem som)",
            Self::Indonesian => " (Dibisukan)",
            Self::Vietnamese => " (Đã tắt tiếng)",
            Self::Russian => " (Без звука)",
            Self::Spanish => " (Silenciado)",
        }
    }

    pub fn slider_default_suffix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => " (預設音量)",
            Self::SimplifiedChinese => " (默认音量)",
            Self::English => " (Default)",
            Self::French => " (Par défaut)",
            Self::German => " (Standard)",
            Self::Italian => " (Predefinito)",
            Self::Japanese => " (デフォルト)",
            Self::Korean => " (기본 볼륨)",
            Self::PortugueseBrazil => " (Padrão)",
            Self::PortuguesePortugal => " (Predefinido)",
            Self::Indonesian => " (Default)",
            Self::Vietnamese => " (Mặc định)",
            Self::Russian => " (По умолч.)",
            Self::Spanish => " (Predeterminado)",
        }
    }

    pub fn advanced_settings_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "進階設定",
            Self::SimplifiedChinese => "高级设置",
            Self::English => "Advanced Settings",
            Self::French => "Paramètres avancés",
            Self::German => "Erweiterte Einstellungen",
            Self::Italian => "Impostazioni avanzate",
            Self::Japanese => "詳細設定",
            Self::Korean => "고급 설정",
            Self::PortugueseBrazil => "Configurações Avançadas",
            Self::PortuguesePortugal => "Definições Avançadas",
            Self::Indonesian => "Pengaturan Lanjutan",
            Self::Vietnamese => "Cài đặt nâng cao",
            Self::Russian => "Расширенные настройки",
            Self::Spanish => "Configuración avanzada",
        }
    }

    pub fn settings_tab_audio(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊與耳機",
            Self::SimplifiedChinese => "音频与耳机",
            Self::English => "Audio & Devices",
            Self::French => "Audio et périphériques",
            Self::German => "Audio & Geräte",
            Self::Italian => "Audio e dispositivi",
            Self::Japanese => "オーディオと機器",
            Self::Korean => "오디오 및 기기",
            Self::PortugueseBrazil => "Áudio e dispositivos",
            Self::PortuguesePortugal => "Áudio e dispositivos",
            Self::Indonesian => "Audio & Perangkat",
            Self::Vietnamese => "Âm thanh & Thiết bị",
            Self::Russian => "Аудио и устройства",
            Self::Spanish => "Audio y dispositivos",
        }
    }

    pub fn settings_tab_receiver(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊接收",
            Self::SimplifiedChinese => "音频接收",
            Self::English => "Audio Receiver",
            Self::French => "Récepteur audio",
            Self::German => "Audioempfänger",
            Self::Italian => "Ricevitore audio",
            Self::Japanese => "音声受信",
            Self::Korean => "오디오 수신",
            Self::PortugueseBrazil => "Receptor de áudio",
            Self::PortuguesePortugal => "Recetor de áudio",
            Self::Indonesian => "Penerima Audio",
            Self::Vietnamese => "Bộ nhận âm thanh",
            Self::Russian => "Аудиоприемник",
            Self::Spanish => "Receptor de audio",
        }
    }

    pub fn settings_tab_system(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "系統與串流",
            Self::SimplifiedChinese => "系统与串流",
            Self::English => "System & Streaming",
            Self::French => "Système et diffusion",
            Self::German => "System & Streaming",
            Self::Italian => "Sistema e streaming",
            Self::Japanese => "システムとストリーミング",
            Self::Korean => "시스템 및 스트리밍",
            Self::PortugueseBrazil => "Sistema e streaming",
            Self::PortuguesePortugal => "Sistema e transmissão",
            Self::Indonesian => "Sistem & Streaming",
            Self::Vietnamese => "Hệ thống & Truyền phát",
            Self::Russian => "Система и трансляция",
            Self::Spanish => "Sistema y transmisión",
        }
    }

    pub fn audio_source_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊來源 (Audio Source):",
            Self::SimplifiedChinese => "音频来源 (Audio Source):",
            Self::English => "Audio Source:",
            Self::French => "Source audio :",
            Self::German => "Audioquelle:",
            Self::Italian => "Sorgente audio:",
            Self::Japanese => "音声ソース:",
            Self::Korean => "오디오 소스:",
            Self::PortugueseBrazil => "Fonte de áudio:",
            Self::PortuguesePortugal => "Fonte de áudio:",
            Self::Indonesian => "Sumber Audio:",
            Self::Vietnamese => "Nguồn âm thanh:",
            Self::Russian => "Источник звука:",
            Self::Spanish => "Fuente de audio:",
        }
    }

    pub fn source_mode_device(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🖥 系統與裝置",
            Self::SimplifiedChinese => "🖥 系统与设备",
            Self::English => "🖥 System & Devices",
            Self::French => "🖥 Système et périphériques",
            Self::German => "🖥 System & Geräte",
            Self::Italian => "🖥 Sistema e dispositivi",
            Self::Japanese => "🖥 システムとデバイス",
            Self::Korean => "🖥 시스템 및 장치",
            Self::PortugueseBrazil => "🖥 Sistema e dispositivos",
            Self::PortuguesePortugal => "🖥 Sistema e dispositivos",
            Self::Indonesian => "🖥 Sistem & Perangkat",
            Self::Vietnamese => "🖥 Hệ thống & Thiết bị",
            Self::Russian => "🖥 Система и устройства",
            Self::Spanish => "🖥 Sistema y dispositivos",
        }
    }

    pub fn source_mode_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎯 應用程式",
            Self::SimplifiedChinese => "🎯 应用程序",
            Self::English => "🎯 Application",
            Self::French => "🎯 Application",
            Self::German => "🎯 Anwendung",
            Self::Italian => "🎯 Applicazione",
            Self::Japanese => "🎯 アプリケーション",
            Self::Korean => "🎯 애플리케이션",
            Self::PortugueseBrazil => "🎯 Aplicativo",
            Self::PortuguesePortugal => "🎯 Aplicação",
            Self::Indonesian => "🎯 Aplikasi",
            Self::Vietnamese => "🎯 Ứng dụng",
            Self::Russian => "🎯 Приложение",
            Self::Spanish => "🎯 Aplicación",
        }
    }

    pub fn select_app_placeholder(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "選擇要串流的應用程式...",
            Self::SimplifiedChinese => "选择要串流的应用程序...",
            Self::English => "Select an application to cast...",
            Self::French => "Sélectionner une application à diffuser...",
            Self::German => "Anwendung zum Streamen auswählen...",
            Self::Italian => "Seleziona un'applicazione da trasmettere...",
            Self::Japanese => "キャストするアプリを選択...",
            Self::Korean => "스트리밍할 애플리케이션 선택...",
            Self::PortugueseBrazil => "Selecione um aplicativo para transmitir...",
            Self::PortuguesePortugal => "Selecione uma aplicação para transmitir...",
            Self::Indonesian => "Pilih aplikasi untuk streaming...",
            Self::Vietnamese => "Chọn ứng dụng để truyền phát...",
            Self::Russian => "Выберите приложение для трансляции...",
            Self::Spanish => "Selecciona una aplicación para transmitir...",
        }
    }

    pub fn tooltip_rescan_apps(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "重新整理執行中的應用程式清單",
            Self::SimplifiedChinese => "刷新正在运行的应用程序列表",
            Self::English => "Refresh list of running applications",
            Self::French => "Actualiser la liste des applications",
            Self::German => "Liste der laufenden Anwendungen aktualisieren",
            Self::Italian => "Aggiorna l'elenco delle applicazioni",
            Self::Japanese => "実行中のアプリ一覧を更新",
            Self::Korean => "실행 중인 애플리케이션 목록 새로고침",
            Self::PortugueseBrazil => "Atualizar lista de aplicativos",
            Self::PortuguesePortugal => "Atualizar lista de aplicações em execução",
            Self::Indonesian => "Perbarui daftar aplikasi yang berjalan",
            Self::Vietnamese => "Làm mới danh sách ứng dụng đang chạy",
            Self::Russian => "Обновить список запущенных приложений",
            Self::Spanish => "Actualizar lista de aplicaciones",
        }
    }

    pub fn no_apps_found(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "未偵測到相容的應用程式",
            Self::SimplifiedChinese => "未检测到兼容的应用程序",
            Self::English => "No compatible applications detected",
            Self::French => "Aucune application compatible détectée",
            Self::German => "Keine kompatiblen Anwendungen erkannt",
            Self::Italian => "Nessuna applicazione compatibile rilevata",
            Self::Japanese => "互換性のあるアプリが見つかりません",
            Self::Korean => "호환되는 애플리케이션을 찾을 수 없습니다",
            Self::PortugueseBrazil => "Nenhum aplicativo compatível detectado",
            Self::PortuguesePortugal => "Nenhuma aplicação compatível detetada",
            Self::Indonesian => "Tidak ada aplikasi yang kompatibel terdeteksi",
            Self::Vietnamese => "Không phát hiện ứng dụng tương thích nào",
            Self::Russian => "Совместимые приложения не найдены",
            Self::Spanish => "No se encontraron aplicaciones compatibles",
        }
    }

    pub fn app_mode_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 應用程式模式：直接擷取指定軟體的音訊並串流至 HomePod；建議開啟「連線時自動關閉電腦喇叭」以避免電腦與音響同時發聲產生時序落差與回音。",
            Self::SimplifiedChinese => "💡 应用程序模式：直接捕获指定软件的音频并串流至 HomePod；建议开启“连接时自动关闭电脑扬声器”以避免电脑与音箱同时发声产生时序差与回音。",
            Self::English => "💡 App Mode: Streams audio from the chosen app directly to HomePod. Keeping 'Auto-mute computer speakers' enabled prevents echo and timing delays.",
            Self::French => "💡 Mode Application : Diffuse le son de l'application choisie vers le HomePod. Activer la coupure automatique des haut-parleurs évite l'écho et le décalage.",
            Self::German => "💡 App-Modus: Streamt Audio der gewählten App direkt auf den HomePod. Das Aktivieren der automatischen Stummschaltung verhindert Echo und Zeitverzögerungen.",
            Self::Italian => "💡 Modalità App: Trasmette l'audio dell'app scelta direttamente a HomePod. Abilitare il silenziamento automatico evita ritardi e riverberi.",
            Self::Japanese => "💡 アプリモード: 選択したアプリの音声をHomePodに直接ストリーミングします。「接続時にPCスピーカーを自動ミュート」を有効にすると、PCとスピーカーの同時再生によるズレやエコーを防げます。",
            Self::Korean => "💡 앱 모드: 선택한 앱의 오디오를 HomePod으로 직접 스트리밍합니다. '연결 시 PC 스피커 자동 음소거'를 활성화하면 동시 재생으로 인한 시간차와 에코를 방지할 수 있습니다.",
            Self::PortugueseBrazil => "💡 Modo Aplicativo: Transmite o áudio do aplicativo diretamente para o HomePod. Manter o mudo automático ativado evita atrasos e eco.",
            Self::PortuguesePortugal => "💡 Modo Aplicação: Transmite o áudio da aplicação diretamente para o HomePod. Manter o corte automático ativado evita atrasos e eco.",
            Self::Indonesian => "💡 Mode Aplikasi: Mengalirkan audio dari aplikasi yang dipilih langsung ke HomePod. Mengaktifkan \"Bisukan speaker komputer otomatis\" mencegah gema dan penundaan waktu.",
            Self::Vietnamese => "💡 Chế độ ứng dụng: Truyền phát âm thanh từ ứng dụng đã chọn trực tiếp tới HomePod. Bật \"Tự động tắt tiếng loa máy tính\" giúp chống tiếng vang và trễ tiếng.",
            Self::Russian => "💡 Режим приложений: прямая трансляция звука программы на HomePod. Включение автозаглушения колонок ПК предотвращает эхо и рассинхрон.",
            Self::Spanish => "💡 Modo de aplicación: Transmite el audio de la app directamente a HomePod. Activar el silenciado automático de los altavoces evita eco y desfases.",
        }
    }

    pub fn audio_source_default(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "全系統混音 (預設輸出裝置)",
            Self::SimplifiedChinese => "全系统混音 (默认输出设备)",
            Self::English => "System Audio (Default Output Device)",
            Self::French => "Audio système (Périphérique par défaut)",
            Self::German => "Systemaudio (Standardausgabegerät)",
            Self::Italian => "Audio di sistema (Dispositivo predefinito)",
            Self::Japanese => "システム全体音声 (既定の出力デバイス)",
            Self::Korean => "전체 시스템 오디오 (기본 출력 장치)",
            Self::PortugueseBrazil => "Áudio do sistema (Dispositivo padrão)",
            Self::PortuguesePortugal => "Áudio do sistema (Dispositivo de saída predefinido)",
            Self::Indonesian => "Audio Sistem (Perangkat Output Default)",
            Self::Vietnamese => "Âm thanh hệ thống (Thiết bị đầu ra mặc định)",
            Self::Russian => "Все системные звуки (По умолчанию)",
            Self::Spanish => "Audio del sistema (Dispositivo predeterminado)",
        }
    }

    pub fn default_output_device(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "預設輸出裝置",
            Self::SimplifiedChinese => "默认输出设备",
            Self::English => "Default Output Device",
            Self::French => "Périphérique de sortie par défaut",
            Self::German => "Standardausgabegerät",
            Self::Italian => "Dispositivo di uscita predefinito",
            Self::Japanese => "既定の出力デバイス",
            Self::Korean => "기본 출력 장치",
            Self::PortugueseBrazil => "Dispositivo de saída padrão",
            Self::PortuguesePortugal => "Dispositivo de saída predefinido",
            Self::Indonesian => "Perangkat Output Default",
            Self::Vietnamese => "Thiết bị đầu ra mặc định",
            Self::Russian => "Устройство вывода по умолчанию",
            Self::Spanish => "Dispositivo de salida predeterminado",
        }
    }

    pub fn tooltip_rescan_audio(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "重新掃描系統音訊裝置 (若剛插上耳機或安裝虛擬音效卡)",
            Self::SimplifiedChinese => "重新扫描系统音频设备 (若刚插上耳机或安装虚拟声卡)",
            Self::English => "Rescan audio devices (e.g., if headphones or virtual devices were connected)",
            Self::French => "Analyser à nouveau les périphériques audio (ex. casque branché)",
            Self::German => "Audiogeräte neu scannen (z. B. nach Anschließen von Kopfhörern)",
            Self::Italian => "Riscansiona dispositivi audio (es. cuffie appena collegate)",
            Self::Japanese => "オーディオデバイスを再スキャン (ヘッドホン接続時など)",
            Self::Korean => "오디오 장치 다시 검색 (헤드폰 연결 또는 가상 사운드카드 설치 시)",
            Self::PortugueseBrazil => "Escanear dispositivos de áudio novamente",
            Self::PortuguesePortugal => "Procurar dispositivos de áudio novamente (ex.: se auscultadores foram ligados)",
            Self::Indonesian => "Pindai ulang perangkat audio (misalnya jika headphone atau perangkat virtual terhubung)",
            Self::Vietnamese => "Quét lại các thiết bị âm thanh (ví dụ: khi vừa cắm tai nghe hoặc thiết bị ảo)",
            Self::Russian => "Повторное сканирование аудиоустройств",
            Self::Spanish => "Volver a escanear dispositivos de audio",
        }
    }

    pub fn split_mode_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "ℹ 分流模式：",
            Self::SimplifiedChinese => "ℹ 分流模式：",
            Self::English => "ℹ Split Routing Mode:",
            Self::French => "ℹ Mode routage séparé :",
            Self::German => "ℹ Split-Routing-Modus:",
            Self::Italian => "ℹ Modalità routing separato:",
            Self::Japanese => "ℹ 個別ルーティングモード：",
            Self::Korean => "ℹ 개별 라우팅 모드:",
            Self::PortugueseBrazil => "ℹ Modo de roteamento separado:",
            Self::PortuguesePortugal => "ℹ Modo de encaminhamento separado:",
            Self::Indonesian => "ℹ Mode Perutean Terpisah:",
            Self::Vietnamese => "ℹ Chế độ định tuyến riêng biệt:",
            Self::Russian => "ℹ Режим раздельного вывода:",
            Self::Spanish => "ℹ Modo de enrutamiento separado:",
        }
    }

    pub fn split_mode_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "僅串流選定裝置之音樂，電腦主喇叭維持正常運作不靜音。",
            Self::SimplifiedChinese => "仅串流选定设备之音乐，电脑主喇叭维持正常运作不静音。",
            Self::English => "Streams only selected device's audio; PC main speakers remain active.",
            Self::French => "Diffuse uniquement le périphérique sélectionné ; haut-parleurs du PC actifs.",
            Self::German => "Streamt nur das ausgewählte Gerät; PC-Lautsprecher bleiben aktiv.",
            Self::Italian => "Trasmette solo l'audio del dispositivo selezionato; altoparlanti PC attivi.",
            Self::Japanese => "選択したデバイスの音声のみを配信し、PCのメインスピーカーは鳴り続けます。",
            Self::Korean => "선택한 장치의 오디오만 스트리밍하며, PC 주 스피커는 정상 작동합니다.",
            Self::PortugueseBrazil => "Transmite apenas o áudio do dispositivo selecionado; alto-falantes do PC ativos.",
            Self::PortuguesePortugal => "Transmite apenas o áudio do dispositivo selecionado; os altifalantes principais do PC mantêm-se ativos.",
            Self::Indonesian => "Mengalirkan hanya audio perangkat yang dipilih; speaker utama PC tetap aktif.",
            Self::Vietnamese => "Chỉ truyền phát âm thanh của thiết bị đã chọn; loa chính của PC vẫn hoạt động bình thường.",
            Self::Russian => "Транслирует звук только выбранного устройства; динамики ПК активны.",
            Self::Spanish => "Solo transmite el audio del dispositivo seleccionado; los altavoces del PC siguen activos.",
        }
    }

    pub fn btn_open_windows_mixer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚙ 開啟 Windows 音量混音程式 (設定軟體輸出)",
            Self::SimplifiedChinese => "⚙ 打开 Windows 音量合成器 (设置应用输出)",
            Self::English => "⚙ Open Windows Volume Mixer (App Output Settings)",
            Self::French => "⚙ Ouvrir le mélangeur de volume Windows",
            Self::German => "⚙ Windows-Lautstärkemixer öffnen",
            Self::Italian => "⚙ Apri mixer volume di Windows",
            Self::Japanese => "⚙ Windows 音量ミキサーを開く (アプリ出力先設定)",
            Self::Korean => "⚙ Windows 볼륨 믹서 열기 (앱 출력 설정)",
            Self::PortugueseBrazil => "⚙ Abrir Mixer de Volume do Windows",
            Self::PortuguesePortugal => "⚙ Abrir Misturador de Volume do Windows (Definições de saída de aplicações)",
            Self::Indonesian => "⚙ Buka Mixer Volume Windows (Pengaturan Output Aplikasi)",
            Self::Vietnamese => "⚙ Mở Bộ trộn âm lượng Windows (Cài đặt đầu ra ứng dụng)",
            Self::Russian => "⚙ Открыть микшер громкости Windows",
            Self::Spanish => "⚙ Abrir Mezclador de volumen de Windows",
        }
    }

    pub fn btn_open_windows_bluetooth(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚙ 開啟 Windows 藍牙與裝置 (須先完成 AirPods 藍牙配對)",
            Self::SimplifiedChinese => "⚙ 打开 Windows 蓝牙和其他设备 (须先完成 AirPods 蓝牙配对)",
            Self::English => "⚙ Open Windows Bluetooth & Devices (Pair AirPods first)",
            Self::French => "⚙ Ouvrir les paramètres Bluetooth et appareils (Jumeler d'abord les AirPods)",
            Self::German => "⚙ Windows Bluetooth- und Geräteeinstellungen öffnen (Zuerst AirPods koppeln)",
            Self::Italian => "⚙ Apri impostazioni Bluetooth e dispositivi Windows (Completare prima l'associazione AirPods)",
            Self::Japanese => "⚙ Windows の Bluetooth とデバイスを開く (事前に AirPods のペアリングを完了してください)",
            Self::Korean => "⚙ Windows Bluetooth 및 장치 설정 열기 (먼저 AirPods 블루투스 페어링 완료 필요)",
            Self::PortugueseBrazil => "⚙ Abrir Bluetooth e dispositivos do Windows (Emparelhe os AirPods primeiro)",
            Self::PortuguesePortugal => "⚙ Abrir Bluetooth e dispositivos do Windows (Emparelhe os AirPods primeiro)",
            Self::Indonesian => "⚙ Buka Bluetooth & Perangkat Windows (Pasangkan AirPods terlebih dahulu)",
            Self::Vietnamese => "⚙ Mở Bluetooth & Thiết bị của Windows (Ghép đôi AirPods trước)",
            Self::Russian => "⚙ Открыть параметры Bluetooth и других устройств Windows (Сначала выполните сопряжение с AirPods)",
            Self::Spanish => "⚙ Abrir Bluetooth y dispositivos de Windows (Emparejar primero los AirPods)",
        }
    }

    pub fn btn_open_windows_mixer_short(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音量混音程式",
            Self::SimplifiedChinese => "音量合成器",
            Self::English => "Volume Mixer",
            Self::French => "Mélangeur",
            Self::German => "Lautstärkemixer",
            Self::Italian => "Mixer volume",
            Self::Japanese => "音量ミキサー",
            Self::Korean => "볼륨 믹서",
            Self::PortugueseBrazil => "Mixer de volume",
            Self::PortuguesePortugal => "Misturador de volume",
            Self::Indonesian => "Mixer Volume",
            Self::Vietnamese => "Bộ trộn âm lượng",
            Self::Russian => "Микшер громкости",
            Self::Spanish => "Mezclador",
        }
    }

    pub fn auto_mute_speakers_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay/AirPods 連線時自動關閉電腦喇叭 (避免重音/雙重發聲)",
            Self::SimplifiedChinese => "AirPlay/AirPods 连接时自动关闭电脑喇叭 (避免重音/双重发声)",
            Self::English => "Mute PC speakers during AirPlay/AirPods playback (prevents duplicate audio / echo)",
            Self::French => "Couper les haut-parleurs du PC lors de la lecture AirPlay/AirPods (évite l'écho)",
            Self::German => "PC-Lautsprecher bei AirPlay/AirPods-Wiedergabe stummschalten (verhindert Echo)",
            Self::Italian => "Disattiva altoparlanti PC durante riproduzione AirPlay/AirPods (evita eco)",
            Self::Japanese => "AirPlay/AirPods 接続時にPCスピーカーを自動ミュート (二重再生・エコー防止)",
            Self::Korean => "AirPlay/AirPods 재생 시 PC 스피커 자동 음소거 (이중 소리 / 에코 방지)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC durante reprodução AirPlay/AirPods (evita eco)",
            Self::PortuguesePortugal => "Cortar som dos altifalantes do PC durante reprodução AirPlay/AirPods (evita som duplicado / eco)",
            Self::Indonesian => "Bisukan speaker PC selama pemutaran AirPlay/AirPods (mencegah audio ganda / gema)",
            Self::Vietnamese => "Tắt tiếng loa PC khi đang phát qua AirPlay/AirPods (chống lặp âm thanh / tiếng vang)",
            Self::Russian => "Отключать динамики ПК при воспроизведении AirPlay/AirPods (без эха)",
            Self::Spanish => "Silenciar altavoces del PC al reproducir AirPlay/AirPods (evita eco)",
        }
    }

    pub fn auto_mute_disabled_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay/AirPods 連線時自動關閉電腦喇叭 (分流模式下主喇叭保持開啟)",
            Self::SimplifiedChinese => "AirPlay/AirPods 连接时自动关闭电脑喇叭 (分流模式下主喇叭保持开启)",
            Self::English => "Mute PC speakers during AirPlay/AirPods playback (Disabled in split mode)",
            Self::French => "Couper les haut-parleurs du PC (Désactivé en mode séparé)",
            Self::German => "PC-Lautsprecher stummschalten (Inaktiv im Split-Modus)",
            Self::Italian => "Disattiva altoparlanti PC (Disabilitato in modalità separata)",
            Self::Japanese => "AirPlay/AirPods 接続時PCスピーカー自動ミュート (個別ルーティング時は無効)",
            Self::Korean => "AirPlay/AirPods 재생 시 PC 스피커 음소거 (개별 라우팅 모드에서는 비활성화)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC (Desativado no modo separado)",
            Self::PortuguesePortugal => "Cortar som dos altifalantes do PC durante reprodução AirPlay/AirPods (Desativado no modo separado)",
            Self::Indonesian => "Bisukan speaker PC selama pemutaran AirPlay/AirPods (Dinonaktifkan dalam mode terpisah)",
            Self::Vietnamese => "Tắt tiếng loa PC khi phát AirPlay/AirPods (Bị tắt trong chế độ tách luồng)",
            Self::Russian => "Отключать динамики ПК (Недоступно в режиме разделения)",
            Self::Spanish => "Silenciar altavoces del PC (Desactivado en modo separado)",
        }
    }

    pub fn auto_mute_disabled_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "已選擇特定裝置分流，主喇叭維持正常運作，以利聆聽系統音效、遊戲與語音。",
            Self::SimplifiedChinese => "已选择特定设备分流，主喇叭维持正常运作，以便收听系统音效、游戏与语音。",
            Self::English => "Specific device routing active; main speakers remain enabled for system sounds and voice.",
            Self::French => "Routage spécifique actif ; les haut-parleurs principaux restent activés.",
            Self::German => "Geräterouting aktiv; Hauptlautsprecher bleiben für Systemtöne aktiviert.",
            Self::Italian => "Routing specifico attivo; gli altoparlanti principali rimangono attivi.",
            Self::Japanese => "個別ルーティング中：システム音や通話を聞くためPCスピーカーは鳴り続けます。",
            Self::Korean => "특정 장치 라우팅 활성화됨: 시스템 소리 및 음성 대화를 위해 주 스피커가 유지됩니다.",
            Self::PortugueseBrazil => "Roteamento específico ativo; alto-falantes principais continuam ligados.",
            Self::PortuguesePortugal => "Encaminhamento específico ativo; os altifalantes principais continuam ligados para sons de sistema e voz.",
            Self::Indonesian => "Perutean perangkat tertentu aktif; speaker utama tetap aktif untuk suara sistem dan panggilan suara.",
            Self::Vietnamese => "Đang định tuyến thiết bị cụ thể; loa chính vẫn mở cho âm thanh hệ thống và đàm thoại.",
            Self::Russian => "Раздельный вывод активен; динамики ПК продолжают воспроизводить системные звуки.",
            Self::Spanish => "Enrutamiento específico activo; los altavoces principales siguen activos.",
        }
    }

    pub fn auto_mute_locked_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay/AirPods 連線時自動關閉電腦喇叭 🔒 (試用期已結束)",
            Self::SimplifiedChinese => "AirPlay/AirPods 连接时自动关闭电脑喇叭 🔒 (试用期已结束)",
            Self::English => "Auto-mute PC speakers on AirPlay/AirPods connect 🔒 (Trial Expired)",
            Self::French => "Couper les haut-parleurs PC à la connexion AirPlay/AirPods 🔒 (Essai expiré)",
            Self::German => "PC-Lautsprecher bei AirPlay/AirPods-Verbindung stummschalten 🔒 (Testversion abgelaufen)",
            Self::Italian => "Disattiva altoparlanti PC alla connessione AirPlay/AirPods 🔒 (Prova scaduta)",
            Self::Japanese => "AirPlay/AirPods 接続時にPCスピーカーを自動ミュート 🔒 (試用期間終了)",
            Self::Korean => "AirPlay/AirPods 연결 시 PC 스피커 자동 음소거 🔒 (체험 기간 만료)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC ao conectar AirPlay/AirPods 🔒 (Avaliação expirada)",
            Self::PortuguesePortugal => "Cortar som automático dos altifalantes do PC ao ligar AirPlay/AirPods 🔒 (Avaliação expirada)",
            Self::Indonesian => "Otomatis bisukan speaker PC saat AirPlay/AirPods terhubung 🔒 (Uji Coba Berakhir)",
            Self::Vietnamese => "Tự động tắt tiếng loa PC khi kết nối AirPlay/AirPods 🔒 (Hết hạn dùng thử)",
            Self::Russian => "Автозаглушение динамиков ПК при подключении AirPlay/AirPods 🔒 (Пробный период истек)",
            Self::Spanish => "Silenciar altavoces de PC al conectar AirPlay/AirPods 🔒 (Prueba expirada)",
        }
    }

    pub fn auto_mute_locked_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "微軟商店試用期已結束，此功能已鎖定。請購買產品以解鎖自動靜音功能。",
            Self::SimplifiedChinese => "微软商店试用期已结束，此功能已锁定。请购买产品以解锁自动静音功能。",
            Self::English => "Microsoft Store trial has expired. This feature is locked. Please purchase the product to unlock auto-mute.",
            Self::French => "La période d'essai Microsoft Store a expiré. Veuillez acheter le produit pour déverrouiller cette fonctionnalité.",
            Self::German => "Die Microsoft Store-Testversion ist abgelaufen. Bitte kaufen Sie das Produkt, um diese Funktion freizuschalten.",
            Self::Italian => "Il periodo di prova di Microsoft Store è scaduto. Acquista il prodotto per sbloccare questa funzionalità.",
            Self::Japanese => "Microsoft Storeの試用期間が終了しました。自動ミュート機能を有効にするには製品をご購入ください。",
            Self::Korean => "Microsoft Store 체험 기간이 만료되었습니다. 자동 음소거 기능을 사용하려면 제품을 구매해 주세요.",
            Self::PortugueseBrazil => "O período de avaliação da Microsoft Store expirou. Compre o produto para desbloquear este recurso.",
            Self::PortuguesePortugal => "O período de avaliação da Microsoft Store expirou. Esta funcionalidade está bloqueada. Adquira o produto para desbloquear.",
            Self::Indonesian => "Masa uji coba Microsoft Store telah berakhir. Fitur ini terkunci. Silakan beli produk untuk membuka kunci pembisuan otomatis.",
            Self::Vietnamese => "Thời gian dùng thử Microsoft Store đã hết hạn. Tính năng này đã bị khóa. Vui lòng mua bản quyền để mở khóa tính năng tự động tắt tiếng.",
            Self::Russian => "Пробный период Microsoft Store истек. Пожалуйста, приобретите продукт для разблокировки этой функции.",
            Self::Spanish => "El período de prueba de Microsoft Store ha expirado. Compre el producto para desbloquear esta función.",
        }
    }

    pub fn license_trial_expired_banner(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "試用期已結束，此進階功能已鎖定。購買產品後開放此功能。",
            Self::SimplifiedChinese => "试用期已结束，此高级功能已锁定。购买产品后开放此功能。",
            Self::English => "Trial period expired. This feature is locked. Purchase the product to unlock.",
            Self::French => "Période d'essai expirée. Fonctionnalité verrouillée. Achetez le produit pour déverrouiller.",
            Self::German => "Testversion abgelaufen. Funktion gesperrt. Kaufen Sie das Produkt zum Freischalten.",
            Self::Italian => "Periodo di prova scaduto. Funzionalità bloccata. Acquista il prodotto per sbloccare.",
            Self::Japanese => "試用期間が終了しました。製品をご購入いただくとこの機能が開放されます。",
            Self::Korean => "체험 기간이 만료되었습니다. 제품을 구매하시면 이 기능이 활성화됩니다.",
            Self::PortugueseBrazil => "Período de avaliação expirado. Recurso bloqueado. Compre o produto para desbloquear.",
            Self::PortuguesePortugal => "Período de avaliação expirado. Esta funcionalidade está bloqueada. Adquira o produto para desbloquear.",
            Self::Indonesian => "Masa uji coba telah berakhir. Fitur ini terkunci. Beli produk untuk membuka kunci.",
            Self::Vietnamese => "Thời gian dùng thử đã hết hạn. Tính năng này đã bị khóa. Vui lòng mua bản quyền để mở khóa.",
            Self::Russian => "Пробный период истек. Функция заблокирована. Приобретите продукт для разблокировки.",
            Self::Spanish => "Período de prueba expirado. Función bloqueada. Compre el producto para desbloquear.",
        }
    }

    pub fn btn_buy_product(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🛍 前往微軟商店購買產品",
            Self::SimplifiedChinese => "🛍 前往微软商店购买产品",
            Self::English => "🛍 Buy Product in Microsoft Store",
            Self::French => "🛍 Acheter sur le Microsoft Store",
            Self::German => "🛍 Im Microsoft Store kaufen",
            Self::Italian => "🛍 Acquista su Microsoft Store",
            Self::Japanese => "🛍 Microsoft Storeで製品を購入",
            Self::Korean => "🛍 Microsoft Store에서 제품 구매",
            Self::PortugueseBrazil => "🛍 Comprar na Microsoft Store",
            Self::PortuguesePortugal => "🛍 Comprar produto na Microsoft Store",
            Self::Indonesian => "🛍 Beli Produk di Microsoft Store",
            Self::Vietnamese => "🛍 Mua sản phẩm trên Microsoft Store",
            Self::Russian => "🛍 Купить в Microsoft Store",
            Self::Spanish => "🛍 Comprar en Microsoft Store",
        }
    }

    pub fn btn_buy_short(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "購買",
            Self::SimplifiedChinese => "购买",
            Self::English => "Buy",
            Self::French => "Acheter",
            Self::German => "Kaufen",
            Self::Italian => "Acquista",
            Self::Japanese => "購入",
            Self::Korean => "구매",
            Self::PortugueseBrazil => "Comprar",
            Self::PortuguesePortugal => "Comprar",
            Self::Indonesian => "Beli",
            Self::Vietnamese => "Mua ngay",
            Self::Russian => "Купить",
            Self::Spanish => "Comprar",
        }
    }

    pub fn license_trial_badge(&self, days: u32) -> String {
        match self {
            Self::TraditionalChinese => format!("(試用剩餘 {days} 天)"),
            Self::SimplifiedChinese => format!("(试用剩余 {days} 天)"),
            Self::English => format!("(Trial: {days}d left)"),
            Self::French => format!("(Essai : {days}j restants)"),
            Self::German => format!("(Testversion: noch {days}T)"),
            Self::Italian => format!("(Prova: {days}g rimanenti)"),
            Self::Japanese => format!("(試用残り {days}日)"),
            Self::Korean => format!("(체험 잔여 {days}일)"),
            Self::PortugueseBrazil => format!("(Avaliação: {days}d restantes)"),
            Self::PortuguesePortugal => format!("(Avaliação: {days}d restantes)"),
            Self::Indonesian => format!("(Uji Coba: sisa {days}hr)"),
            Self::Vietnamese => format!("(Dùng thử: còn {days} ngày)"),
            Self::Russian => format!("(Пробный: ост. {days}д)"),
            Self::Spanish => format!("(Prueba: {days}d restantes)"),
        }
    }

    pub fn license_trial_days_remaining(&self, days: u32) -> String {
        match self {
            Self::TraditionalChinese => format!("試用中 (剩餘 {days} 天)"),
            Self::SimplifiedChinese => format!("试用中 (剩余 {days} 天)"),
            Self::English => format!("Trial Active ({days} days remaining)"),
            Self::French => format!("Essai actif ({days} jours restants)"),
            Self::German => format!("Testversion aktiv (noch {days} Tage)"),
            Self::Italian => format!("Prova attiva ({days} giorni rimanenti)"),
            Self::Japanese => format!("試用中 (残り {days} 日)"),
            Self::Korean => format!("체험판 사용 중 (남은 기간 {days}일)"),
            Self::PortugueseBrazil => format!("Avaliação ativa ({days} dias restantes)"),
            Self::PortuguesePortugal => format!("Avaliação ativa ({days} dias restantes)"),
            Self::Indonesian => format!("Uji Coba Aktif (sisa {days} hari)"),
            Self::Vietnamese => format!("Bản dùng thử đang hoạt động (còn {days} ngày)"),
            Self::Russian => format!("Пробный период (осталось {days} дн.)"),
            Self::Spanish => format!("Prueba activa ({days} días restantes)"),
        }
    }

    pub fn license_status_full(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "正式授權版 ✅",
            Self::SimplifiedChinese => "正式授权版 ✅",
            Self::English => "Full Licensed Version ✅",
            Self::French => "Version avec licence complète ✅",
            Self::German => "Vollversion lizenziert ✅",
            Self::Italian => "Versione con licenza completa ✅",
            Self::Japanese => "製品版ライセンス ✅",
            Self::Korean => "정식 라이선스 버전 ✅",
            Self::PortugueseBrazil => "Versão com licença completa ✅",
            Self::PortuguesePortugal => "Versão com licença completa ✅",
            Self::Indonesian => "Versi Berlisensi Penuh ✅",
            Self::Vietnamese => "Phiên bản đầy đủ bản quyền ✅",
            Self::Russian => "Полная лицензионная версия ✅",
            Self::Spanish => "Versión con licencia completa ✅",
        }
    }

    pub fn license_status_expired(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "試用期已結束 🔒",
            Self::SimplifiedChinese => "试用期已结束 🔒",
            Self::English => "Trial Expired 🔒",
            Self::French => "Essai expiré 🔒",
            Self::German => "Testversion abgelaufen 🔒",
            Self::Italian => "Prova scaduta 🔒",
            Self::Japanese => "試用期間終了 🔒",
            Self::Korean => "체험 기간 만료 🔒",
            Self::PortugueseBrazil => "Avaliação expirada 🔒",
            Self::PortuguesePortugal => "Avaliação expirada 🔒",
            Self::Indonesian => "Uji Coba Berakhir 🔒",
            Self::Vietnamese => "Hết hạn dùng thử 🔒",
            Self::Russian => "Пробный период истек 🔒",
            Self::Spanish => "Prueba expirada 🔒",
        }
    }

    pub fn license_status_unpackaged(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "本機獨立執行檔 (Unpackaged)",
            Self::SimplifiedChinese => "本机独立执行档 (Unpackaged)",
            Self::English => "Standalone Executable (Unpackaged)",
            Self::French => "Exécutable autonome (Non empaqueté)",
            Self::German => "Eigenständige Ausführungsdatei (Unpackaged)",
            Self::Italian => "Eseguibile autonomo (Non pacchettizzato)",
            Self::Japanese => "スタンドアロン実行 (Unpackaged)",
            Self::Korean => "독립 실행 파일 (Unpackaged)",
            Self::PortugueseBrazil => "Executável independente (Desempacotado)",
            Self::PortuguesePortugal => "Executável autónomo (Não empacotado)",
            Self::Indonesian => "Eksekusi Mandiri (Tanpa Paket)",
            Self::Vietnamese => "Bản thực thi độc lập (Chưa đóng gói)",
            Self::Russian => "Автономный исполняемый файл (Unpackaged)",
            Self::Spanish => "Ejecutable independiente (Desempaquetado)",
        }
    }

    pub fn footer_trial_remaining(&self, days: u32) -> String {
        match self {
            Self::TraditionalChinese => format!("⏳ 試用期剩餘 {days} 天"),
            Self::SimplifiedChinese => format!("⏳ 试用期剩余 {days} 天"),
            Self::English => format!("⏳ Trial: {days} days left"),
            Self::French => format!("⏳ Essai : {days} jours restants"),
            Self::German => format!("⏳ Testversion: Noch {days} Tage"),
            Self::Italian => format!("⏳ Prova: {days} giorni rimasti"),
            Self::Japanese => format!("⏳ 試用期間：残り {days} 日"),
            Self::Korean => format!("⏳ 평가판: {days}일 남음"),
            Self::PortugueseBrazil => format!("⏳ Avaliação: restam {days} dias"),
            Self::PortuguesePortugal => format!("⏳ Avaliação: restam {days} dias"),
            Self::Indonesian => format!("⏳ Uji Coba: sisa {days} hari"),
            Self::Vietnamese => format!("⏳ Dùng thử: còn {days} ngày"),
            Self::Russian => format!("⏳ Пробный период: осталось {days} дн."),
            Self::Spanish => format!("⏳ Prueba: quedan {days} días"),
        }
    }

    pub fn footer_btn_buy_full(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⭐ 購買正式版 ➜",
            Self::SimplifiedChinese => "⭐ 购买正式版 ➜",
            Self::English => "⭐ Buy Full Version ➜",
            Self::French => "⭐ Acheter la version complète ➜",
            Self::German => "⭐ Vollversion kaufen ➜",
            Self::Italian => "⭐ Acquista versione completa ➜",
            Self::Japanese => "⭐ 製品版を購入 ➜",
            Self::Korean => "⭐ 정식 버전 구매 ➜",
            Self::PortugueseBrazil => "⭐ Comprar versão completa ➜",
            Self::PortuguesePortugal => "⭐ Comprar versão completa ➜",
            Self::Indonesian => "⭐ Beli Versi Penuh ➜",
            Self::Vietnamese => "⭐ Mua phiên bản đầy đủ ➜",
            Self::Russian => "⭐ Купить полную версию ➜",
            Self::Spanish => "⭐ Comprar versión completa ➜",
        }
    }

    pub fn footer_trial_expired(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚠️ 試用期已結束，進階功能已鎖定",
            Self::SimplifiedChinese => "⚠️ 试用期已结束，高级功能已锁定",
            Self::English => "⚠️ Trial expired, premium features locked",
            Self::French => "⚠️ Essai expiré, fonctionnalités avancées verrouillées",
            Self::German => "⚠️ Testversion abgelaufen, Premium-Funktionen gesperrt",
            Self::Italian => "⚠️ Prova scaduta, funzionalità avanzate bloccate",
            Self::Japanese => "⚠️ 試用期間が終了し、上位機能が制限されています",
            Self::Korean => "⚠️ 평가판 기간이 만료되어 고급 기능이 잠겼습니다",
            Self::PortugueseBrazil => "⚠️ Avaliação expirada, recursos premium bloqueados",
            Self::PortuguesePortugal => "⚠️ Avaliação expirada, funcionalidades premium bloqueadas",
            Self::Indonesian => "⚠️ Masa uji coba berakhir, fitur premium terkunci",
            Self::Vietnamese => "⚠️ Bản dùng thử đã hết hạn, các tính năng cao cấp bị khóa",
            Self::Russian => "⚠️ Пробный период истек, функции заблокированы",
            Self::Spanish => "⚠️ Prueba finalizada, funciones avanzadas bloqueadas",
        }
    }

    pub fn footer_btn_unlock_now(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔓 立即購買解鎖",
            Self::SimplifiedChinese => "🔓 立即购买解锁",
            Self::English => "🔓 Unlock Full Version",
            Self::French => "🔓 Débloquer la version complète",
            Self::German => "🔓 Jetzt freischalten",
            Self::Italian => "🔓 Sblocca ora",
            Self::Japanese => "🔓 今すぐ購入して解除",
            Self::Korean => "🔓 지금 구매하고 잠금 해제",
            Self::PortugueseBrazil => "🔓 Desbloquear versão completa",
            Self::PortuguesePortugal => "🔓 Desbloquear versão completa",
            Self::Indonesian => "🔓 Buka Kunci Versi Penuh",
            Self::Vietnamese => "🔓 Mở khóa phiên bản đầy đủ",
            Self::Russian => "🔓 Разблокировать версию",
            Self::Spanish => "🔓 Desbloquear versión completa",
        }
    }

    pub fn footer_trial_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "感謝體驗 Win2Pod Bridge！試用期間享有完整功能，點擊右側按鈕可隨時前往微軟商店升級正式版。",
            Self::SimplifiedChinese => "感谢体验 Win2Pod Bridge！试用期间享有完整功能，点击右侧按钮可随时前往微软商店升级正式版。",
            Self::English => "Thank you for trying Win2Pod Bridge! All features are unlocked during the trial. Click the button to buy the full version in Microsoft Store anytime.",
            Self::French => "Merci d'essayer Win2Pod Bridge ! Toutes les fonctionnalités sont disponibles pendant l'essai. Cliquez sur le bouton pour acheter la version complète sur le Microsoft Store.",
            Self::German => "Vielen Dank fürs Ausprobieren von Win2Pod Bridge! Während der Testphase sind alle Funktionen freigeschaltet. Klicken Sie rechts, um die Vollversion im Microsoft Store zu kaufen.",
            Self::Italian => "Grazie per aver provato Win2Pod Bridge! Tutte le funzionalità sono sbloccate durante la prova. Fai clic sul pulsante per acquistare la versione completa su Microsoft Store.",
            Self::Japanese => "Win2Pod Bridge をお試しいただきありがとうございます！試用期間中は全機能をご利用いただけます。右側のボタンからいつでも Microsoft Store で製品版を購入できます。",
            Self::Korean => "Win2Pod Bridge를 이용해 주셔서 감사합니다! 평가판 기간 동안 모든 기능을 이용할 수 있습니다. 오른쪽 버튼을 클릭하여 언제든지 Microsoft Store에서 정식 버전을 구매하세요.",
            Self::PortugueseBrazil => "Obrigado por experimentar o Win2Pod Bridge! Todos os recursos estão disponíveis durante o teste. Clique no botão para comprar a versão completa na Microsoft Store a qualquer momento.",
            Self::PortuguesePortugal => "Obrigado por experimentar o Win2Pod Bridge! Todas as funcionalidades estão disponíveis durante o teste. Clique no botão para comprar a versão completa na Microsoft Store a qualquer momento.",
            Self::Indonesian => "Terima kasih telah mencoba Win2Pod Bridge! Semua fitur tidak terkunci selama masa uji coba. Klik tombol untuk membeli versi penuh di Microsoft Store kapan saja.",
            Self::Vietnamese => "Cảm ơn bạn đã dùng thử Win2Pod Bridge! Tất cả tính năng đều được mở khóa trong thời gian dùng thử. Nhấp vào nút để mua phiên bản đầy đủ trên Microsoft Store bất kỳ lúc nào.",
            Self::Russian => "Спасибо за использование Win2Pod Bridge! В пробный период доступны все функции. Нажмите кнопку справа, чтобы приобрести полную версию в Microsoft Store.",
            Self::Spanish => "¡Gracias por probar Win2Pod Bridge! Todas las funciones están desbloqueadas durante la prueba. Haga clic en el botón para comprar la versión completa en Microsoft Store en cualquier momento.",
        }
    }

    pub fn about_store_status_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "授權狀態：",
            Self::SimplifiedChinese => "授权状态：",
            Self::English => "License Status:",
            Self::French => "Statut de licence :",
            Self::German => "Lizenzstatus:",
            Self::Italian => "Stato licenza:",
            Self::Japanese => "ライセンス状態：",
            Self::Korean => "라이선스 상태:",
            Self::PortugueseBrazil => "Status da licença:",
            Self::PortuguesePortugal => "Estado da licença:",
            Self::Indonesian => "Status Lisensi:",
            Self::Vietnamese => "Trạng thái bản quyền:",
            Self::Russian => "Статус лицензии:",
            Self::Spanish => "Estado de licencia:",
        }
    }

    pub fn license_test_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🧪 模擬測試：",
            Self::SimplifiedChinese => "🧪 模拟测试：",
            Self::English => "🧪 Test Mode:",
            Self::French => "🧪 Mode test :",
            Self::German => "🧪 Testmodus:",
            Self::Italian => "🧪 Modalità test:",
            Self::Japanese => "🧪 テスト切替：",
            Self::Korean => "🧪 테스트 모드:",
            Self::PortugueseBrazil => "🧪 Modo de teste:",
            Self::PortuguesePortugal => "🧪 Modo de teste:",
            Self::Indonesian => "🧪 Mode Uji:",
            Self::Vietnamese => "🧪 Chế độ thử nghiệm:",
            Self::Russian => "🧪 Тестовый режим:",
            Self::Spanish => "🧪 Modo de prueba:",
        }
    }

    pub fn license_test_btn_expired(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "試用結束 (鎖定功能)",
            Self::SimplifiedChinese => "试用结束 (锁定功能)",
            Self::English => "Trial Expired (Locked)",
            Self::French => "Essai expiré (Verrouillé)",
            Self::German => "Testversion abgelaufen (Gesperrt)",
            Self::Italian => "Prova scaduta (Bloccato)",
            Self::Japanese => "試用終了 (機能ロック)",
            Self::Korean => "체험 만료 (기능 잠금)",
            Self::PortugueseBrazil => "Avaliação expirada (Bloqueado)",
            Self::PortuguesePortugal => "Avaliação expirada (Bloqueado)",
            Self::Indonesian => "Uji Coba Berakhir (Terkunci)",
            Self::Vietnamese => "Hết hạn dùng thử (Đã khóa)",
            Self::Russian => "Период истек (Блокировка)",
            Self::Spanish => "Prueba expirada (Bloqueado)",
        }
    }

    pub fn license_test_btn_trial(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "試用中 (剩餘7天)",
            Self::SimplifiedChinese => "试用中 (剩余7天)",
            Self::English => "Active Trial (7 days)",
            Self::French => "Essai actif (7 jours)",
            Self::German => "Testversion aktiv (7 Tage)",
            Self::Italian => "Prova attiva (7 giorni)",
            Self::Japanese => "試用中 (残り7日)",
            Self::Korean => "체험판 (남은 7일)",
            Self::PortugueseBrazil => "Avaliação ativa (7 dias)",
            Self::PortuguesePortugal => "Avaliação ativa (7 dias)",
            Self::Indonesian => "Uji Coba Aktif (7 hari)",
            Self::Vietnamese => "Bản dùng thử đang hoạt động (7 ngày)",
            Self::Russian => "Пробный период (7 дней)",
            Self::Spanish => "Prueba activa (7 días)",
        }
    }

    pub fn license_test_btn_full(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "正式版",
            Self::SimplifiedChinese => "正式版",
            Self::English => "Full Version",
            Self::French => "Version complète",
            Self::German => "Vollversion",
            Self::Italian => "Versione completa",
            Self::Japanese => "製品版",
            Self::Korean => "정식 버전",
            Self::PortugueseBrazil => "Versão completa",
            Self::PortuguesePortugal => "Versão completa",
            Self::Indonesian => "Versi Penuh",
            Self::Vietnamese => "Phiên bản đầy đủ",
            Self::Russian => "Полная версия",
            Self::Spanish => "Versión completa",
        }
    }

    pub fn license_test_btn_reset(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "重設 (未打包)",
            Self::SimplifiedChinese => "重置 (未打包)",
            Self::English => "Reset (Unpackaged)",
            Self::French => "Réinitialiser",
            Self::German => "Zurücksetzen",
            Self::Italian => "Ripristina",
            Self::Japanese => "リセット",
            Self::Korean => "재설정 (미패키징)",
            Self::PortugueseBrazil => "Redefinir",
            Self::PortuguesePortugal => "Repor (Não empacotado)",
            Self::Indonesian => "Atur Ulang (Tanpa Paket)",
            Self::Vietnamese => "Đặt lại (Chưa đóng gói)",
            Self::Russian => "Сброс",
            Self::Spanish => "Restablecer",
        }
    }

    pub fn prevent_sleep_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "串流時抑制系統睡眠 (允許關閉螢幕省電)",
            Self::SimplifiedChinese => "串流时抑制系统睡眠 (允许关闭屏幕省电)",
            Self::English => "Prevent system sleep during streaming (display may turn off)",
            Self::French => "Empêcher la veille pendant la lecture (l'écran peut s'éteindre)",
            Self::German => "Ruhezustand während Streaming verhindern (Bildschirm darf ausgehen)",
            Self::Italian => "Impedisci sospensione durante lo streaming (lo schermo può spegnersi)",
            Self::Japanese => "ストリーミング中のPCスリープを防止 (画面OFFは許可)",
            Self::Korean => "스트리밍 중 시스템 절전 모드 방지 (화면 꺼짐 허용)",
            Self::PortugueseBrazil => "Prevenir suspensão do sistema ao transmitir (tela pode desligar)",
            Self::PortuguesePortugal => "Prevenir suspensão do sistema durante a transmissão (o ecrã pode desligar)",
            Self::Indonesian => "Cegah tidur sistem selama streaming (layar dapat mati)",
            Self::Vietnamese => "Ngăn hệ thống vào chế độ ngủ khi đang truyền phát (màn hình có thể tắt)",
            Self::Russian => "Запретить спящий режим при трансляции (экран может выключаться)",
            Self::Spanish => "Evitar suspensión del sistema durante reproducción (la pantalla puede apagarse)",
        }
    }

    pub fn prevent_sleep_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟：串流中防止電腦進入休眠，螢幕關閉時音樂持續播放。\n關閉：電腦閒置達到系統設定時間後自動休眠省電。",
            Self::SimplifiedChinese => "开启：串流中防止电脑进入休眠，屏幕关闭时音乐持续播放。\n关闭：电脑闲置达到系统设定时间后自动休眠省电。",
            Self::English => "On: Keeps PC awake while streaming; music continues when screen is off.\nOff: PC sleeps according to Windows power plan.",
            Self::French => "Activé : Empêche la mise en veille ; la musique continue écran éteint.\nDésactivé : Mise en veille automatique selon Windows.",
            Self::German => "Ein: Verhindert Ruhezustand beim Streamen; Musik läuft bei ausgeschaltetem Display weiter.\nAus: PC geht planmäßig in den Ruhezustand.",
            Self::Italian => "Attivo: Impedisce la sospensione; la musica continua a schermo spento.\nDisattivo: Sospensione secondo le impostazioni di Windows.",
            Self::Japanese => "ON：再生中のスリープを抑止し画面OFFでも再生継続。\nOFF：Windows設定時間後に通常通りスリープ。",
            Self::Korean => "켜짐: 스트리밍 중 PC 절전 방지, 화면이 꺼져도 음악 재생 지속.\n꺼짐: Windows 설정에 따라 절전 모드로 진입.",
            Self::PortugueseBrazil => "Ligado: Impede a suspensão ao transmitir; música continua com a tela desligada.\nDesligado: Suspende conforme o plano do Windows.",
            Self::PortuguesePortugal => "Ligado: Mantém o PC ativo durante a transmissão; a música continua com o ecrã desligado.\nDesligado: O PC suspende de acordo com o plano do Windows.",
            Self::Indonesian => "Aktif: Menjaga PC tetap bangun saat streaming; musik berlanjut saat layar mati.\nNonaktif: PC tidur sesuai dengan paket daya Windows.",
            Self::Vietnamese => "Bật: Giữ máy tính hoạt động khi đang truyền phát; âm nhạc tiếp tục phát ngay cả khi tắt màn hình.\nTắt: Máy tính sẽ ngủ theo cài đặt nguồn của Windows.",
            Self::Russian => "Вкл: Запрещает сон при трансляции; музыка играет при погасшем экране.\nВыкл: Обычный спящий режим по настройкам Windows.",
            Self::Spanish => "Activado: Evita la suspensión al reproducir; la música continúa con la pantalla apagada.\nDesactivado: Suspende según el plan de energía.",
        }
    }

    pub fn power_badge_tooltip(&self, state: crate::power::PowerState) -> &'static str {
        match state {
            crate::power::PowerState::AwakePresentation => match self {
                Self::TraditionalChinese => "全螢幕視覺化中 (紅色警示)：阻止系統休眠且螢幕保持恆亮",
                Self::SimplifiedChinese => "全屏可视化中 (红色警示)：阻止系统休眠且屏幕保持恒亮",
                _ => "Fullscreen visualizer (Red): keeps PC awake and display on",
            },
            crate::power::PowerState::AwakeStreaming => match self {
                Self::TraditionalChinese => "音訊播放中 (黃色)：阻止系統休眠，螢幕可依照系統設定自動關閉省電",
                Self::SimplifiedChinese => "音频播放中 (黄色)：阻止系统休眠，屏幕可依照系统设置自动关闭省电",
                _ => "Audio streaming (Yellow): keeps PC awake, display can turn off to save power",
            },
            crate::power::PowerState::EcoSilent => match self {
                Self::TraditionalChinese => "靜音超過 30 秒 (綠色 ECO)：最省電模式，已釋放防休眠鎖，電腦可正常休眠",
                Self::SimplifiedChinese => "静音超过 30 秒 (绿色 ECO)：最省电模式，已释放防休眠锁，电脑可正常休眠",
                _ => "Silent >30s (Green ECO): maximum power saving, sleep lock released",
            },
            crate::power::PowerState::Standby => match self {
                Self::TraditionalChinese => "待命中 (白色)：未啟用休眠抑制，不參與決定電源設定",
                Self::SimplifiedChinese => "待命中 (白色)：未启用休眠抑制，不参与决定电源设置",
                _ => "Standby (White): sleep prevention inactive",
            },
            crate::power::PowerState::Suspended => match self {
                Self::TraditionalChinese => "系統休眠中",
                Self::SimplifiedChinese => "系统休眠中",
                _ => "System suspended",
            },
        }
    }

    pub fn hint_sleep_active(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🟢 偵測到音訊串流中，系統睡眠已暫時抑制 (螢幕可關閉省電)。",
            Self::SimplifiedChinese => "🟢 检测到音频串流中，系统睡眠已暂时抑制 (屏幕可关闭省电)。",
            Self::English => "🟢 Audio streaming active; system sleep temporarily suppressed (display can sleep).",
            Self::French => "🟢 Diffusion audio en cours ; veille système temporairement suspendue.",
            Self::German => "🟢 Audiostreaming aktiv; Ruhezustand vorübergehend unterdrückt.",
            Self::Italian => "🟢 Streaming audio attivo; sospensione del sistema temporaneamente inibita.",
            Self::Japanese => "🟢 音声ストリーミング中：スリープ一時抑止中 (画面OFF可)。",
            Self::Korean => "🟢 오디오 스트리밍 중: 시스템 절전 일시 차단됨 (화면 꺼짐 가능).",
            Self::PortugueseBrazil => "🟢 Transmissão de áudio ativa; suspensão do sistema temporariamente desativada.",
            Self::PortuguesePortugal => "🟢 Transmissão de áudio ativa; suspensão do sistema temporariamente desativada (o ecrã pode suspender).",
            Self::Indonesian => "🟢 Streaming audio aktif; tidur sistem dinonaktifkan sementara (layar dapat tidur).",
            Self::Vietnamese => "🟢 Đang truyền phát âm thanh; tạm dừng chế độ ngủ hệ thống (màn hình vẫn có thể ngủ).",
            Self::Russian => "🟢 Трансляция активна; спящий режим временно заблокирован.",
            Self::Spanish => "🟢 Transmisión de audio activa; suspensión del sistema bloqueada temporalmente.",
        }
    }

    pub fn hint_sleep_idle(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚪ 目前無音樂訊號 (超過 30 秒)，已自動釋放睡眠鎖允許電腦休眠。",
            Self::SimplifiedChinese => "⚪ 目前无音乐信号 (超过 30 秒)，已自动释放睡眠锁允许电脑休眠。",
            Self::English => "⚪ No audio signal for 30s; sleep lock released to allow PC sleep.",
            Self::French => "⚪ Aucun signal audio depuis 30s ; verrou de veille libéré.",
            Self::German => "⚪ Kein Audiosignal für 30s; Ruhezustand wieder erlaubt.",
            Self::Italian => "⚪ Nessun segnale audio da 30s; blocco sospensione rilasciato.",
            Self::Japanese => "⚪ 30秒以上無音のため、スリープ抑止を解除しました。",
            Self::Korean => "⚪ 30초 이상 오디오 신호 없음: 절전 차단 해제됨.",
            Self::PortugueseBrazil => "⚪ Sem sinal de áudio há 30s; bloqueio de suspensão liberado.",
            Self::PortuguesePortugal => "⚪ Sem sinal de áudio há 30s; bloqueio de suspensão libertado para permitir a suspensão do PC.",
            Self::Indonesian => "⚪ Tidak ada sinyal audio selama 30 detik; kunci tidur dilepas untuk memungkinkan PC tidur.",
            Self::Vietnamese => "⚪ Không có tín hiệu âm thanh trong 30 giây; đã giải phóng khóa ngủ để máy tính có thể ngủ bình thường.",
            Self::Russian => "⚪ Нет звука более 30 сек; блокировка сна снята.",
            Self::Spanish => "⚪ Sin señal de audio durante 30s; bloqueo de suspensión liberado.",
        }
    }

    pub fn hint_sleep_muted(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚪ 目前已啟用靜音，已自動釋放睡眠鎖允許電腦休眠。",
            Self::SimplifiedChinese => "⚪ 目前已启用静音，已自动释放睡眠锁允许电脑休眠。",
            Self::English => "⚪ Audio is muted; sleep lock released to allow PC sleep.",
            Self::French => "⚪ Audio en sourdine ; verrou de veille libéré.",
            Self::German => "⚪ Audio stummgeschaltet; Ruhezustand erlaubt.",
            Self::Italian => "⚪ Audio disattivato; blocco sospensione rilasciato.",
            Self::Japanese => "⚪ 消音中のため、スリープ抑止を解除しました。",
            Self::Korean => "⚪ 음소거 상태: 절전 차단 해제됨.",
            Self::PortugueseBrazil => "⚪ Áudio silenciado; bloqueio de suspensão liberado.",
            Self::PortuguesePortugal => "⚪ Áudio sem som; bloqueio de suspensão libertado para permitir a suspensão do PC.",
            Self::Indonesian => "⚪ Audio dibisukan; kunci tidur dilepas untuk memungkinkan PC tidur.",
            Self::Vietnamese => "⚪ Âm thanh đã tắt tiếng; đã giải phóng khóa ngủ để máy tính có thể ngủ.",
            Self::Russian => "⚪ Звук выключен; блокировка сна снята.",
            Self::Spanish => "⚪ Audio silenciado; bloqueo de suspensión liberado.",
        }
    }

    pub fn hint_sleep_streaming_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 串流播放時抑制睡眠且音樂不中斷；無音樂逾時自動允許休眠。",
            Self::SimplifiedChinese => "💡 串流播放时抑制睡眠且音乐不中断；无音乐超时自动允许休眠。",
            Self::English => "💡 Suppresses sleep during playback; allows sleep when audio is idle.",
            Self::French => "💡 Empêche la veille pendant la lecture ; autorise la veille en cas d'inactivité.",
            Self::German => "💡 Unterdrückt Ruhezustand beim Abspielen; erlaubt Ruhezustand bei Inaktivität.",
            Self::Italian => "💡 Inibisce la sospensione in riproduzione; la consente in assenza di audio.",
            Self::Japanese => "💡 再生中はスリープを防止、無音が続くと自動でスリープを許可します。",
            Self::Korean => "💡 재생 중 절전 방지, 무음 시 자동으로 절전 허용.",
            Self::PortugueseBrazil => "💡 Suprime a suspensão durante a reprodução; permite suspensão quando inativo.",
            Self::PortuguesePortugal => "💡 Suprime a suspensão durante a reprodução; permite a suspensão quando inativo.",
            Self::Indonesian => "💡 Menekan tidur selama pemutaran; memungkinkan tidur saat audio menganggur.",
            Self::Vietnamese => "💡 Ngăn máy tính ngủ khi đang phát; cho phép ngủ bình thường khi không có âm thanh.",
            Self::Russian => "💡 Блокирует сон при воспроизведении; разрешает при отсутствии звука.",
            Self::Spanish => "💡 Bloquea la suspensión al reproducir; la permite cuando no hay audio.",
        }
    }

    pub fn hint_sleep_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚠ 電腦閒置達到系統設定時間後將自動休眠並暫停串流。",
            Self::SimplifiedChinese => "⚠ 电脑闲置达到系统设定时间后将自动休眠并暂停串流。",
            Self::English => "⚠ PC will sleep after idle timeout, pausing audio stream.",
            Self::French => "⚠ Le PC se mettra en veille après expiration du délai, suspendant la lecture.",
            Self::German => "⚠ PC wechselt nach Inaktivität in den Ruhezustand und pausiert das Streaming.",
            Self::Italian => "⚠ Il PC andrà in sospensione per inattività, interrompendo lo streaming.",
            Self::Japanese => "⚠ アイドル時間が設定に達するとPCがスリープし、配信が中断されます。",
            Self::Korean => "⚠ 유휴 시간 초과 시 PC가 절전 모드로 전환되어 스트리밍이 일시 정지됩니다.",
            Self::PortugueseBrazil => "⚠ O PC suspenderá após tempo ocioso, pausando a transmissão.",
            Self::PortuguesePortugal => "⚠ O PC suspenderá após o tempo de inatividade, pausando a transmissão de áudio.",
            Self::Indonesian => "⚠ PC akan tidur setelah batas waktu menganggur, menjeda streaming audio.",
            Self::Vietnamese => "⚠ Máy tính sẽ ngủ sau khi hết thời gian chờ, làm tạm dừng luồng âm thanh.",
            Self::Russian => "⚠ ПК перейдет в спящий режим по таймеру, прервав трансляцию.",
            Self::Spanish => "⚠ El PC se suspenderá tras el tiempo de inactividad, pausando el streaming.",
        }
    }


    pub fn low_latency_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚡ 低延遲模式 (推薦看影片 / 影音同步)",
            Self::SimplifiedChinese => "⚡ 低延迟模式 (推荐看视频 / 音画同步)",
            Self::English => "⚡ Low Latency Mode (Recommended for video & AV sync)",
            Self::French => "⚡ Mode faible latence (Recommandé pour vidéo / synchro)",
            Self::German => "⚡ Modus für niedrige Latenz (Empfohlen für Videos & AV-Sync)",
            Self::Italian => "⚡ Modalità bassa latenza (Consigliato per video e sincronizzazione)",
            Self::Japanese => "⚡ 低遅延モード (動画視聴・音ズレ軽減推奨)",
            Self::Korean => "⚡ 저지연 모드 (동영상 시청 / 싱크 맞춤 추천)",
            Self::PortugueseBrazil => "⚡ Modo de Baixa Latência (Recomendado para vídeo e sincronia)",
            Self::PortuguesePortugal => "⚡ Modo de Baixa Latência (Recomendado para vídeo e sincronização A/V)",
            Self::Indonesian => "⚡ Mode Latensi Rendah (Disarankan untuk video & sinkronisasi A/V)",
            Self::Vietnamese => "⚡ Chế độ độ trễ thấp (Khuyên dùng cho video & đồng bộ âm thanh/hình ảnh)",
            Self::Russian => "⚡ Режим низкой задержки (Для видео и синхронизации)",
            Self::Spanish => "⚡ Modo de baja latencia (Recomendado para vídeo y sincronización)",
        }
    }

    pub fn low_latency_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟：延遲縮短至約 250~350ms，大幅改善看影片時嘴型與音效落差。\n關閉：延遲約 700~800ms，適用於 WiFi 訊號微弱或易掉包的環境。",
            Self::SimplifiedChinese => "开启：延迟缩短至约 250~350ms，大幅改善看视频时口型与音效落差。\n关闭：延迟约 700~800ms，适用于 WiFi 信号微弱或易丢包的环境。",
            Self::English => "On: Latency reduced to ~250-350ms, greatly improving video lip-sync.\nOff: ~700-800ms buffer, best for weak or unstable Wi-Fi.",
            Self::French => "Activé : Latence ~250-350ms, améliore la synchro labiale vidéo.\nDésactivé : ~700-800ms, idéal pour Wi-Fi instable.",
            Self::German => "Ein: Latenz auf ~250-350ms reduziert für Lippensynchronität bei Videos.\nAus: ~700-800ms Puffer für schwaches Wi-Fi.",
            Self::Italian => "Attivo: Latenza ridotta a ~250-350ms, migliora la sincronizzazione video.\nDisattivo: ~700-800ms per Wi-Fi debole.",
            Self::Japanese => "ON：遅延を約250〜350msに短縮し動画の口ズレを大幅改善。\nOFF：約700〜800msバッファ、不安定なWiFi環境向け。",
            Self::Korean => "켜짐: 지연 시간이 약 250~350ms로 줄어 영상 싱크 대폭 개선.\n꺼짐: 약 700~800ms 버퍼, 불안정한 Wi-Fi 환경에 적합.",
            Self::PortugueseBrazil => "Ligado: Latência reduzida para ~250-350ms para melhor sincronia de vídeo.\nDesligado: Buffer de ~700-800ms para Wi-Fi instável.",
            Self::PortuguesePortugal => "Ligado: Latência reduzida para ~250-350ms, melhorando significativamente a sincronia labial em vídeos.\nDesligado: Buffer de ~700-800ms, ideal para redes Wi-Fi fracas ou instáveis.",
            Self::Indonesian => "Aktif: Latensi berkurang menjadi ~250-350ms, sangat meningkatkan sinkronisasi bibir video.\nNonaktif: Buffer ~700-800ms, terbaik untuk Wi-Fi lemah atau tidak stabil.",
            Self::Vietnamese => "Bật: Độ trễ giảm xuống ~250-350ms, cải thiện đáng kể độ khớp hình tiếng cho video.\nTắt: Bộ đệm ~700-800ms, tối ưu cho mạng Wi-Fi yếu hoặc không ổn định.",
            Self::Russian => "Вкл: Задержка ~250-350мс, улучшает синхронизацию видео и звука.\nВыкл: Буфер ~700-800мс для слабого Wi-Fi.",
            Self::Spanish => "Activado: Reduce la latencia a ~250-350ms, mejorando la sincronía labial.\nDesactivado: Búfer de ~700-800ms para Wi-Fi inestable.",
        }
    }

    pub fn hint_low_latency_on(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 串流與硬體緩衝降至最低；若 WiFi 不穩發生爆音可切換回一般模式。",
            Self::SimplifiedChinese => "💡 串流与硬件缓冲降至最低；若 WiFi 不稳发生爆音可切换回一般模式。",
            Self::English => "💡 Buffer minimized; switch to standard mode if audio crackles occur.",
            Self::French => "💡 Mémoire tampon minimisée ; repassez en mode normal en cas de craquements.",
            Self::German => "💡 Minimaler Puffer; bei Tonaussetzern in den Standardmodus wechseln.",
            Self::Italian => "💡 Buffer ridotto al minimo; torna alla modalità normale in caso di disturbi audio.",
            Self::Japanese => "💡 バッファを最小化中。Wi-Fi不安定でノイズが出る場合は通常モードに切り替えてください。",
            Self::Korean => "💡 버퍼 최소화됨. Wi-Fi 불안정으로 잡음 발생 시 일반 모드로 전환하세요.",
            Self::PortugueseBrazil => "💡 Buffer minimizado; volte ao modo padrão se houver estalos no áudio.",
            Self::PortuguesePortugal => "💡 Buffer minimizado; mude para o modo normal se ocorrerem estalidos no áudio.",
            Self::Indonesian => "💡 Buffer diminimalkan; beralih ke mode standar jika terjadi kresek audio.",
            Self::Vietnamese => "💡 Đã tối thiểu hóa bộ đệm; hãy chuyển sang chế độ tiêu chuẩn nếu âm thanh bị rè/giật.",
            Self::Russian => "💡 Буфер минимален; при щелчках звука переключитесь в обычный режим.",
            Self::Spanish => "💡 Búfer minimizado; cambia al modo estándar si se producen chasquidos de audio.",
        }
    }

    pub fn hint_low_latency_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🛡 高抗干擾模式；適合純音樂聆聽或 WiFi 距離路由器較遠時使用。",
            Self::SimplifiedChinese => "🛡 高抗干扰模式；适合纯音乐聆听或 WiFi 距离路由器较远时使用。",
            Self::English => "🛡 High stability mode; recommended for music listening or distant Wi-Fi.",
            Self::French => "🛡 Mode haute stabilité ; idéal pour la musique ou Wi-Fi éloigné.",
            Self::German => "🛡 Hohe Stabilität; empfohlen für reinen Musikgenuss oder schwaches Wi-Fi.",
            Self::Italian => "🛡 Massima stabilità; consigliato per l'ascolto musicale o Wi-Fi distante.",
            Self::Japanese => "🛡 耐干渉モード：音楽鑑賞やルーターから離れた場所での使用に最適。",
            Self::Korean => "🛡 높은 안정성 모드: 음악 감상 또는 신호가 약한 Wi-Fi 환경에 적합.",
            Self::PortugueseBrazil => "🛡 Modo de alta estabilidade; recomendado para música ou Wi-Fi distante.",
            Self::PortuguesePortugal => "🛡 Modo de alta estabilidade; recomendado para ouvir música ou com Wi-Fi distante.",
            Self::Indonesian => "🛡 Mode stabilitas tinggi; disarankan untuk mendengarkan musik atau Wi-Fi yang jauh.",
            Self::Vietnamese => "🛡 Chế độ ổn định cao; khuyên dùng khi nghe nhạc hoặc kết nối Wi-Fi ở xa.",
            Self::Russian => "🛡 Режим повышенной стабильности; подходит для музыки и дальнего Wi-Fi.",
            Self::Spanish => "🛡 Modo de alta estabilidad; recomendado para música o Wi-Fi distante.",
        }
    }

    pub fn mascot_section_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "👻 視窗幸運物 (Window Mascot)",
            Self::SimplifiedChinese => "👻 窗口吉祥物 (Window Mascot)",
            Self::English => "👻 Window Mascot",
            Self::French => "👻 Mascotte de fenêtre",
            Self::German => "👻 Fenstermaskottchen",
            Self::Italian => "👻 Mascotte della finestra",
            Self::Spanish => "👻 Mascota de la ventana",
            Self::Japanese => "👻 ウィンドウマスコット",
            Self::Korean => "👻 윈도우 마스코트",
            Self::Russian => "👻 Талисман окна",
            _ => "👻 Window Mascot",
        }
    }

    pub fn mascot_enable_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "啟用視窗幸運物漫步 (Window Mascot)",
            Self::SimplifiedChinese => "启用窗口吉祥物漫步 (Window Mascot)",
            Self::English => "Enable Window Mascot Crawler",
            Self::Japanese => "ウィンドウマスコットを有効化",
            _ => "Enable Window Mascot Crawler",
        }
    }

    pub fn mascot_enable_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "在視窗邊界悠閒漫步與轉向巡邏，滑鼠靠近時會調皮捉迷藏",
            Self::SimplifiedChinese => "在窗口边界悠闲漫步与转向巡逻，鼠标靠近时会调皮捉迷藏",
            Self::English => "Patrols along window borders; playfully plays peek-a-boo when cursor approaches",
            Self::Japanese => "ウィンドウの境界をパトロールし、カーソルが近づくと隠れます",
            _ => "Patrols along window borders; playfully plays peek-a-boo when cursor approaches",
        }
    }

    pub fn mascot_rhythm_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎵 播放音樂時隨節奏律動起舞 (Rhythm Dance)",
            Self::SimplifiedChinese => "🎵 播放音乐时随节奏律动起舞 (Rhythm Dance)",
            Self::English => "🎵 Rhythm Dance during music playback",
            Self::Japanese => "🎵 音楽再生時にリズムに合わせてダンス",
            _ => "🎵 Rhythm Dance during music playback",
        }
    }

    pub fn mascot_rhythm_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 串流播放時，幽靈隨音樂重拍點頭跳躍與左右搖擺",
            Self::SimplifiedChinese => "AirPlay 串流播放时，幽灵随音乐重拍点头跳跃与左右摇摆",
            Self::English => "Bobs and grooves along with music beats and audio bass energy during playback",
            Self::Japanese => "AirPlay再生時、ビートに合わせて頷き左右に揺れます",
            _ => "Bobs and grooves along with music beats and audio bass energy during playback",
        }
    }

    pub fn mascot_easter_egg_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡「最喜歡在音樂頻譜頂端衝浪了！聽說在頻譜那邊點右鍵叫我，我會使出大絕招喔～」",
            Self::SimplifiedChinese => "💡「最喜欢在音乐频谱顶端冲浪了！听说在频谱那边点右键叫我，我会使出大绝招喔～」",
            Self::English => "💡 \"Surfing on the spectrum peaks is my favorite! Right-click on the visualizer bars to summon my special moves!\"",
            Self::Japanese => "💡「音楽スペクトラムの頂点で波乗りするのが大好き！スペクトラム上で右クリックすると大技を披露するよ～」",
            _ => "💡 \"Surfing on the spectrum peaks is my favorite! Right-click on the visualizer bars to summon my special moves!\"",
        }
    }

    pub fn mascot_supporter_modal_title_existing(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🌟 致最珍貴的早期支持者與老朋友",
            Self::SimplifiedChinese => "🌟 致最珍贵的早期支持者与老朋友",
            Self::English => "🌟 To Our Dearest Early Supporter & Friend",
            Self::Japanese => "🌟 最愛の初期サポーターと古き友人へ",
            _ => "🌟 To Our Dearest Early Supporter & Friend",
        }
    }

    pub fn mascot_supporter_modal_title_new(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💖 非常感謝您的購買與肯定！",
            Self::SimplifiedChinese => "💖 非常感谢您的购买与肯定！",
            Self::English => "💖 Thank You for Purchasing Win2Pod!",
            Self::Japanese => "💖 ご購入いただき心より感謝申し上げます！",
            _ => "💖 Thank You for Purchasing Win2Pod!",
        }
    }

    pub fn mascot_supporter_modal_body_existing(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "謝謝您一路以來的信任與相伴！有了您的早期支持，Win2Pod 才能持續進化。\n\n為了感謝您的熱情相挺，我和作者特別為您準備了一份正式版專屬的秘密特技：\n\n🎵「在播歌時，對著音樂頻譜按滑鼠右鍵，我就會飛上浪尖為您使出 360° 特技空翻衝浪喔！」",
            Self::SimplifiedChinese => "谢谢您一路以来的信任与相伴！有了您的早期支持，Win2Pod 才能持续进化。\n\n为了感谢您的热情相挺，我和作者特别为您准备了一份正式版专属的秘密特技：\n\n🎵「在播歌时，对着音乐频谱按鼠标右键，我就会飞上浪尖为您使出 360° 特技空翻冲浪喔！」",
            Self::English => "Thank you for your trust and companionship along the way! Your early support keeps Win2Pod evolving.\n\nAs a heartfelt token of gratitude, the developer and I have prepared an exclusive Pro-version secret acrobatic move for you:\n\n🎵 \"During music playback, right-click inside the audio spectrum visualizer, and I will soar up high and perform a 360° air-surf flip for you!\"",
            Self::Japanese => "これまでのご信頼と温かいご愛用に心から感謝いたします！皆様の初期のご支援があったからこそ、Win2Pod は進化を続けることができました。\n\n熱い応援への感謝を込めて、正式版オーナー様限定の秘密のアクロバット技をご用意しました：\n\n🎵「音楽再生中にスペクトラム上で右クリックすると、波の頂点へ飛び上がって360°特技エアサーフ宙返りを披露するよ！」",
            _ => "Thank you for your trust and companionship along the way! Your early support keeps Win2Pod evolving.\n\nAs a heartfelt token of gratitude, the developer and I have prepared an exclusive Pro-version secret acrobatic move for you:\n\n🎵 \"During music playback, right-click inside the audio spectrum visualizer, and I will soar up high and perform a 360° air-surf flip for you!\"",
        }
    }

    pub fn mascot_supporter_modal_body_new(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "歡迎成為正式版主人！我和作者都非常感激您的贊助與肯定，特地為您揭曉一份專屬秘密：\n\n🎵「在播歌時，只要對著音樂頻譜點擊滑鼠右鍵，我就會飛上浪尖使出 360° 特技空翻衝浪喔！」",
            Self::SimplifiedChinese => "欢迎成为正式版主人！我和作者都非常感激您的赞助与肯定，特地为您揭晓一份专属秘密：\n\n🎵「在播歌时，只要对着音乐频谱点击鼠标右键，我就会飞上浪尖使出 360° 特技空翻冲浪喔！」",
            Self::English => "Welcome to the Pro family! The developer and I deeply appreciate your support, and we have a special secret to share with you:\n\n🎵 \"During music playback, right-click inside the audio spectrum visualizer, and I will soar up high and perform a 360° air-surf flip for you!\"",
            Self::Japanese => "正式版オーナーファミリーへようこそ！ご支援に心より感謝申し上げます。感謝を込めて特別な秘密をお教えします：\n\n🎵「音楽再生中にスペクトラム上で右クリックすると、波の頂点へ飛び上がって360°特技エアサーフ宙返りを披露するよ！」",
            _ => "Welcome to the Pro family! The developer and I deeply appreciate your support, and we have a special secret to share with you:\n\n🎵 \"During music playback, right-click inside the audio spectrum visualizer, and I will soar up high and perform a 360° air-surf flip for you!\"",
        }
    }

    pub fn mascot_supporter_modal_btn_existing(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💖 收下這份專屬感謝與秘密",
            Self::SimplifiedChinese => "💖 收下这份专属感谢与秘密",
            Self::English => "💖 Accept with Gratitude & Secrets",
            Self::Japanese => "💖 感謝と秘密を受け取る",
            _ => "💖 Accept with Gratitude & Secrets",
        }
    }

    pub fn mascot_supporter_modal_btn_new(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎵 太棒了！收下這個秘密",
            Self::SimplifiedChinese => "🎵 太棒了！收下这个秘密",
            Self::English => "🎵 Awesome! Got the Secret",
            Self::Japanese => "🎵 すばらしい！秘密を受け取る",
            _ => "🎵 Awesome! Got the Secret",
        }
    }

    pub fn mascot_supporter_replay_btn(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💌 重溫支持者感謝信與秘密",
            Self::SimplifiedChinese => "💌 重温支持者感谢信与秘密",
            Self::English => "💌 Replay Supporter Gratitude & Secret",
            Self::Japanese => "💌 サポーター感謝レターと秘密を再確認",
            _ => "💌 Replay Supporter Gratitude & Secret",
        }
    }

    pub fn mascot_pro_locked_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔒 購買正式版即可解鎖小幽靈 360° 空中特技衝浪彩蛋",
            Self::SimplifiedChinese => "🔒 购买正式版即可解锁小幽灵 360° 空中特技冲浪彩蛋",
            Self::English => "🔒 Purchase full version to unlock mascot 360° air-surf acrobatic easter egg",
            Self::Japanese => "🔒 正式版をご購入いただくとマスコットの360°エアサーフ宙返り技が解放されます",
            _ => "🔒 Purchase full version to unlock mascot 360° air-surf acrobatic easter egg",
        }
    }

    pub fn auto_connect_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🚀 啟動時自動連線上次播放音響",
            Self::SimplifiedChinese => "🚀 启动时自动连接上次播放音响",
            Self::English => "🚀 Auto-connect to last played speaker on launch",
            Self::French => "🚀 Reconnexion automatique au démarrage",
            Self::German => "🚀 Beim Start automatisch mit letztem Lautsprecher verbinden",
            Self::Italian => "🚀 Connessione automatica all'avvio all'ultimo altoparlante",
            Self::Japanese => "🚀 起動時に前回再生したスピーカーへ自動接続",
            Self::Korean => "🚀 시작 시 마지막으로 재생한 스피커 자동 연결",
            Self::PortugueseBrazil => "🚀 Conectar automaticamente ao iniciar",
            Self::PortuguesePortugal => "🚀 Ligar automaticamente ao último altifalante reproduzido ao iniciar",
            Self::Indonesian => "🚀 Hubungkan otomatis ke speaker terakhir yang diputar saat peluncuran",
            Self::Vietnamese => "🚀 Tự động kết nối tới loa phát gần nhất khi khởi động",
            Self::Russian => "🚀 Автоподключение к последней колонке при запуске",
            Self::Spanish => "🚀 Conectar automáticamente al iniciar",
        }
    }

    pub fn auto_connect_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟：程式啟動時自動還原上次播放音路（HomePod 或 本機/螢幕音響）。\n關閉：啟動後維持閒置狀態，等待手動點選音響。",
            Self::SimplifiedChinese => "开启：程序启动时自动还原上次播放音路（HomePod 或 本机/屏幕音响）。\n关闭：启动后维持闲置状态，等待手动点选音响。",
            Self::English => "On: Automatically restores last active audio route on startup (HomePod or local speaker).\nOff: Starts in idle state.",
            Self::French => "Activé : Restaure automatiquement la dernière sortie audio au démarrage (HomePod ou haut-parleur local).\nDésactivé : Reste inactif au démarrage.",
            Self::German => "Ein: Stellt beim Start automatisch die letzte Audio-Route wieder her (HomePod oder lokaler Lautsprecher).\nAus: Startet im Leerlauf.",
            Self::Italian => "Attivo: Ripristina automaticamente l'ultimo percorso audio all'avvio (HomePod o altoparlante locale).\nDisattivo: Rimane inattivo all'avvio.",
            Self::Japanese => "ON：起動時に前回再生した音路（HomePodまたはローカルスピーカー）を自動復元。\nOFF：待機状態で起動し手動選択を待ちます。",
            Self::Korean => "켜짐: 프로그램 시작 시 마지막 오디오 경로(HomePod 또는 로컬 스피커) 자동 복원.\n꺼짐: 대기 상태로 시작하여 수동 연결 대기.",
            Self::PortugueseBrazil => "Ligado: Restaura automaticamente a última rota de áudio na inicialização (HomePod ou alto-falante local).\nDesligado: Inicia em espera.",
            Self::PortuguesePortugal => "Ligado: Restaura automaticamente a última rota de áudio ativa no arranque (HomePod ou altifalante local).\nDesligado: Inicia em estado de espera.",
            Self::Indonesian => "Aktif: Memulihkan rute audio aktif terakhir saat startup otomatis (HomePod atau speaker lokal).\nNonaktif: Mulai dalam keadaan siaga.",
            Self::Vietnamese => "Bật: Tự động khôi phục tuyến âm thanh hoạt động gần nhất khi khởi động (HomePod hoặc loa cục bộ).\nTắt: Khởi động ở trạng thái chờ.",
            Self::Russian => "Вкл: Автоматически восстанавливает последний аудиовыход при старте (HomePod или локальный динамик).\nВыкл: Запуск в режиме ожидания.",
            Self::Spanish => "Activado: Restaura automáticamente la última ruta de audio al iniciar (HomePod o altavoz local).\nDesactivado: Inicia en espera.",
        }
    }

    pub fn auto_connect_target(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("📌 記憶目標：{name}"),
            Self::SimplifiedChinese => format!("📌 记忆目标：{name}"),
            Self::English => format!("📌 Saved Target: {name}"),
            Self::French => format!("📌 Cible mémorisée : {name}"),
            Self::German => format!("📌 Gespeichertes Ziel: {name}"),
            Self::Italian => format!("📌 Destinazione memorizzata: {name}"),
            Self::Japanese => format!("📌 記憶された対象：{name}"),
            Self::Korean => format!("📌 저장된 대상: {name}"),
            Self::PortugueseBrazil => format!("📌 Destino salvo: {name}"),
            Self::PortuguesePortugal => format!("📌 Destino guardado: {name}"),
            Self::Indonesian => format!("📌 Target Tersimpan: {name}"),
            Self::Vietnamese => format!("📌 Mục tiêu đã lưu: {name}"),
            Self::Russian => format!("📌 Запомненная колонка: {name}"),
            Self::Spanish => format!("📌 Objetivo guardado: {name}"),
        }
    }

    pub fn auto_connect_hint_none(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 啟動後自動連線；請先手動連線一次以記憶目標音響。",
            Self::SimplifiedChinese => "💡 启动后自动连接；请先手动连接一次以记忆目标音响。",
            Self::English => "💡 Auto-connect active; please connect once manually to set target.",
            Self::French => "💡 Connexion auto active ; connectez-vous une première fois pour mémoriser l'enceinte.",
            Self::German => "💡 Autoverbindung aktiv; bitte einmal manuell verbinden, um das Ziel zu speichern.",
            Self::Italian => "💡 Connessione auto attiva; connetti manualmente una volta per memorizzare.",
            Self::Japanese => "💡 起動時自動接続：初回は手動で一度接続して対象を記憶させてください。",
            Self::Korean => "💡 자동 연결 켜짐: 대상을 기억하기 위해 먼저 한 번 수동 연결해 주세요.",
            Self::PortugueseBrazil => "💡 Conexão auto ativa; conecte uma vez manualmente para salvar o destino.",
            Self::PortuguesePortugal => "💡 Ligação automática ativa; ligue manualmente uma vez para definir o destino.",
            Self::Indonesian => "💡 Sambung otomatis aktif; silakan hubungkan sekali secara manual untuk menetapkan target.",
            Self::Vietnamese => "💡 Tự động kết nối đang bật; vui lòng kết nối thủ công một lần để ghi nhớ mục tiêu.",
            Self::Russian => "💡 Автоподключение активно; подключитесь один раз вручную для сохранения цели.",
            Self::Spanish => "💡 Conexión auto activa; conéctate una vez manualmente para guardar el altavoz.",
        }
    }

    pub fn auto_connect_hint_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚪ 啟動後維持閒置，需手動點選音響發起連線。",
            Self::SimplifiedChinese => "⚪ 启动后维持闲置，需手动点选音响发起连接。",
            Self::English => "⚪ Remains idle on launch; manual connection required.",
            Self::French => "⚪ Reste inactif au démarrage ; connexion manuelle requise.",
            Self::German => "⚪ Startet im Leerlauf; manuelle Verbindung erforderlich.",
            Self::Italian => "⚪ Rimane inattivo all'avvio; connessione manuale richiesta.",
            Self::Japanese => "⚪ 起動時は待機状態です。スピーカーをクリックして接続してください。",
            Self::Korean => "⚪ 시작 시 대기 상태 유지, 수동으로 스피커를 선택해야 합니다.",
            Self::PortugueseBrazil => "⚪ Permanece em espera ao iniciar; conexão manual necessária.",
            Self::PortuguesePortugal => "⚪ Permanece em espera ao iniciar; é necessária ligação manual.",
            Self::Indonesian => "⚪ Tetap siaga saat peluncuran; koneksi manual diperlukan.",
            Self::Vietnamese => "⚪ Giữ ở chế độ chờ khi mở ứng dụng; cần kết nối thủ công.",
            Self::Russian => "⚪ Запуск в ожидании; требуется подключение вручную.",
            Self::Spanish => "⚪ Permanece en espera al iniciar; conexión manual requerida.",
        }
    }

    pub fn autostart_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💻 開機時自動啟動 (Launch on Boot)",
            Self::SimplifiedChinese => "💻 开机时自动启动 (Launch on Boot)",
            Self::English => "💻 Launch at Windows startup",
            Self::French => "💻 Lancer au démarrage de Windows",
            Self::German => "💻 Beim Windows-Start ausführen",
            Self::Italian => "💻 Avvia all'avvio di Windows",
            Self::Japanese => "💻 Windows 起動時に自動実行",
            Self::Korean => "💻 Windows 시작 시 자동 실행",
            Self::PortugueseBrazil => "💻 Iniciar com o Windows",
            Self::PortuguesePortugal => "💻 Iniciar com o Windows",
            Self::Indonesian => "💻 Jalankan saat startup Windows",
            Self::Vietnamese => "💻 Khởi động cùng Windows",
            Self::Russian => "💻 Запускать при загрузке Windows",
            Self::Spanish => "💻 Iniciar al arrancar Windows",
        }
    }

    pub fn autostart_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟：在登入 Windows 時自動在背景啟動 Win2Pod Bridge 並常駐於系統托盤。\n關閉：僅在手動點開時執行。",
            Self::SimplifiedChinese => "开启：在登录 Windows 时自动在后台启动 Win2Pod Bridge 并常驻系统托盘。\n关闭：仅在手动打开时运行。",
            Self::English => "On: Automatically launches Win2Pod Bridge minimized to the system tray on Windows startup.\nOff: Starts only when opened manually.",
            Self::French => "Activé : Lance automatiquement Win2Pod Bridge dans la barre d'état au démarrage.\nDésactivé : Lancement manuel uniquement.",
            Self::German => "Ein: Startet Win2Pod Bridge bei der Windows-Anmeldung automatisch im Infobereich.\nAus: Nur manueller Start.",
            Self::Italian => "Attivo: Avvia automaticamente Win2Pod Bridge nell'area di notifica all'avvio di Windows.\nDisattivo: Solo avvio manuale.",
            Self::Japanese => "ON：Windows ログイン時に自動でシステムトレイに常駐起動します。\nOFF：手動実行時のみ起動します。",
            Self::Korean => "켜짐: Windows 로그인 시 백그라운드 시스템 트레이로 자동 실행됩니다.\n꺼짐: 수동 실행 시에만 시작됩니다.",
            Self::PortugueseBrazil => "Ligado: Inicia automaticamente o Win2Pod Bridge na bandeja ao fazer login no Windows.\nDesligado: Inicia apenas manualmente.",
            Self::PortuguesePortugal => "Ligado: Inicia automaticamente o Win2Pod Bridge minimizado na área de notificação ao iniciar o Windows.\nDesligado: Inicia apenas quando aberto manualmente.",
            Self::Indonesian => "Aktif: Meluncurkan Win2Pod Bridge otomatis diminimalkan ke baki sistem saat startup Windows.\nNonaktif: Hanya mulai saat dibuka secara manual.",
            Self::Vietnamese => "Bật: Tự động khởi chạy Win2Pod Bridge thu nhỏ dưới khay hệ thống khi khởi động Windows.\nTắt: Chỉ chạy khi mở thủ công.",
            Self::Russian => "Вкл: Автоматический запуск Win2Pod Bridge в системном трее при входе в Windows.\nВыкл: Только ручной запуск.",
            Self::Spanish => "Activado: Inicia automáticamente Win2Pod Bridge en la bandeja al iniciar Windows.\nDesactivado: Inicio manual únicamente.",
        }
    }

    pub fn autostart_hint_on(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 登入 Windows 時將自動啟動並常駐於系統托盤。",
            Self::SimplifiedChinese => "💡 登录 Windows 时将自动启动并常驻系统托盘。",
            Self::English => "💡 Automatically starts in the system tray upon Windows logon.",
            Self::French => "💡 Se lance automatiquement dans la barre d'état au démarrage de Windows.",
            Self::German => "💡 Startet bei der Windows-Anmeldung automatisch im Infobereich.",
            Self::Italian => "💡 Si avvia automaticamente nell'area di notifica all'accesso a Windows.",
            Self::Japanese => "💡 Windows ログイン時に自動でトレイに常駐起動します。",
            Self::Korean => "💡 Windows 로그인 시 시스템 트레이로 자동 실행됩니다.",
            Self::PortugueseBrazil => "💡 Inicia automaticamente na bandeja do sistema ao fazer login.",
            Self::PortuguesePortugal => "💡 Inicia automaticamente na área de notificação ao iniciar sessão no Windows.",
            Self::Indonesian => "💡 Secara otomatis dimulai di baki sistem saat masuk Windows.",
            Self::Vietnamese => "💡 Tự động khởi chạy dưới khay hệ thống khi đăng nhập Windows.",
            Self::Russian => "💡 Автоматически запускается в системном трее при входе в Windows.",
            Self::Spanish => "💡 Se inicia automáticamente en la bandeja del sistema al iniciar sesión.",
        }
    }

    pub fn autostart_hint_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚪ 開機時不自動載入，需手動執行軟體。",
            Self::SimplifiedChinese => "⚪ 开机时不自动载入，需手动运行软件。",
            Self::English => "⚪ Does not start at boot; manual launch required.",
            Self::French => "⚪ Ne démarre pas au démarrage ; lancement manuel requis.",
            Self::German => "⚪ Kein automatischer Start; manueller Start erforderlich.",
            Self::Italian => "⚪ Non si avvia al boot; avvio manuale richiesto.",
            Self::Japanese => "⚪ 自動起動は無効です。手動で起動してください。",
            Self::Korean => "⚪ 부팅 시 자동 실행되지 않으며, 수동 실행이 필요합니다.",
            Self::PortugueseBrazil => "⚪ Não inicia com o Windows; início manual necessário.",
            Self::PortuguesePortugal => "⚪ Não inicia no arranque; é necessária abertura manual.",
            Self::Indonesian => "⚪ Tidak dimulai saat boot; peluncuran manual diperlukan.",
            Self::Vietnamese => "⚪ Không khởi động cùng máy tính; cần mở ứng dụng thủ công.",
            Self::Russian => "⚪ Автозапуск отключен; требуется ручной запуск.",
            Self::Spanish => "⚪ No se inicia al arrancar; inicio manual requerido.",
        }
    }

    pub fn global_hotkey_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⌨ 全域面板快捷鍵：",
            Self::SimplifiedChinese => "⌨ 全局面板热键：",
            Self::English => "⌨ Global Panel Hotkey:",
            Self::French => "⌨ Raccourci global du panneau :",
            Self::German => "⌨ Globaler Panel-Hotkey:",
            Self::Italian => "⌨ Scorciatoia globale pannello:",
            Self::Japanese => "⌨ パネル表示グローバルショートカット：",
            Self::Korean => "⌨ 전역 패널 단축키:",
            Self::PortugueseBrazil => "⌨ Atalho global do painel:",
            Self::PortuguesePortugal => "⌨ Atalho global do painel:",
            Self::Indonesian => "⌨ Tombol Pintas Panel Global:",
            Self::Vietnamese => "⌨ Phím tắt toàn cục mở bảng điều khiển:",
            Self::Russian => "⌨ Глобальная горячая клавиша:",
            Self::Spanish => "⌨ Atajo global del panel:",
        }
    }

    pub fn global_hotkey_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 在任何程式或遊戲中按下快捷鍵，即可快速喚起或收合控制面板 (支援 Esc 鍵關閉)。",
            Self::SimplifiedChinese => "💡 在任何程序或游戏中按下快捷键，即可快速唤起或收起控制面板 (支持 Esc 键关闭)。",
            Self::English => "💡 Press hotkey in any app or game to toggle the panel (Esc to dismiss).",
            Self::French => "💡 Utilisez ce raccourci dans n'importe quelle app/jeu pour afficher le panneau (Échap pour fermer).",
            Self::German => "💡 Drücken Sie den Hotkey in jeder App oder jedem Spiel, um das Panel ein-/auszublenden (Esc zum Schließen).",
            Self::Italian => "💡 Premi la scorciatoia in qualsiasi app o gioco per mostrare/nascondere il pannello (Esc per chiudere).",
            Self::Japanese => "💡 任意のアプリやゲームでショートカットを押すとパネルを開閉できます (Escキーで閉じる)。",
            Self::Korean => "💡 프로그램이나 게임 중 단축키를 눌러 패널을 열거나 닫을 수 있습니다 (Esc 키로 닫기 지원).",
            Self::PortugueseBrazil => "💡 Pressione o atalho em qualquer aplicativo para abrir/fechar o painel (Esc fecha).",
            Self::PortuguesePortugal => "💡 Prima o atalho em qualquer aplicação ou jogo para abrir/fechar o painel (Esc fecha).",
            Self::Indonesian => "💡 Tekan tombol pintas di aplikasi atau game apa pun untuk beralih panel (Esc untuk menutup).",
            Self::Vietnamese => "💡 Nhấn phím tắt trong bất kỳ ứng dụng hoặc trò chơi nào để bật/tắt bảng điều khiển (Esc để ẩn).",
            Self::Russian => "💡 Нажмите горячую клавишу в любой игре или приложении для открытия панели (Esc закрывает).",
            Self::Spanish => "💡 Pulsa el atajo en cualquier aplicación o juego para abrir o cerrar el panel (Esc cierra).",
        }
    }

    pub fn hotkey_default_suffix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "(預設)",
            Self::SimplifiedChinese => "(默认)",
            Self::English => "(Default)",
            Self::French => "(Défaut)",
            Self::German => "(Standard)",
            Self::Italian => "(Predefinito)",
            Self::Japanese => "(デフォルト)",
            Self::Korean => "(기본값)",
            Self::PortugueseBrazil => "(Padrão)",
            Self::PortuguesePortugal => "(Predefinido)",
            Self::Indonesian => "(Default)",
            Self::Vietnamese => "(Mặc định)",
            Self::Russian => "(По умолч.)",
            Self::Spanish => "(Por defecto)",
        }
    }

    pub fn hotkey_disabled(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關閉快捷鍵 (無)",
            Self::SimplifiedChinese => "关闭快捷键 (无)",
            Self::English => "Disabled (None)",
            Self::French => "Désactivé (Aucun)",
            Self::German => "Deaktiviert (Keiner)",
            Self::Italian => "Disabilitato (Nessuno)",
            Self::Japanese => "無効 (なし)",
            Self::Korean => "단축키 끄기 (없음)",
            Self::PortugueseBrazil => "Desativado (Nenhum)",
            Self::PortuguesePortugal => "Desativado (Nenhum)",
            Self::Indonesian => "Dinonaktifkan (Tidak Ada)",
            Self::Vietnamese => "Đã tắt (Không dùng)",
            Self::Russian => "Отключено (Нет)",
            Self::Spanish => "Desactivado (Ninguno)",
        }
    }

    pub fn hotkey_custom(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "自訂快捷鍵",
            Self::SimplifiedChinese => "自定义快捷键",
            Self::English => "Custom Hotkey",
            Self::French => "Raccourci personnalisé",
            Self::German => "Benutzerdefinierter Hotkey",
            Self::Italian => "Scorciatoia personalizzata",
            Self::Japanese => "カスタムショートカット",
            Self::Korean => "사용자 지정 단축키",
            Self::PortugueseBrazil => "Atalho personalizado",
            Self::PortuguesePortugal => "Atalho personalizado",
            Self::Indonesian => "Pintasan Kustom",
            Self::Vietnamese => "Phím tắt tùy chỉnh",
            Self::Russian => "Своя клавиша",
            Self::Spanish => "Atajo personalizado",
        }
    }

    pub fn language_selector_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🌐 介面語言 / Language：",
            Self::SimplifiedChinese => "🌐 界面语言 / Language：",
            Self::English => "🌐 Interface Language:",
            Self::French => "🌐 Langue de l'interface :",
            Self::German => "🌐 Sprache der Benutzeroberfläche:",
            Self::Italian => "🌐 Lingua dell'interfaccia:",
            Self::Japanese => "🌐 表示言語 / Language：",
            Self::Korean => "🌐 인터페이스 언어 / Language:",
            Self::PortugueseBrazil => "🌐 Idioma da interface:",
            Self::PortuguesePortugal => "🌐 Idioma da interface:",
            Self::Indonesian => "🌐 Bahasa Antarmuka:",
            Self::Vietnamese => "🌐 Ngôn ngữ giao diện:",
            Self::Russian => "🌐 Язык интерфейса:",
            Self::Spanish => "🌐 Idioma de la interfaz:",
        }
    }

    pub fn tooltip_about(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "軟體資訊",
            Self::SimplifiedChinese => "软件信息",
            Self::English => "Software Information",
            Self::French => "Informations sur le logiciel",
            Self::German => "Software-Informationen",
            Self::Italian => "Informazioni sul software",
            Self::Japanese => "ソフトウェア情報",
            Self::Korean => "소프트웨어 정보",
            Self::PortugueseBrazil => "Informações do software",
            Self::PortuguesePortugal => "Informações do software",
            Self::Indonesian => "Informasi Perangkat Lunak",
            Self::Vietnamese => "Thông tin phần mềm",
            Self::Russian => "Информация о программе",
            Self::Spanish => "Información del software",
        }
    }

    pub fn btn_about(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "軟體資訊",
            Self::SimplifiedChinese => "软件信息",
            Self::English => "Software Information",
            Self::French => "Informations sur le logiciel",
            Self::German => "Software-Informationen",
            Self::Italian => "Informazioni sul software",
            Self::Japanese => "ソフトウェア情報",
            Self::Korean => "소프트웨어 정보",
            Self::PortugueseBrazil => "Informações do software",
            Self::PortuguesePortugal => "Informações do software",
            Self::Indonesian => "Informasi Perangkat Lunak",
            Self::Vietnamese => "Thông tin phần mềm",
            Self::Russian => "Информация о программе",
            Self::Spanish => "Información del software",
        }
    }

    pub fn about_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "軟體資訊",
            Self::SimplifiedChinese => "软件信息",
            Self::English => "About Win2Pod",
            Self::French => "À propos de Win2Pod",
            Self::German => "Über Win2Pod",
            Self::Italian => "Informazioni su Win2Pod",
            Self::Japanese => "Win2Pod について",
            Self::Korean => "Win2Pod 정보",
            Self::PortugueseBrazil => "Sobre o Win2Pod",
            Self::PortuguesePortugal => "Sobre o Win2Pod",
            Self::Indonesian => "Tentang Win2Pod",
            Self::Vietnamese => "Giới thiệu về Win2Pod",
            Self::Russian => "О программе Win2Pod",
            Self::Spanish => "Acerca de Win2Pod",
        }
    }

    pub fn about_description(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "適用於 Windows 的開源純 Rust AirPlay 2 音訊串流橋接工具",
            Self::SimplifiedChinese => "适用于 Windows 的开源纯 Rust AirPlay 2 音频串流桥接工具",
            Self::English => "Open-source pure Rust AirPlay 2 audio streaming bridge for Windows",
            Self::French => "Passerelle de streaming audio AirPlay 2 en pur Rust pour Windows",
            Self::German => "Open-Source Pure-Rust AirPlay 2 Audio-Streaming-Bridge für Windows",
            Self::Italian => "Bridge di streaming audio AirPlay 2 open source in puro Rust per Windows",
            Self::Japanese => "Windows 向けオープンソース Pure-Rust AirPlay 2 オーディオストリーミングツール",
            Self::Korean => "Windows용 오픈 소스 순수 Rust AirPlay 2 오디오 스트리밍 브리지",
            Self::PortugueseBrazil => "Ponte de streaming de áudio AirPlay 2 em Rust puro e código aberto para Windows",
            Self::PortuguesePortugal => "Ponte de transmissão de áudio AirPlay 2 em Rust puro e código aberto para Windows",
            Self::Indonesian => "Jembatan streaming audio AirPlay 2 murni Rust sumber terbuka untuk Windows",
            Self::Vietnamese => "Cầu nối truyền phát âm thanh AirPlay 2 mã nguồn mở thuần Rust cho Windows",
            Self::Russian => "AirPlay 2 аудиотранслятор для Windows с открытым исходным кодом на чистом Rust",
            Self::Spanish => "Puente de transmisión de audio AirPlay 2 de código abierto en Rust puro para Windows",
        }
    }

    pub fn about_version_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "版本號碼：",
            Self::SimplifiedChinese => "版本号：",
            Self::English => "Version:",
            Self::French => "Version :",
            Self::German => "Version:",
            Self::Italian => "Versione:",
            Self::Japanese => "バージョン：",
            Self::Korean => "버전:",
            Self::PortugueseBrazil => "Versão:",
            Self::PortuguesePortugal => "Versão:",
            Self::Indonesian => "Versi:",
            Self::Vietnamese => "Phiên bản:",
            Self::Russian => "Версия:",
            Self::Spanish => "Versión:",
        }
    }

    pub fn about_author_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "軟體作者：",
            Self::SimplifiedChinese => "软件作者：",
            Self::English => "Author:",
            Self::French => "Auteur :",
            Self::German => "Autor:",
            Self::Italian => "Autore:",
            Self::Japanese => "開発者：",
            Self::Korean => "작성자:",
            Self::PortugueseBrazil => "Autor:",
            Self::PortuguesePortugal => "Autor:",
            Self::Indonesian => "Penulis:",
            Self::Vietnamese => "Tác giả:",
            Self::Russian => "Автор:",
            Self::Spanish => "Autor:",
        }
    }

    pub fn about_author_value(&self) -> &'static str {
        "mesmerli"
    }

    pub fn about_license_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開源授權：",
            Self::SimplifiedChinese => "开源协议：",
            Self::English => "License:",
            Self::French => "Licence :",
            Self::German => "Lizenz:",
            Self::Italian => "Licenza:",
            Self::Japanese => "ライセンス：",
            Self::Korean => "라이선스:",
            Self::PortugueseBrazil => "Licença:",
            Self::PortuguesePortugal => "Licença:",
            Self::Indonesian => "Lisensi:",
            Self::Vietnamese => "Giấy phép:",
            Self::Russian => "Лицензия:",
            Self::Spanish => "Licencia:",
        }
    }

    pub fn about_copyright_notice(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "版權宣告：Win2Pod 介面版權所有 © 2024-2026 mesmerli。保留一切權利（專屬商業授權）。後端串流引擎 win2pod-engine.exe 為自由軟體，依據 GNU 通用公眾授權條款第三版或後續版本 (GPL-3.0-or-later) 發布。",
            Self::SimplifiedChinese => "版权宣告：Win2Pod 界面版权所有 © 2024-2026 mesmerli。保留一切权利（专有商业授权）。后端流引擎 win2pod-engine.exe 为自由软件，依据 GNU 通用公共许可证第三版或后续版本 (GPL-3.0-or-later) 发布。",
            Self::English => "Copyright Notice: Win2Pod GUI Copyright © 2024-2026 mesmerli. All Rights Reserved (Proprietary). Backend audio engine win2pod-engine.exe is free software released under the GNU General Public License v3.0 or later (GPL-3.0-or-later).",
            Self::French => "Mentions de droits d'auteur : Win2Pod GUI Copyright © 2024-2026 mesmerli. Tous droits réservés (Propriétaire). Le moteur audio win2pod-engine.exe est un logiciel libre publié sous GNU GPL v3.0 ou ultérieure.",
            Self::German => "Urheberrechtshinweis: Win2Pod GUI Copyright © 2024-2026 mesmerli. Alle Rechte vorbehalten (Proprietär). Das Audio-Backend win2pod-engine.exe ist freie Software unter GNU GPL v3.0 oder neuer.",
            Self::Italian => "Avviso di copyright: Win2Pod GUI Copyright © 2024-2026 mesmerli. Tutti i diritti riservati (Proprietario). Il motore audio win2pod-engine.exe è software libero sotto licenza GNU GPL v3.0 o successiva.",
            Self::Japanese => "著作権表記：Win2Pod GUI Copyright © 2024-2026 mesmerli。不許複製・禁無断転載（商用プロプライエタリ）。バックエンドエンジン win2pod-engine.exe は GNU GPL v3.0 以降で配布されるフリーソフトウェアです。",
            Self::Korean => "저작권 고지: Win2Pod GUI Copyright © 2024-2026 mesmerli. 판권 소유 (상용 독점 라이선스). 백엔드 오디오 엔진 win2pod-engine.exe는 GNU GPL v3.0 이상에 따라 배포되는 자유 소프트웨어입니다.",
            Self::PortugueseBrazil => "Aviso de direitos autorais: Win2Pod GUI Copyright © 2024-2026 mesmerli. Todos os direitos reservados (Proprietário). O mecanismo win2pod-engine.exe é software livre sob a GNU GPL v3.0 ou posterior.",
            Self::PortuguesePortugal => "Aviso de direitos de autor: Win2Pod GUI Copyright © 2024-2026 mesmerli. Todos os direitos reservados (Proprietário). O motor áudio win2pod-engine.exe é software livre sob a GNU GPL v3.0 ou posterior.",
            Self::Indonesian => "Pemberitahuan Hak Cipta: Win2Pod GUI Hak Cipta © 2024-2026 mesmerli. Hak cipta dilindungi undang-undang (Hak Milik). Mesin audio win2pod-engine.exe dirilis di bawah GNU GPL v3.0 atau yang lebih baru.",
            Self::Vietnamese => "Thông báo bản quyền: Giao diện Win2Pod Bản quyền © 2024-2026 mesmerli. Đã đăng ký bản quyền (Độc quyền). Động cơ win2pod-engine.exe phát hành theo Giấy phép GNU GPL v3.0 trở lên.",
            Self::Russian => "Уведомление об авторских правах: Win2Pod GUI Copyright © 2024-2026 mesmerli. Все права защищены (Проприетарное ПО). Аудиодвижок win2pod-engine.exe распространяется под лицензией GNU GPL v3.0 или более поздней.",
            Self::Spanish => "Aviso de copyright: Win2Pod GUI Copyright © 2024-2026 mesmerli. Todos los derechos reservados (Propietario). El motor de audio win2pod-engine.exe es software libre bajo GNU GPL v3.0 o posterior.",
        }
    }

    pub fn pkg_airplay2_rs_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 2 串流協議核心、RTSP 工作階段與 PTP/NTP 時鐘同步實作",
            Self::SimplifiedChinese => "AirPlay 2 串流协议核心、RTSP 会话与 PTP/NTP 时钟同步实现",
            Self::English => "AirPlay 2 streaming protocol engine, RTSP sessions & PTP/NTP clock sync",
            Self::French => "Moteur de protocole AirPlay 2, sessions RTSP et synchro d'horloge PTP/NTP",
            Self::German => "AirPlay 2 Streaming-Protokoll-Engine, RTSP-Sitzungen & PTP/NTP Zeitsynchronisation",
            Self::Italian => "Motore del protocollo AirPlay 2, sessioni RTSP e sincronizzazione orologio PTP/NTP",
            Self::Japanese => "AirPlay 2 ストリーミングプロトコルエンジン、RTSPセッション、PTP/NTPクロック同期",
            Self::Korean => "AirPlay 2 스트리밍 프로토콜 엔진, RTSP 세션 및 PTP/NTP 클럭 동기화",
            Self::PortugueseBrazil => "Motor de protocolo AirPlay 2, sessões RTSP e sincronização de relógio PTP/NTP",
            Self::PortuguesePortugal => "Motor de protocolo de transmissão AirPlay 2, sessões RTSP e sincronização de relógio PTP/NTP",
            Self::Indonesian => "Mesin protokol streaming AirPlay 2, sesi RTSP & sinkronisasi jam PTP/NTP",
            Self::Vietnamese => "Công cụ giao thức truyền phát AirPlay 2, phiên RTSP & đồng bộ xung nhịp PTP/NTP",
            Self::Russian => "Движок протокола AirPlay 2, сессии RTSP и синхронизация часов PTP/NTP",
            Self::Spanish => "Motor de protocolo AirPlay 2, sesiones RTSP y sincronización de reloj PTP/NTP",
        }
    }

    pub fn pkg_windows_airplay_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Windows AirPlay 串流原型與上游移植專案",
            Self::SimplifiedChinese => "Windows AirPlay 串流原型与上游移植项目",
            Self::English => "Windows AirPlay streaming prototype & upstream port project",
            Self::French => "Prototype de streaming Windows AirPlay et projet amont porté",
            Self::German => "Windows AirPlay Streaming-Prototyp & Upstream-Port-Projekt",
            Self::Italian => "Prototipo di streaming Windows AirPlay e progetto upstream",
            Self::Japanese => "Windows AirPlay ストリーミングプロトタイプとアップストリーム移植プロジェクト",
            Self::Korean => "Windows AirPlay 스트리밍 프로토타입 및 업스트림 포팅 프로젝트",
            Self::PortugueseBrazil => "Protótipo de streaming Windows AirPlay e projeto de portabilidade upstream",
            Self::PortuguesePortugal => "Protótipo de transmissão Windows AirPlay e projeto de portabilidade upstream",
            Self::Indonesian => "Prototipe streaming Windows AirPlay & proyek port hulu",
            Self::Vietnamese => "Nguyên mẫu truyền phát Windows AirPlay & dự án chuyển đổi mã nguồn phía trên",
            Self::Russian => "Прототип стриминга Windows AirPlay и апстрим-проект",
            Self::Spanish => "Prototipo de transmisión Windows AirPlay y proyecto portado upstream",
        }
    }

    pub fn pkg_uxplay_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開源 AirPlay 接收端伺服器 (RAOP / RTSP 協定實作與 DACP 遠端控制參考)",
            Self::SimplifiedChinese => "开源 AirPlay 接收端服务器 (RAOP / RTSP 协议实现与 DACP 远程控制参考)",
            Self::English => "Open-source AirPlay receiver server (RAOP / RTSP protocol implementation & DACP remote control reference)",
            Self::French => "Serveur récepteur AirPlay open-source (implémentation des protocoles RAOP / RTSP et contrôle DACP)",
            Self::German => "Open-Source-AirPlay-Empfängerserver (RAOP / RTSP-Protokollimplementierung & DACP-Fernbedienungsreferenz)",
            Self::Italian => "Server ricevitore AirPlay open source (implementazione dei protocolli RAOP / RTSP e controllo remoto DACP)",
            Self::Japanese => "オープンソース AirPlay レシーバーサーバー (RAOP / RTSP プロトコル実装および DACP リモートコントロールリファレンス)",
            Self::Korean => "오픈 소스 AirPlay 수신기 서버 (RAOP / RTSP 프로토콜 구현 및 DACP 원격 제어 참조)",
            Self::PortugueseBrazil => "Servidor receptor AirPlay de código aberto (implementação dos protocolos RAOP / RTSP e controle remoto DACP)",
            Self::PortuguesePortugal => "Servidor recetor AirPlay de código aberto (implementação dos protocolos RAOP / RTSP e referência de controlo remoto DACP)",
            Self::Indonesian => "Server penerima AirPlay sumber terbuka (implementasi protokol RAOP / RTSP & referensi kendali jarak jauh DACP)",
            Self::Vietnamese => "Máy chủ nhận AirPlay mã nguồn mở (triển khai giao thức RAOP / RTSP & tham chiếu điều khiển từ xa DACP)",
            Self::Russian => "Сервер-приемник AirPlay с открытым исходным кодом (реализация протоколов RAOP / RTSP и управление DACP)",
            Self::Spanish => "Servidor receptor AirPlay de código abierto (implementación de protocolos RAOP / RTSP y control remoto DACP)",
        }
    }

    pub fn pkg_airpodsdesktop_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Apple Continuity BLE 廣播協定逆向工程與 AirPods 入耳/電量狀態偵測參考",
            Self::SimplifiedChinese => "Apple Continuity BLE 广播协议逆向工程与 AirPods 入耳/电量状态检测参考",
            Self::English => "Apple Continuity BLE protocol reverse-engineering & AirPods in-ear detection reference",
            Self::French => "Rétro-ingénierie du protocole Apple Continuity BLE et détection de port AirPods",
            Self::German => "Reverse-Engineering des Apple Continuity BLE-Protokolls & AirPods Trageerkennung",
            Self::Italian => "Reverse-engineering del protocollo Apple Continuity BLE e rilevamento auricolari AirPods",
            Self::Japanese => "Apple Continuity BLE プロトコルの解析および AirPods 装着・バッテリー状態検出リファレンス",
            Self::Korean => "Apple Continuity BLE 프로토콜 리버스 엔지니어링 및 AirPods 착용/배터리 감지 참조",
            Self::PortugueseBrazil => "Engenharia reversa do protocolo Apple Continuity BLE e detecção intra-auricular do AirPods",
            Self::PortuguesePortugal => "Engenharia inversa do protocolo Apple Continuity BLE e referência de deteção intra-auricular dos AirPods",
            Self::Indonesian => "Rekayasa balik protokol Apple Continuity BLE & referensi deteksi dalam telinga AirPods",
            Self::Vietnamese => "Kỹ thuật dịch ngược giao thức Apple Continuity BLE & tham chiếu phát hiện đeo tai nghe AirPods",
            Self::Russian => "Обратная разработка протокола Apple Continuity BLE и обнаружение ношения AirPods",
            Self::Spanish => "Ingeniería inversa del protocolo Apple Continuity BLE y detección de uso de AirPods",
        }
    }

    pub fn pkg_muelle_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "macOS Genie Effect 神燈最小化動畫數學模型與視覺演算法參考",
            Self::SimplifiedChinese => "macOS Genie Effect 神灯最小化动画数学模型与视觉算法参考",
            Self::English => "macOS Genie Effect minimize animation mathematical model and visual algorithms reference",
            Self::French => "Modèle mathématique et algorithmes visuels de l'effet Génie de macOS",
            Self::German => "Mathematisches Modell und visuelle Algorithmen für den macOS Genie-Effekt",
            Self::Italian => "Modello matematico e algoritmi visivi dell'effetto Genio di macOS",
            Self::Japanese => "macOS Genie Effect 最小化アニメーションの数学モデルおよび視覚アルゴリズムのリファレンス",
            Self::Korean => "macOS Genie Effect 최소화 애니메이션 수학 모델 및 시각 알고리즘 참조",
            Self::PortugueseBrazil => "Modelo matemático e algoritmos visuais do efeito Gênio do macOS",
            Self::PortuguesePortugal => "Modelo matemático e algoritmos visuais do efeito Génio do macOS",
            Self::Indonesian => "Model matematika dan algoritme visual animasi Genie Effect macOS",
            Self::Vietnamese => "Mô hình toán học và thuật toán trực quan hiệu ứng Genie Effect của macOS",
            Self::Russian => "Математическая модель и визуальные алгоритмы анимации Genie Effect в macOS",
            Self::Spanish => "Modelo matemático y algoritmos visuales del efecto Genio de macOS",
        }
    }

    pub fn about_packages_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開源致謝與授權",
            Self::SimplifiedChinese => "开源致谢与许可",
            Self::English => "Open Source Credits & Licenses",
            Self::French => "Remerciements open source et licences",
            Self::German => "Open-Source-Danksagungen & Lizenzen",
            Self::Italian => "Riconoscimenti open source e licenze",
            Self::Japanese => "オープンソース謝辞とライセンス",
            Self::Korean => "오픈 소스 감사 및 라이선스",
            Self::PortugueseBrazil => "Créditos e licenças de código aberto",
            Self::PortuguesePortugal => "Créditos e licenças de código aberto",
            Self::Indonesian => "Kredit & Lisensi Sumber Terbuka",
            Self::Vietnamese => "Ghi công & Giấy phép mã nguồn mở",
            Self::Russian => "Благодарности Open Source и лицензии",
            Self::Spanish => "Agradecimientos de código abierto y licencias",
        }
    }

    pub fn pkg_egui_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "即時模式原生圖形介面框架 (Immediate Mode GUI)",
            Self::SimplifiedChinese => "即时模式原生图形界面框架 (Immediate Mode GUI)",
            Self::English => "Immediate mode native GUI framework",
            Self::French => "Framework d'interface graphique native en mode immédiat",
            Self::German => "Immediate-Mode natives GUI-Framework",
            Self::Italian => "Framework GUI nativo in modalità immediata",
            Self::Japanese => "イミディエイトモード ネイティブGUIフレームワーク",
            Self::Korean => "즉시 모드 네이티브 GUI 프레임워크",
            Self::PortugueseBrazil => "Framework de interface gráfica nativa em modo imediato",
            Self::PortuguesePortugal => "Framework de interface gráfica nativa em modo imediato",
            Self::Indonesian => "Kerangka kerja GUI asli mode langsung",
            Self::Vietnamese => "Framework giao diện người dùng trực tiếp thuần Rust",
            Self::Russian => "Фреймворк нативного графического интерфейса",
            Self::Spanish => "Framework de interfaz gráfica nativa en modo inmediato",
        }
    }

    pub fn pkg_wasapi_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Rust 音訊回錄擷取庫 (WASAPI Loopback Capture)",
            Self::SimplifiedChinese => "Rust 音频回录捕获库 (WASAPI Loopback Capture)",
            Self::English => "Audio loopback capture library for Windows",
            Self::French => "Bibliothèque de capture audio de bouclage",
            Self::German => "Audio-Loopback-Aufnahme-Bibliothek",
            Self::Italian => "Libreria di cattura audio loopback",
            Self::Japanese => "オーディオループバック録音ライブラリ",
            Self::Korean => "오디오 루프백 캡처 라이브러리",
            Self::PortugueseBrazil => "Biblioteca de captura de áudio em loopback",
            Self::PortuguesePortugal => "Biblioteca de captura de áudio em loopback para o Windows",
            Self::Indonesian => "Pustaka tangkapan loopback audio untuk Windows",
            Self::Vietnamese => "Thư viện thu nhận âm thanh lặp lại (loopback) cho Windows",
            Self::Russian => "Библиотека захвата системного аудио",
            Self::Spanish => "Biblioteca de captura de audio en bucle",
        }
    }

    pub fn pkg_alac_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Apple 無損音訊壓縮編碼器 (ALAC Encoder)",
            Self::SimplifiedChinese => "Apple 无损音频压缩编码器 (ALAC Encoder)",
            Self::English => "Apple Lossless Audio Codec (ALAC) encoder",
            Self::French => "Encodeur Apple Lossless Audio Codec (ALAC)",
            Self::German => "Apple Lossless Audio Codec (ALAC) Encoder",
            Self::Italian => "Codificatore Apple Lossless Audio Codec (ALAC)",
            Self::Japanese => "Apple Lossless Audio Codec (ALAC) エンコーダー",
            Self::Korean => "Apple Lossless Audio Codec (ALAC) 인코더",
            Self::PortugueseBrazil => "Codificador Apple Lossless Audio Codec (ALAC)",
            Self::PortuguesePortugal => "Codificador Apple Lossless Audio Codec (ALAC)",
            Self::Indonesian => "Pembuat enkode Apple Lossless Audio Codec (ALAC)",
            Self::Vietnamese => "Bộ mã hóa Apple Lossless Audio Codec (ALAC)",
            Self::Russian => "Кодировщик Apple Lossless Audio Codec (ALAC)",
            Self::Spanish => "Codificador Apple Lossless Audio Codec (ALAC)",
        }
    }

    pub fn pkg_rubato_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "高品質非同步音訊取樣率轉換 (Audio Resampler)",
            Self::SimplifiedChinese => "高质量异步音频采样率转换 (Audio Resampler)",
            Self::English => "High-quality asynchronous audio resampler",
            Self::French => "Rééchantillonneur audio asynchrone haute fidélité",
            Self::German => "Hochwertiger asynchroner Audio-Resampler",
            Self::Italian => "Ricampionatore audio asincrono ad alta fedeltà",
            Self::Japanese => "高品質非同期オーディオリサンプラー",
            Self::Korean => "고품질 비동기 오디오 리샘플러",
            Self::PortugueseBrazil => "Reamostrador de áudio assíncrono de alta qualidade",
            Self::PortuguesePortugal => "Reamostrador de áudio assíncrono de alta qualidade",
            Self::Indonesian => "Pengambil sampel ulang audio asinkron berkualitas tinggi",
            Self::Vietnamese => "Bộ tái lấy mẫu âm thanh bất đồng bộ chất lượng cao",
            Self::Russian => "Высококачественный асинхронный аудио-ресемплер",
            Self::Spanish => "Remuestreador de audio asíncrono de alta calidad",
        }
    }

    pub fn pkg_symphonia_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "純 Rust 多格式音訊解碼與探測庫",
            Self::SimplifiedChinese => "纯 Rust 多格式音频解码与探测库",
            Self::English => "Pure Rust audio decoding and media probing library",
            Self::French => "Bibliothèque de décodage audio et détection multimédia en pur Rust",
            Self::German => "Pure-Rust Audio-Decodierungs- und Medienbibliothek",
            Self::Italian => "Libreria di decodifica audio e analisi multimediale in puro Rust",
            Self::Japanese => "Pure-Rust マルチフォーマット音声デコード・プローブライブラリ",
            Self::Korean => "순수 Rust 멀티 포맷 오디오 디코딩 및 미디어 분석 라이브러리",
            Self::PortugueseBrazil => "Biblioteca de decodificação de áudio e análise de mídia em Rust puro",
            Self::PortuguesePortugal => "Biblioteca de descodificação de áudio e análise de multimédia em Rust puro",
            Self::Indonesian => "Pustaka decoding audio dan probing media murni Rust",
            Self::Vietnamese => "Thư viện giải mã âm thanh và kiểm tra đa phương tiện thuần Rust",
            Self::Russian => "Библиотека декодирования аудио и анализа медиа на чистом Rust",
            Self::Spanish => "Biblioteca de decodificación de audio y análisis multimedia en Rust puro",
        }
    }

    pub fn pkg_tokio_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "高效能非同步多執行緒網路執行環境 (Async Runtime)",
            Self::SimplifiedChinese => "高性能异步多线程网络运行时 (Async Runtime)",
            Self::English => "High-performance asynchronous multi-threaded runtime",
            Self::French => "Moteur d'exécution réseau asynchrone multithread haute performance",
            Self::German => "Hochleistungsfähige asynchrone Multithread-Laufzeitumgebung",
            Self::Italian => "Runtime di rete asincrono multithread ad alte prestazioni",
            Self::Japanese => "高性能非同期マルチスレッド ネットワークランタイム",
            Self::Korean => "고성능 비동기 멀티스레드 네트워크 런타임",
            Self::PortugueseBrazil => "Ambiente de execução de rede assíncrono multithread de alta performance",
            Self::PortuguesePortugal => "Ambiente de execução de rede assíncrono multithread de alto desempenho",
            Self::Indonesian => "Runtime multi-threaded asinkron berkinerja tinggi",
            Self::Vietnamese => "Môi trường thực thi bất đồng bộ đa luồng hiệu năng cao",
            Self::Russian => "Высокопроизводительная асинхронная многопоточная среда выполнения",
            Self::Spanish => "Entorno de ejecución de red asíncrono y multihilo de alto rendimiento",
        }
    }

    pub fn pkg_mdns_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "mDNS / DNS-SD 區域網路服務零設定自動發現",
            Self::SimplifiedChinese => "mDNS / DNS-SD 局域网服务零配置自动发现",
            Self::English => "Zero-configuration multicast DNS service discovery (mDNS/DNS-SD)",
            Self::French => "Découverte de services réseau sans configuration (mDNS/DNS-SD)",
            Self::German => "Konfigurationsfreie Multicast-DNS-Diensterkennung (mDNS/DNS-SD)",
            Self::Italian => "Rilevamento servizi di rete a configurazione zero (mDNS/DNS-SD)",
            Self::Japanese => "ゼロコンフィグレーション mDNS/DNS-SD サービス自動検出",
            Self::Korean => "무설정 mDNS/DNS-SD 로컬 네트워크 서비스 자동 검색",
            Self::PortugueseBrazil => "Descoberta de serviços sem configuração via mDNS/DNS-SD",
            Self::PortuguesePortugal => "Deteção de serviços sem configuração através de mDNS/DNS-SD",
            Self::Indonesian => "Penemuan layanan multicast DNS tanpa konfigurasi (mDNS/DNS-SD)",
            Self::Vietnamese => "Tự động phát hiện dịch vụ DNS phát đa hướng không cần cấu hình (mDNS/DNS-SD)",
            Self::Russian => "Автоматическое обнаружение сервисов в сети (mDNS/DNS-SD)",
            Self::Spanish => "Detección de servicios de red sin configuración (mDNS/DNS-SD)",
        }
    }

    pub fn pkg_socket2_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "跨平台底層 Socket 網路套接字抽象控制庫",
            Self::SimplifiedChinese => "跨平台底层 Socket 网络套接字抽象控制库",
            Self::English => "Cross-platform low-level socket networking configuration",
            Self::French => "Configuration et contrôle de sockets réseau bas niveau multiplateforme",
            Self::German => "Plattformübergreifende Low-Level-Socket-Netzwerkkonfiguration",
            Self::Italian => "Configurazione e controllo di socket di rete a basso livello",
            Self::Japanese => "クロスプラットフォーム低レベルソケットネットワーク制御",
            Self::Korean => "크로스 플랫폼 저수준 소켓 네트워크 구성 라이브러리",
            Self::PortugueseBrazil => "Configuração de sockets de rede de baixo nível multiplataforma",
            Self::PortuguesePortugal => "Configuração de sockets de rede de baixo nível multiplataforma",
            Self::Indonesian => "Konfigurasi jaringan soket tingkat rendah lintas platform",
            Self::Vietnamese => "Cấu hình mạng socket cấp thấp đa nền tảng",
            Self::Russian => "Низкоуровневая кроссплатформенная работа с сетевыми сокетами",
            Self::Spanish => "Configuración y control de sockets de red de bajo nivel multiplataforma",
        }
    }

    pub fn pkg_chacha_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "ChaCha20-Poly1305 AEAD 認證加密演算法",
            Self::SimplifiedChinese => "ChaCha20-Poly1305 AEAD 认证加密算法",
            Self::English => "ChaCha20-Poly1305 authenticated encryption cipher (AEAD)",
            Self::French => "Chiffrement authentifié ChaCha20-Poly1305 (AEAD)",
            Self::German => "ChaCha20-Poly1305 authentifizierte Verschlüsselung (AEAD)",
            Self::Italian => "Cifratura autenticata ChaCha20-Poly1305 (AEAD)",
            Self::Japanese => "ChaCha20-Poly1305 認証付き暗号アルゴリズム (AEAD)",
            Self::Korean => "ChaCha20-Poly1305 인증 암호화 알고리즘 (AEAD)",
            Self::PortugueseBrazil => "Cifra de criptografia autenticada ChaCha20-Poly1305 (AEAD)",
            Self::PortuguesePortugal => "Cifra de encriptação autenticada ChaCha20-Poly1305 (AEAD)",
            Self::Indonesian => "Sandi enkripsi terautentikasi ChaCha20-Poly1305 (AEAD)",
            Self::Vietnamese => "Mật mã mã hóa xác thực ChaCha20-Poly1305 (AEAD)",
            Self::Russian => "Аутентифицированное шифрование ChaCha20-Poly1305 (AEAD)",
            Self::Spanish => "Cifrado autenticado ChaCha20-Poly1305 (AEAD)",
        }
    }

    pub fn pkg_aes_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AES-GCM / CBC / CTR 高效對稱分組加密",
            Self::SimplifiedChinese => "AES-GCM / CBC / CTR 高效对称分组加密",
            Self::English => "AES-GCM, CBC, and CTR symmetric block ciphers",
            Self::French => "Chiffrements par blocs symétriques AES-GCM, CBC et CTR",
            Self::German => "Symmetrische Blockchiffren AES-GCM, CBC und CTR",
            Self::Italian => "Cifrature simmetriche a blocchi AES-GCM, CBC e CTR",
            Self::Japanese => "AES-GCM / CBC / CTR 高速対称ブロック暗号",
            Self::Korean => "AES-GCM, CBC 및 CTR 대칭 블록 암호",
            Self::PortugueseBrazil => "Cifras de bloco simétricas AES-GCM, CBC e CTR",
            Self::PortuguesePortugal => "Cifras de bloco simétricas AES-GCM, CBC e CTR",
            Self::Indonesian => "Sandi blok simetris AES-GCM, CBC, dan CTR",
            Self::Vietnamese => "Các mật mã khối đối xứng AES-GCM, CBC và CTR",
            Self::Russian => "Симметричные блочные шифры AES-GCM, CBC и CTR",
            Self::Spanish => "Cifrados de bloque simétricos AES-GCM, CBC y CTR",
        }
    }

    pub fn pkg_dalek_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Curve25519 / Ed25519 數位簽章與 X25519 金鑰協商",
            Self::SimplifiedChinese => "Curve25519 / Ed25519 数字签名与 X25519 密钥协商",
            Self::English => "Curve25519 / Ed25519 signatures and X25519 key exchange",
            Self::French => "Signatures Curve25519 / Ed25519 et échange de clés X25519",
            Self::German => "Curve25519 / Ed25519 Signaturen und X25519-Schlüsselaustausch",
            Self::Italian => "Firme Curve25519 / Ed25519 e scambio chiavi X25519",
            Self::Japanese => "Curve25519 / Ed25519 署名および X25519 鍵交換",
            Self::Korean => "Curve25519 / Ed25519 전자 서명 및 X25519 키 교환",
            Self::PortugueseBrazil => "Assinaturas Curve25519 / Ed25519 e troca de chaves X25519",
            Self::PortuguesePortugal => "Assinaturas Curve25519 / Ed25519 e troca de chaves X25519",
            Self::Indonesian => "Tanda tangan Curve25519 / Ed25519 dan pertukaran kunci X25519",
            Self::Vietnamese => "Chữ ký Curve25519 / Ed25519 và trao đổi khóa X25519",
            Self::Russian => "Цифровые подписи Curve25519 / Ed25519 и обмен ключами X25519",
            Self::Spanish => "Firmas Curve25519 / Ed25519 e intercambio de claves X25519",
        }
    }

    pub fn pkg_rsa_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "純 Rust RSA 公鑰加密與金鑰封裝演算法",
            Self::SimplifiedChinese => "纯 Rust RSA 公钥加密与密钥封装算法",
            Self::English => "Pure Rust RSA public-key encryption and key encapsulation",
            Self::French => "Chiffrement à clé publique RSA en pur Rust",
            Self::German => "Pure-Rust RSA-Public-Key-Verschlüsselung",
            Self::Italian => "Crittografia a chiave pubblica RSA in puro Rust",
            Self::Japanese => "Pure-Rust RSA 公開鍵暗号化および鍵カプセル化",
            Self::Korean => "순수 Rust RSA 공개키 암호화 및 키 캡슐화 알고리즘",
            Self::PortugueseBrazil => "Criptografia de chave pública RSA em Rust puro",
            Self::PortuguesePortugal => "Encriptação de chave pública RSA em Rust puro e encapsulamento de chaves",
            Self::Indonesian => "Enkripsi kunci publik RSA dan enkapsulasi kunci murni Rust",
            Self::Vietnamese => "Mã hóa khóa công khai RSA và đóng gói khóa thuần Rust",
            Self::Russian => "Криптосистема с открытым ключом RSA на чистом Rust",
            Self::Spanish => "Criptografía de clave pública RSA en Rust puro",
        }
    }

    pub fn pkg_plist_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Apple Property List (PList) 二進位與 XML 格式編解碼",
            Self::SimplifiedChinese => "Apple Property List (PList) 二进制与 XML 格式编解码",
            Self::English => "Apple Property List (PList) binary and XML serializer",
            Self::French => "Encodeur et décodeur de listes de propriétés Apple (PList)",
            Self::German => "Apple Property List (PList) Binär- und XML-Parser",
            Self::Italian => "Serializzatore binario e XML Apple Property List (PList)",
            Self::Japanese => "Apple Property List (PList) バイナリおよびXMLエンコーダー",
            Self::Korean => "Apple Property List (PList) 바이너리 및 XML 인코더/디코더",
            Self::PortugueseBrazil => "Serializador binário e XML de listas de propriedades da Apple (PList)",
            Self::PortuguesePortugal => "Serializador binário e XML de listas de propriedades da Apple (PList)",
            Self::Indonesian => "Serializer biner dan XML Apple Property List (PList)",
            Self::Vietnamese => "Bộ tuần tự hóa nhị phân và XML cho Apple Property List (PList)",
            Self::Russian => "Парсер бинарных и XML файлов Apple Property List (PList)",
            Self::Spanish => "Serializador binario y XML de listas de propiedades de Apple (PList)",
        }
    }

    pub fn pkg_serde_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Rust 業界標準通用資料序列化與反序列化框架",
            Self::SimplifiedChinese => "Rust 行业标准通用数据序列化与反序列化框架",
            Self::English => "Standard data serialization and deserialization framework",
            Self::French => "Framework standard de sérialisation et désérialisation de données",
            Self::German => "Standard-Framework für Datenserialisierung und -deserialisierung",
            Self::Italian => "Framework standard per la serializzazione e deserializzazione dei dati",
            Self::Japanese => "Rust 標準汎用データシリアライズ・デシリアライズフレームワーク",
            Self::Korean => "Rust 표준 데이터 직렬화 및 역직렬화 프레임워크",
            Self::PortugueseBrazil => "Framework padrão de serialização e desserialização de dados",
            Self::PortuguesePortugal => "Framework padrão de serialização e desserialização de dados",
            Self::Indonesian => "Kerangka kerja serialisasi dan deserialisasi data standar",
            Self::Vietnamese => "Framework tiêu chuẩn để tuần tự hóa và giải tuần tự hóa dữ liệu",
            Self::Russian => "Стандартный фреймворк сериализации и десериализации данных",
            Self::Spanish => "Framework estándar de serialización y deserialización de datos",
        }
    }

    pub fn pkg_uuid_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "RFC 4122 通用唯一識別碼 (UUIDv4) 生成與解析",
            Self::SimplifiedChinese => "RFC 4122 通用唯一标识符 (UUIDv4) 生成与解析",
            Self::English => "RFC 4122 Universally Unique Identifier (UUIDv4) generator",
            Self::French => "Générateur et analyseur d'identifiants uniques universels (UUIDv4)",
            Self::German => "RFC 4122 Universally Unique Identifier (UUIDv4) Generator",
            Self::Italian => "Generatore e parser di identificatori univoci universali (UUIDv4)",
            Self::Japanese => "RFC 4122 汎用一意識別子 (UUIDv4) 生成および解析",
            Self::Korean => "RFC 4122 범용 고유 식별자 (UUIDv4) 생성 및 파서",
            Self::PortugueseBrazil => "Gerador e analisador de identificadores universais exclusivos (UUIDv4)",
            Self::PortuguesePortugal => "Gerador e analisador de identificadores universais exclusivos (UUIDv4) RFC 4122",
            Self::Indonesian => "Pembuat Pengenal Unik Universal RFC 4122 (UUIDv4)",
            Self::Vietnamese => "Bộ tạo và phân tích mã định danh duy nhất toàn cầu RFC 4122 (UUIDv4)",
            Self::Russian => "Генератор универсальных уникальных идентификаторов (UUIDv4)",
            Self::Spanish => "Generador y analizador de identificadores únicos universales (UUIDv4)",
        }
    }

    pub fn pkg_tray_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Windows 系統匣常駐圖示與原生右鍵選單整合",
            Self::SimplifiedChinese => "Windows 托盘常驻图标与原生右键菜单集成",
            Self::English => "System tray icon and native context menu integration",
            Self::French => "Icône de la zone de notification et menu contextuel natif",
            Self::German => "Systemtray-Symbol und natives Kontextmenü",
            Self::Italian => "Icona nell'area di notifica e menu contestuale nativo",
            Self::Japanese => "システムトレイ常駐アイコンおよびネイティブメニュー統合",
            Self::Korean => "시스템 트레이 상주 아이콘 및 네이티브 메뉴 통합",
            Self::PortugueseBrazil => "Ícone na bandeja do sistema e menu de contexto nativo",
            Self::PortuguesePortugal => "Ícone na área de notificação do sistema e integração de menu de contexto nativo",
            Self::Indonesian => "Ikon baki sistem dan integrasi menu konteks asli",
            Self::Vietnamese => "Tích hợp biểu tượng khay hệ thống và menu ngữ cảnh gốc",
            Self::Russian => "Значок в системном трее и нативное контекстное меню",
            Self::Spanish => "Icono en la bandeja del sistema y menú contextual nativo",
        }
    }

    pub fn pkg_tracing_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "非同步應用程式結構化事件診斷與追蹤記錄",
            Self::SimplifiedChinese => "异步应用程序结构化事件诊断与追踪日志",
            Self::English => "Application-level tracing and structured diagnostics",
            Self::French => "Diagnostics structurés et traçage d'événements asynchrones",
            Self::German => "Strukturierte Diagnose und asynchrones Event-Tracing",
            Self::Italian => "Diagnostica strutturata e tracciamento eventi asincroni",
            Self::Japanese => "非同期アプリケーション向け構造化イベント診断・トレース",
            Self::Korean => "비동기 애플리케이션 구조화된 이벤트 진단 및 추적 로그",
            Self::PortugueseBrazil => "Diagnóstico estruturado e rastreamento de eventos assíncronos",
            Self::PortuguesePortugal => "Diagnóstico estruturado e rastreio de eventos ao nível da aplicação",
            Self::Indonesian => "Pelacakan tingkat aplikasi dan diagnostik terstruktur",
            Self::Vietnamese => "Ghi vết mức ứng dụng và chẩn đoán có cấu trúc",
            Self::Russian => "Структурированная диагностика и трассировка асинхронных событий",
            Self::Spanish => "Diagnóstico estructurado y rastreo de eventos asíncronos",
        }
    }

    pub fn pkg_err_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "強型別定義與彈性鏈路錯誤處理庫 (Anyhow / Thiserror)",
            Self::SimplifiedChinese => "强类型定义与弹性链路错误处理库 (Anyhow / Thiserror)",
            Self::English => "Flexible and typed error handling library (Anyhow / Thiserror)",
            Self::French => "Gestion d'erreurs flexible et typée (Anyhow / Thiserror)",
            Self::German => "Typisierte und flexible Fehlerbehandlung (Anyhow / Thiserror)",
            Self::Italian => "Gestione errori tipizzata e flessibile (Anyhow / Thiserror)",
            Self::Japanese => "型安全かつ柔軟なエラーハンドリングライブラリ (Anyhow / Thiserror)",
            Self::Korean => "타입 안정성 및 유연한 오류 처리 라이브러리 (Anyhow / Thiserror)",
            Self::PortugueseBrazil => "Tratamento de erros tipificado e flexível (Anyhow / Thiserror)",
            Self::PortuguesePortugal => "Tratamento de erros tipificado e flexível (Anyhow / Thiserror)",
            Self::Indonesian => "Pustaka penanganan kesalahan yang fleksibel dan berjenis (Anyhow / Thiserror)",
            Self::Vietnamese => "Thư viện xử lý lỗi linh hoạt và có định kiểu (Anyhow / Thiserror)",
            Self::Russian => "Гибкая и типизированная обработка ошибок (Anyhow / Thiserror)",
            Self::Spanish => "Gestión de errores flexible y tipificada (Anyhow / Thiserror)",
        }
    }

    pub fn pkg_image_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "影像格式解析與圖示處理庫 (Image Processing)",
            Self::SimplifiedChinese => "图像格式解析与图标处理库 (Image Processing)",
            Self::English => "Image format decoding and processing library",
            Self::French => "Bibliothèque de décodage et traitement d'images",
            Self::German => "Bildformat-Dekodierungs- und Verarbeitungsbibliothek",
            Self::Italian => "Libreria di decodifica ed elaborazione immagini",
            Self::Japanese => "画像フォーマットのデコードと処理ライブラリ",
            Self::Korean => "이미지 포맷 디코딩 및 처리 라이브러리",
            Self::PortugueseBrazil => "Biblioteca de decodificação e processamento de imagem",
            Self::PortuguesePortugal => "Biblioteca de descodificação e processamento de formatos de imagem",
            Self::Indonesian => "Pustaka decoding dan pemrosesan format gambar",
            Self::Vietnamese => "Thư viện giải mã và xử lý định dạng hình ảnh",
            Self::Russian => "Библиотека декодирования и обработки изображений",
            Self::Spanish => "Biblioteca de decodificación y procesamiento de imágenes",
        }
    }

    pub fn pkg_biquad_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "10 頻段等化器與低架雙二階濾波器 (Biquad IIR Filters)",
            Self::SimplifiedChinese => "10 频段等化器与低架双二阶滤波器 (Biquad IIR Filters)",
            Self::English => "Biquad IIR audio filters for 10-band EQ and warmth enhancement",
            Self::French => "Filtres audio biquad IIR pour l'égaliseur et l'amplification",
            Self::German => "Biquad-IIR-Audiofilter für Equalizer und Klangveredelung",
            Self::Italian => "Filtri audio IIR biquad per equalizzatore e calore vinilico",
            Self::Japanese => "10バンドEQおよび暖かみ補正用 Biquad IIR フィルター",
            Self::Korean => "10밴드 EQ 및 아날로그 웜스를 위한 Biquad IIR 오디오 필터",
            Self::PortugueseBrazil => "Filtros de áudio biquad IIR para equalizador e calor analógico",
            Self::PortuguesePortugal => "Filtros de áudio biquad IIR para equalizador de 10 bandas e calor analógico",
            Self::Indonesian => "Filter audio IIR biquad untuk EQ 10-band dan peningkatan kehangatan",
            Self::Vietnamese => "Bộ lọc âm thanh biquad IIR cho EQ 10 băng tần và tăng độ ấm âm thanh",
            Self::Russian => "Биквадратные IIR-фильтры для эквалайзера и теплоты",
            Self::Spanish => "Filtros de audio IIR biquad para ecualizador y calidez de vinilo",
        }
    }

    pub fn pkg_oximedia_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "高頻諧波修復 (HE) 與專業類比音訊音效引擎",
            Self::SimplifiedChinese => "高频谐波修复 (HE) 与专业模拟音频音效引擎",
            Self::English => "Audio DSP effects engine for harmonic excitation and analog warmth",
            Self::French => "Moteur d'effets DSP pour l'excitation harmonique et la chaleur",
            Self::German => "Audio-DSP-Effekt-Engine für Harmonic Exciter und Sättigung",
            Self::Italian => "Motore di effetti DSP per eccitazione armonica e calore analogico",
            Self::Japanese => "ハーモニック・エキサイターおよびアナログ効果用 DSP エンジン",
            Self::Korean => "하모닉 익サイ터 및 아날로그 효과를 위한 오디오 DSP 엔진",
            Self::PortugueseBrazil => "Motor de efeitos DSP de áudio para excitação harmônica e calor",
            Self::PortuguesePortugal => "Motor de efeitos DSP de áudio para excitação harmónica e calor analógico",
            Self::Indonesian => "Mesin efek DSP audio untuk eksitasi harmonik dan kehangatan analog",
            Self::Vietnamese => "Công cụ hiệu ứng âm thanh DSP cho kích thích họa âm và độ ấm analog",
            Self::Russian => "DSP-движок аудиоэффектов для гармонического экситера",
            Self::Spanish => "Motor de efectos de audio DSP para excitación armónica y calidez",
        }
    }

    pub fn pkg_realfft_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "16 頻段即時動態音訊頻譜快速傅立葉變換加速庫",
            Self::SimplifiedChinese => "16 频段实时动态音频频谱快速傅里叶变换加速库",
            Self::English => "Real-to-complex FFT acceleration for real-time audio spectrum",
            Self::French => "Accélération FFT réel-complexe pour le spectre audio en temps réel",
            Self::German => "Echtzeit-FFT-Beschleunigung für den 16-Band-Spektrumanalysator",
            Self::Italian => "Accelerazione FFT per analizzatore di spettro audio in tempo reale",
            Self::Japanese => "16バンド・リアルタイム音響スペクトラム解析用 高速FFTライブラリ",
            Self::Korean => "16밴드 실시간 오디오 스펙트럼 분석을 위한 초고속 FFT 라이브러리",
            Self::PortugueseBrazil => "Aceleração FFT para analisador de espectro de áudio em tempo real",
            Self::PortuguesePortugal => "Aceleração FFT para analisador de espetro de áudio em tempo real",
            Self::Indonesian => "Akselerasi FFT nyata-ke-kompleks untuk spektrum audio waktu nyata",
            Self::Vietnamese => "Tăng tốc biến đổi FFT thực sang phức cho phổ âm thanh thời gian thực",
            Self::Russian => "Быстрое преобразование Фурье (FFT) для спектроанализатора",
            Self::Spanish => "Aceleración FFT para analizador de espectro de audio en tiempo real",
        }
    }

    pub fn pkg_windows_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Windows 全域媒體控制 (GSMTC) 與 Win32 系統 API 綁定",
            Self::SimplifiedChinese => "Windows 全局媒体控制 (GSMTC) 与 Win32 系统 API 绑定",
            Self::English => "Windows System Media Transport Controls (GSMTC) & Win32 APIs",
            Self::French => "Contrôles multimédias Windows (GSMTC) et liaisons API Win32",
            Self::German => "Windows System Media Transport Controls (GSMTC) & Win32-APIs",
            Self::Italian => "Controlli multimediali di sistema Windows (GSMTC) e API Win32",
            Self::Japanese => "Windows グローバルメディアコントロール (GSMTC) および Win32 API",
            Self::Korean => "Windows 전역 미디어 제어 (GSMTC) 및 Win32 시스템 API",
            Self::PortugueseBrazil => "Controles de mídia do sistema Windows (GSMTC) e APIs Win32",
            Self::PortuguesePortugal => "Controlos de multimédia do sistema Windows (GSMTC) e APIs Win32",
            Self::Indonesian => "Kontrol Transportasi Media Sistem Windows (GSMTC) & API Win32",
            Self::Vietnamese => "Điều khiển truyền phát đa phương tiện hệ thống Windows (GSMTC) & API Win32",
            Self::Russian => "Системные элементы управления мультимедиа Windows (GSMTC) и Win32 API",
            Self::Spanish => "Controles multimedia del sistema Windows (GSMTC) y APIs Win32",
        }
    }

    pub fn pkg_wry_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "跨平台原生 WebView2 渲染與自訂通訊協定整合庫 (Wry)",
            Self::SimplifiedChinese => "跨平台原生 WebView2 渲染与自定义协议集成库 (Wry)",
            Self::English => "Cross-platform native WebView2 rendering and custom protocol library",
            Self::French => "Bibliothèque de rendu WebView2 natif et protocoles personnalisés",
            Self::German => "Plattformübergreifende native WebView2-Rendering- und Protokollbibliothek",
            Self::Italian => "Libreria di rendering WebView2 nativa multipiattaforma e protocolli personalizzati",
            Self::Japanese => "クロスプラットフォームネイティブ WebView2 レンダリングおよびプロトコル統合ライブラリ",
            Self::Korean => "크로스 플랫폼 네이티브 WebView2 렌더링 및 프로토콜 통합 라이브러리",
            Self::PortugueseBrazil => "Biblioteca de renderização nativa WebView2 multiplataforma e protocolos personalizados",
            Self::PortuguesePortugal => "Biblioteca de renderização nativa WebView2 multiplataforma e protocolos personalizados",
            Self::Indonesian => "Rendering WebView2 asli lintas platform dan pustaka protokol kustom",
            Self::Vietnamese => "Thư viện giao thức tùy chỉnh và kết xuất WebView2 gốc đa nền tảng",
            Self::Russian => "Кроссплатформенная библиотека рендеринга нативного WebView2",
            Self::Spanish => "Biblioteca de renderizado nativo WebView2 multiplataforma y protocolos personalizados",
        }
    }

    pub fn pkg_threejs_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "WebGL 3D 視覺場景渲染、相機控制與頂點著色器函式庫 (Three.js)",
            Self::SimplifiedChinese => "WebGL 3D 视觉场景渲染、相机控制与顶点着色器库 (Three.js)",
            Self::English => "WebGL 3D scene rendering, camera controls, and shader library",
            Self::French => "Rendu de scènes 3D WebGL, contrôles de caméra et bibliothèque de shaders",
            Self::German => "WebGL 3D-Szenen-Rendering, Kamerasteuerung und Shader-Bibliothek",
            Self::Italian => "Rendering scene 3D WebGL, controlli telecamera e libreria shader",
            Self::Japanese => "WebGL 3D シーン描画、カメラ制御およびシェーダーライブラリ (Three.js)",
            Self::Korean => "WebGL 3D 씬 렌더링, 카메라 제어 및 셰이더 라이브러리 (Three.js)",
            Self::PortugueseBrazil => "Renderização de cena 3D WebGL, controles de câmera e biblioteca de shaders",
            Self::PortuguesePortugal => "Renderização de cenas 3D WebGL, controlos de câmara e biblioteca de shaders",
            Self::Indonesian => "Rendering pemandangan 3D WebGL, kontrol kamera, dan pustaka shader",
            Self::Vietnamese => "Kết xuất hoạt cảnh 3D WebGL, điều khiển camera và thư viện đổ bóng shader",
            Self::Russian => "Рендеринг 3D-сцен WebGL, управление камерой и библиотека шейдеров",
            Self::Spanish => "Renderizado de escenas 3D WebGL, controles de cámara y biblioteca de shaders",
        }
    }

    pub fn pkg_three_painterly_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Three.js 印象派油畫筆觸渲染與世界座標後製管線 (PainterlyPass)",
            Self::SimplifiedChinese => "Three.js 印象派油画笔触渲染与世界坐标后处理管线 (PainterlyPass)",
            Self::English => "Three.js impressionist painterly brush stroke post-processing pipeline (PainterlyPass)",
            Self::French => "Pipeline de post-traitement pictural Three.js aux touches impressionnistes (PainterlyPass)",
            Self::German => "Three.js impressionistische Pinselstrich-Post-Processing-Pipeline (PainterlyPass)",
            Self::Italian => "Pipeline di post-elaborazione a pennellate impressioniste Three.js (PainterlyPass)",
            Self::Japanese => "Three.js 印象派油彩筆致シェーダーおよびポストプロセスパイプライン (PainterlyPass)",
            Self::Korean => "Three.js 유화풍 인상파 붓터치 셰이더 및 후처리 파이프라인 (PainterlyPass)",
            Self::PortugueseBrazil => "Pipeline de pós-processamento de pinceladas impressionistas Three.js (PainterlyPass)",
            Self::PortuguesePortugal => "Pipeline de pós-processamento de pinceladas impressionistas Three.js (PainterlyPass)",
            Self::Indonesian => "Saluran pasca-pemrosesan goresan kuas pelukis impresionis Three.js (PainterlyPass)",
            Self::Vietnamese => "Quy trình hậu xử lý nét cọ hội họa trường phái ấn tượng Three.js (PainterlyPass)",
            Self::Russian => "Пост-процессинг живописных мазков кисти для Three.js (PainterlyPass)",
            Self::Spanish => "Canal de posprocesamiento de pinceladas impresionistas Three.js (PainterlyPass)",
        }
    }

    pub fn pkg_baked_debris_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "GPU 烘焙加速粒子爆炸碎裂與音訊律動擴散模擬 (BakedDebris)",
            Self::SimplifiedChinese => "GPU 烘焙加速粒子爆炸碎裂与音频律动扩散模拟 (BakedDebris)",
            Self::English => "GPU-baked particle collision debris & audio motion scatter simulation (BakedDebris)",
            Self::French => "Simulation de débris de collision et de diffusion audio accélérée par GPU (BakedDebris)",
            Self::German => "GPU-beschleunigte Partikelkollisions- & Audiomotions-Streuungssimulation (BakedDebris)",
            Self::Italian => "Simulazione particellare accelerata su GPU per diffusione audio 3D (BakedDebris)",
            Self::Japanese => "GPU事前計算による粒子衝突・爆発音響拡散シミュレーション (BakedDebris)",
            Self::Korean => "GPU 가속 파티클 충돌 폭발 및 오디오 리듬 분산 시뮬레이션 (BakedDebris)",
            Self::PortugueseBrazil => "Simulação de colisão de detritos de partículas acelerada por GPU (BakedDebris)",
            Self::PortuguesePortugal => "Simulação de colisão de detritos de partículas acelerada por GPU (BakedDebris)",
            Self::Indonesian => "Simulasi serpihan tabrakan partikel dan hamburan gerak audio yang dibakar GPU (BakedDebris)",
            Self::Vietnamese => "Mô phỏng mảnh vụn va chạm hạt nướng qua GPU & tán xạ chuyển động âm thanh (BakedDebris)",
            Self::Russian => "GPU-симуляция столкновения и разлета частиц под ритм звука (BakedDebris)",
            Self::Spanish => "Simulación de colisión de partículas acelerada por GPU (BakedDebris)",
        }
    }

    pub fn pkg_raw_window_handle_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win32 原生視窗控制代碼跨語言互通介面標準 (Raw Window Handle)",
            Self::SimplifiedChinese => "Win32 原生窗口句柄跨语言互通接口标准 (Raw Window Handle)",
            Self::English => "Native Win32 window handle interoperability standard",
            Self::French => "Standard d'interopérabilité des descripteurs de fenêtre natifs Win32",
            Self::German => "Interoperabilitätsstandard für native Win32-Fenster-Handles",
            Self::Italian => "Standard di interoperabilità degli handle di finestra nativi Win32",
            Self::Japanese => "Win32 ネイティブウィンドウハンドル相互運用標準インターフェース",
            Self::Korean => "Win32 네이티브 윈도우 핸들 상호 운용 표준 인터페이스",
            Self::PortugueseBrazil => "Padrão de interoperabilidade de identificadores de janela nativos Win32",
            Self::PortuguesePortugal => "Padrão de interoperabilidade de identificadores de janela nativos Win32",
            Self::Indonesian => "Standar interoperabilitas pegangan jendela asli Win32",
            Self::Vietnamese => "Tiêu chuẩn tương tác tay cầm cửa sổ Win32 gốc",
            Self::Russian => "Стандарт взаимодействия с нативными дескрипторами окон Win32",
            Self::Spanish => "Estándar de interoperabilidad de identificadores de ventana nativos Win32",
        }
    }

    pub fn pkg_serde_json_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "高效能 JSON 格式資料流序列化與跨行程訊息傳遞 (Serde JSON)",
            Self::SimplifiedChinese => "高性能 JSON 格式数据流序列化与跨进程消息传递 (Serde JSON)",
            Self::English => "High-performance JSON serialization for IPC message passing",
            Self::French => "Sérialisation JSON haute performance pour transmission de messages IPC",
            Self::German => "Hochleistungs-JSON-Serialisierung für IPC-Nachrichtenübertragung",
            Self::Italian => "Serializzazione JSON ad alte prestazioni per passaggio messaggi IPC",
            Self::Japanese => "プロセス間通信(IPC)メッセージ用 高性能 JSON シリアライザー",
            Self::Korean => "프로세스 간 통신(IPC)을 위한 고성능 JSON 직렬화 라이브러리",
            Self::PortugueseBrazil => "Serialização JSON de alto desempenho para passagem de mensagens IPC",
            Self::PortuguesePortugal => "Serialização JSON de alto desempenho para passagem de mensagens IPC",
            Self::Indonesian => "Serialisasi JSON berkinerja tinggi untuk pengiriman pesan IPC",
            Self::Vietnamese => "Tuần tự hóa JSON hiệu năng cao cho truyền thông điệp IPC",
            Self::Russian => "Высокопроизводительная сериализация JSON для сообщений IPC",
            Self::Spanish => "Serialización JSON de alto rendimiento para paso de mensajes IPC",
        }
    }

    // -------------------------------------------------------------
    // Tray & Notification Strings
    // -------------------------------------------------------------

    pub fn tray_open_panel(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "置中開啟控制面板",
            Self::SimplifiedChinese => "居中打开控制面板",
            Self::English => "Center & Open Control Panel",
            Self::French => "Ouvrir le panneau au centre",
            Self::German => "Bedienfeld zentriert öffnen",
            Self::Italian => "Apri pannello di controllo al centro",
            Self::Japanese => "中央にコントロールパネルを開く",
            Self::Korean => "제어판 화면 중앙에 열기",
            Self::PortugueseBrazil => "Abrir painel de controle centralizado",
            Self::PortuguesePortugal => "Abrir painel de controlo centralizado",
            Self::Indonesian => "Buka Panel Kontrol di Tengah",
            Self::Vietnamese => "Mở bảng điều khiển ở giữa màn hình",
            Self::Russian => "Открыть панель управления по центру",
            Self::Spanish => "Abrir panel de control centrado",
        }
    }

    pub fn tray_mute(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "靜音模式 (未靜音)",
            Self::SimplifiedChinese => "静音模式 (未静音)",
            Self::English => "Mute (Currently unmuted)",
            Self::French => "Mode silencieux (Son actif)",
            Self::German => "Stummschaltung (Nicht stumm)",
            Self::Italian => "Modalità silenziosa (Non muto)",
            Self::Japanese => "ミュートモード (非ミュート)",
            Self::Korean => "음소거 모드 (음소거 해제됨)",
            Self::PortugueseBrazil => "Modo mudo (Não silenciado)",
            Self::PortuguesePortugal => "Cortar som (Atualmente com som)",
            Self::Indonesian => "Bisukan (Saat ini tidak dibisukan)",
            Self::Vietnamese => "Tắt tiếng (Hiện đang bật tiếng)",
            Self::Russian => "Режим звука (Звук включен)",
            Self::Spanish => "Modo silencio (No silenciado)",
        }
    }

    pub fn tray_muted(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "✓ 靜音模式 (已靜音)",
            Self::SimplifiedChinese => "✓ 静音模式 (已静音)",
            Self::English => "✓ Mute (Currently muted)",
            Self::French => "✓ Mode silencieux (Son coupé)",
            Self::German => "✓ Stummschaltung (Stumm)",
            Self::Italian => "✓ Modalità silenziosa (Muto)",
            Self::Japanese => "✓ ミュートモード (ミュート中)",
            Self::Korean => "✓ 음소거 모드 (음소거됨)",
            Self::PortugueseBrazil => "✓ Modo mudo (Silenciado)",
            Self::PortuguesePortugal => "✓ Cortar som (Atualmente sem som)",
            Self::Indonesian => "✓ Bisu (Saat ini dibisukan)",
            Self::Vietnamese => "✓ Tắt tiếng (Hiện đang tắt tiếng)",
            Self::Russian => "✓ Режим звука (Звук отключен)",
            Self::Spanish => "✓ Modo silencio (Silenciado)",
        }
    }

    pub fn tray_reset_windows(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🖥 重設所有視窗至螢幕中央",
            Self::SimplifiedChinese => "🖥 重设所有窗口至屏幕中央",
            Self::English => "🖥 Reset all windows to screen center",
            Self::French => "🖥 Réinitialiser toutes les fenêtres au centre",
            Self::German => "🖥 Alle Fenster in Bildschirmmitte zentrieren",
            Self::Italian => "🖥 Reimposta tutte le finestre al centro",
            Self::Japanese => "🖥 すべてのウィンドウを画面中央にリセット",
            Self::Korean => "🖥 모든 창을 화면 중앙으로 재설정",
            Self::PortugueseBrazil => "🖥 Redefinir todas as janelas para o centro",
            Self::PortuguesePortugal => "🖥 Repor todas as janelas no centro",
            Self::Indonesian => "🖥 Atur ulang semua jendela ke tengah layar",
            Self::Vietnamese => "🖥 Đặt lại tất cả cửa sổ về giữa màn hình",
            Self::Russian => "🖥 Сбросить все окна в центр экрана",
            Self::Spanish => "🖥 Restablecer todas las ventanas al centro",
        }
    }

    pub fn tray_quit(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "結束 Win2Pod Bridge",
            Self::SimplifiedChinese => "退出 Win2Pod Bridge",
            Self::English => "Exit Win2Pod Bridge",
            Self::French => "Quitter Win2Pod Bridge",
            Self::German => "Win2Pod Bridge beenden",
            Self::Italian => "Esci da Win2Pod Bridge",
            Self::Japanese => "Win2Pod Bridge を終了",
            Self::Korean => "Win2Pod Bridge 종료",
            Self::PortugueseBrazil => "Sair do Win2Pod Bridge",
            Self::PortuguesePortugal => "Sair do Win2Pod Bridge",
            Self::Indonesian => "Keluar dari Win2Pod Bridge",
            Self::Vietnamese => "Thoát Win2Pod Bridge",
            Self::Russian => "Выйти из Win2Pod Bridge",
            Self::Spanish => "Salir de Win2Pod Bridge",
        }
    }

    pub fn tray_idle_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod Bridge - 待機中",
            Self::SimplifiedChinese => "Win2Pod Bridge - 待机中",
            Self::English => "Win2Pod Bridge - Idle",
            Self::French => "Win2Pod Bridge - En attente",
            Self::German => "Win2Pod Bridge - Bereit",
            Self::Italian => "Win2Pod Bridge - In attesa",
            Self::Japanese => "Win2Pod Bridge - 待機中",
            Self::Korean => "Win2Pod Bridge - 대기 중",
            Self::PortugueseBrazil => "Win2Pod Bridge - Em espera",
            Self::PortuguesePortugal => "Win2Pod Bridge - Em espera",
            Self::Indonesian => "Win2Pod Bridge - Siaga",
            Self::Vietnamese => "Win2Pod Bridge - Chờ",
            Self::Russian => "Win2Pod Bridge - Ожидание",
            Self::Spanish => "Win2Pod Bridge - En espera",
        }
    }

    pub fn tray_muted_text(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "靜音",
            Self::SimplifiedChinese => "静音",
            Self::English => "Muted",
            Self::French => "Muet",
            Self::German => "Stumm",
            Self::Italian => "Muto",
            Self::Japanese => "消音",
            Self::Korean => "음소거",
            Self::PortugueseBrazil => "Mudo",
            Self::PortuguesePortugal => "Sem som",
            Self::Indonesian => "Dibisukan",
            Self::Vietnamese => "Đã tắt tiếng",
            Self::Russian => "Без звука",
            Self::Spanish => "Silenciado",
        }
    }

    pub fn tray_connecting_tooltip(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("🔄 連線中：{name}"),
            Self::SimplifiedChinese => format!("🔄 连接中：{name}"),
            Self::English => format!("🔄 Connecting: {name}"),
            Self::French => format!("🔄 Connexion : {name}"),
            Self::German => format!("🔄 Verbinden: {name}"),
            Self::Italian => format!("🔄 Connessione: {name}"),
            Self::Japanese => format!("🔄 接続中：{name}"),
            Self::Korean => format!("🔄 연결 중: {name}"),
            Self::PortugueseBrazil => format!("🔄 Conectando: {name}"),
            Self::PortuguesePortugal => format!("🔄 A ligar: {name}"),
            Self::Indonesian => format!("🔄 Menghubungkan: {name}"),
            Self::Vietnamese => format!("🔄 Đang kết nối: {name}"),
            Self::Russian => format!("🔄 Подключение: {name}"),
            Self::Spanish => format!("🔄 Conectando: {name}"),
        }
    }

    pub fn tray_streaming_tooltip(&self, is_muted: bool, name: &str, vol_pct: i32) -> String {
        let vol_owned = format!("{vol_pct}%");
        let (icon, vol_str) = if is_muted {
            ("🔇", self.tray_muted_text())
        } else {
            ("🔊", vol_owned.as_str())
        };
        match self {
            Self::TraditionalChinese => format!("{icon} 串流中：{name} ({vol_str})"),
            Self::SimplifiedChinese => format!("{icon} 正在串流：{name} ({vol_str})"),
            Self::English => format!("{icon} Streaming: {name} ({vol_str})"),
            Self::French => format!("{icon} Lecture : {name} ({vol_str})"),
            Self::German => format!("{icon} Streaming: {name} ({vol_str})"),
            Self::Italian => format!("{icon} In riproduzione: {name} ({vol_str})"),
            Self::Japanese => format!("{icon} 再生中：{name} ({vol_str})"),
            Self::Korean => format!("{icon} 스트리밍 중: {name} ({vol_str})"),
            Self::PortugueseBrazil => format!("{icon} Transmitindo: {name} ({vol_str})"),
            Self::PortuguesePortugal => format!("{icon} A transmitir: {name} ({vol_str})"),
            Self::Indonesian => format!("{icon} Streaming: {name} ({vol_str})"),
            Self::Vietnamese => format!("{icon} Đang truyền phát: {name} ({vol_str})"),
            Self::Russian => format!("{icon} Трансляция: {name} ({vol_str})"),
            Self::Spanish => format!("{icon} Reproduciendo: {name} ({vol_str})"),
        }
    }

    pub fn tray_airpods_with_battery(&self, model: &str, battery_info: &str) -> String {
        match self {
            Self::TraditionalChinese | Self::SimplifiedChinese | Self::Japanese => {
                format!("🎧 {model}：{battery_info}")
            }
            _ => {
                format!("🎧 {model}: {battery_info}")
            }
        }
    }

    pub fn tray_airpods_status_only(&self, model: &str, status: &str) -> String {
        format!("🎧 {model} ({status})")
    }

    pub fn tray_error_tooltip(&self, err: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("Win2Pod Bridge - 錯誤: {err}"),
            Self::SimplifiedChinese => format!("Win2Pod Bridge - 错误: {err}"),
            Self::English => format!("Win2Pod Bridge - Error: {err}"),
            Self::French => format!("Win2Pod Bridge - Erreur : {err}"),
            Self::German => format!("Win2Pod Bridge - Fehler: {err}"),
            Self::Italian => format!("Win2Pod Bridge - Errore: {err}"),
            Self::Japanese => format!("Win2Pod Bridge - エラー: {err}"),
            Self::Korean => format!("Win2Pod Bridge - 오류: {err}"),
            Self::PortugueseBrazil => format!("Win2Pod Bridge - Erro: {err}"),
            Self::PortuguesePortugal => format!("Win2Pod Bridge - Erro: {err}"),
            Self::Indonesian => format!("Win2Pod Bridge - Kesalahan: {err}"),
            Self::Vietnamese => format!("Win2Pod Bridge - Lỗi: {err}"),
            Self::Russian => format!("Win2Pod Bridge - Ошибка: {err}"),
            Self::Spanish => format!("Win2Pod Bridge - Error: {err}"),
        }
    }

    pub fn nav_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "等化器",
            Self::SimplifiedChinese => "均衡器",
            Self::English => "Equalizer",
            Self::French => "Égaliseur",
            Self::German => "Equalizer",
            Self::Italian => "Equalizzatore",
            Self::Japanese => "イコライザー",
            Self::Korean => "이퀄라이저",
            Self::PortugueseBrazil => "Equalizador",
            Self::PortuguesePortugal => "Equalizador",
            Self::Indonesian => "Equalizer",
            Self::Vietnamese => "Bộ cân bằng âm thanh",
            Self::Russian => "Эквалайзер",
            Self::Spanish => "Ecualizador",
        }
    }

    pub fn tooltip_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟音訊等化器與即時頻譜",
            Self::SimplifiedChinese => "开启音频均衡器与实时频谱",
            Self::English => "Open Audio Equalizer & Real-time Spectrum",
            Self::French => "Ouvrir l'égaliseur et le spectre en temps réel",
            Self::German => "Audio-Equalizer & Echtzeit-Spektrum öffnen",
            Self::Italian => "Apri equalizzatore audio e spettro in tempo reale",
            Self::Japanese => "オーディオイコライザーとリアルタイムスペクトルを開く",
            Self::Korean => "오디오 이퀄라이저 및 실시간 스펙트럼 열기",
            Self::PortugueseBrazil => "Abrir equalizador de áudio e espectro em tempo real",
            Self::PortuguesePortugal => "Abrir equalizador de áudio e espetro em tempo real",
            Self::Indonesian => "Buka Equalizer Audio & Spektrum Waktu Nyata",
            Self::Vietnamese => "Mở Bộ cân bằng âm thanh & Phổ tần số thời gian thực",
            Self::Russian => "Открыть эквалайзер и спектр реального времени",
            Self::Spanish => "Abrir ecualizador de audio y espectro en tiempo real",
        }
    }

    pub fn tooltip_player(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod 播放器與等化器",
            Self::SimplifiedChinese => "Win2Pod 播放器与均衡器",
            Self::English => "Win2Pod Player & Equalizer",
            Self::French => "Lecteur & Égaliseur Win2Pod",
            Self::German => "Win2Pod Player & Equalizer",
            Self::Italian => "Lettore & Equalizzatore Win2Pod",
            Self::Japanese => "Win2Pod プレーヤー＆イコライザー",
            Self::Korean => "Win2Pod 플레이어 및 이퀄라이저",
            Self::PortugueseBrazil => "Player & Equalizador Win2Pod",
            Self::PortuguesePortugal => "Leitor & Equalizador Win2Pod",
            Self::Indonesian => "Pemutar & Equalizer Win2Pod",
            Self::Vietnamese => "Trình phát & Bộ cân bằng Win2Pod",
            Self::Russian => "Win2Pod Плеер и Эквалайзер",
            Self::Spanish => "Reproductor & Ecualizador Win2Pod",
        }
    }

    pub fn tooltip_pin_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "保持在最上層",
            Self::SimplifiedChinese => "保持在最顶层",
            Self::English => "Keep on Top",
            Self::French => "Garder au premier plan",
            Self::German => "Im Vordergrund halten",
            Self::Italian => "Mantieni in primo piano",
            Self::Japanese => "常に最前面に表示",
            Self::Korean => "항상 위에 고정",
            Self::PortugueseBrazil => "Manter no topo",
            Self::PortuguesePortugal => "Manter no topo",
            Self::Indonesian => "Tetap di Atas",
            Self::Vietnamese => "Ghim lên trên cùng",
            Self::Russian => "Поверх остальных окон",
            Self::Spanish => "Mantener en primer plano",
        }
    }

    pub fn tooltip_unpin_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "取消保持在最上層",
            Self::SimplifiedChinese => "取消保持在最顶层",
            Self::English => "Unpin from Top",
            Self::French => "Ne plus garder au premier plan",
            Self::German => "Nicht mehr im Vordergrund halten",
            Self::Italian => "Non mantenere più in primo piano",
            Self::Japanese => "最前面の固定を解除",
            Self::Korean => "항상 위 고정 해제",
            Self::PortugueseBrazil => "Desafixar do topo",
            Self::PortuguesePortugal => "Desafixar do topo",
            Self::Indonesian => "Lepas Pin dari Atas",
            Self::Vietnamese => "Bỏ ghim khỏi trên cùng",
            Self::Russian => "Открепить от верха",
            Self::Spanish => "Desanclar del primer plano",
        }
    }

    pub fn tooltip_pin_cluster(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "保持工作站叢集在最上層",
            Self::SimplifiedChinese => "保持工作站集群在最顶层",
            Self::English => "Keep Workstation Cluster on Top",
            Self::French => "Garder le cluster au premier plan",
            Self::German => "Cluster im Vordergrund halten",
            Self::Italian => "Mantieni il cluster in primo piano",
            Self::Japanese => "クラスタを常に最前面に表示",
            Self::Korean => "클러스터 항상 위에 고정",
            Self::PortugueseBrazil => "Manter o cluster no topo",
            Self::PortuguesePortugal => "Manter o cluster no topo",
            Self::Indonesian => "Tetap di Atas (Kluster)",
            Self::Vietnamese => "Ghim cụm lên trên cùng",
            Self::Russian => "Поверх остальных окон (кластер)",
            Self::Spanish => "Mantener el grupo en primer plano",
        }
    }

    pub fn tooltip_unpin_cluster(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "取消工作站叢集最上層",
            Self::SimplifiedChinese => "取消工作站集群最顶层",
            Self::English => "Unpin Workstation Cluster from Top",
            Self::French => "Ne plus garder le cluster au premier plan",
            Self::German => "Cluster nicht mehr im Vordergrund halten",
            Self::Italian => "Non mantenere più il cluster in primo piano",
            Self::Japanese => "クラスタの最前面固定を解除",
            Self::Korean => "클러스터 항상 위 고정 해제",
            Self::PortugueseBrazil => "Desafixar cluster do topo",
            Self::PortuguesePortugal => "Desafixar cluster do topo",
            Self::Indonesian => "Lepas Pin Kluster dari Atas",
            Self::Vietnamese => "Bỏ ghim cụm khỏi trên cùng",
            Self::Russian => "Открепить кластер от верха",
            Self::Spanish => "Desanclar el grupo del primer plano",
        }
    }

    pub fn opacity_menu_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "視窗透明度",
            Self::SimplifiedChinese => "窗口透明度",
            Self::English => "Window Opacity",
            Self::French => "Opacité de la fenêtre",
            Self::German => "Fenstertransparenz",
            Self::Italian => "Opacità della finestra",
            Self::Japanese => "ウィンドウの不透明度",
            Self::Korean => "창 불투명도",
            Self::PortugueseBrazil => "Opacidade da janela",
            Self::PortuguesePortugal => "Opacidade da janela",
            Self::Indonesian => "Opasitas Jendela",
            Self::Vietnamese => "Độ mờ của cửa sổ",
            Self::Russian => "Прозрачность окна",
            Self::Spanish => "Opacidad de la ventana",
        }
    }

    pub fn opacity_hover_peek(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "懸停時恢復實體 (Hover Peek)",
            Self::SimplifiedChinese => "悬停时恢复实体 (Hover Peek)",
            Self::English => "Restore on hover (Hover Peek)",
            Self::French => "Restaurer au survol (Hover Peek)",
            Self::German => "Beim Zeigen wiederherstellen (Hover Peek)",
            Self::Italian => "Ripristina al passaggio (Hover Peek)",
            Self::Japanese => "ホバー時に不透明に戻す (Hover Peek)",
            Self::Korean => "호버 시 불투명 복원 (Hover Peek)",
            Self::PortugueseBrazil => "Restaurar ao passar o mouse (Hover Peek)",
            Self::PortuguesePortugal => "Restaurar ao passar o rato (Hover Peek)",
            Self::Indonesian => "Pulihkan saat kursor di atasnya (Hover Peek)",
            Self::Vietnamese => "Khôi phục khi di chuột (Hover Peek)",
            Self::Russian => "Восстанавливать при наведении (Hover Peek)",
            Self::Spanish => "Restaurar al pasar el ratón (Hover Peek)",
        }
    }

    pub fn opacity_alt_wheel_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "提示：按住 Alt + 滾輪可無級微調",
            Self::SimplifiedChinese => "提示：按住 Alt + 滚轮可无级微调",
            Self::English => "Tip: Hold Alt + Scroll to fine-tune",
            Self::French => "Astuce : Alt + Molette pour ajuster",
            Self::German => "Tipp: Alt + Mausrad zum Anpassen",
            Self::Italian => "Suggerimento: Alt + Rotellina per regolare",
            Self::Japanese => "ヒント: Alt + スクロールで無段階調整",
            Self::Korean => "팁: Alt + 스크롤로 연속 미세 조정",
            Self::PortugueseBrazil => "Dica: Alt + Rolagem para ajustar",
            Self::PortuguesePortugal => "Dica: Alt + Roda para ajustar",
            Self::Indonesian => "Tips: Alt + Gulir untuk menyesuaikan",
            Self::Vietnamese => "Mẹo: Alt + Cuộn chuột để tinh chỉnh",
            Self::Russian => "Alt + колесо мыши для точной настройки",
            Self::Spanish => "Consejo: Alt + Rueda para ajustar",
        }
    }

    pub fn tooltip_visualizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟 3D 油畫音訊視覺化 (F11)",
            Self::SimplifiedChinese => "开启 3D 油画音频可视化 (F11)",
            Self::English => "Open 3D Paint Pulse Visualizer (F11)",
            Self::French => "Ouvrir le visualiseur 3D Paint Pulse (F11)",
            Self::German => "3D-Paint-Pulse-Visualisierer öffnen (F11)",
            Self::Italian => "Apri visualizzatore 3D Paint Pulse (F11)",
            Self::Japanese => "3D 油彩オーディオビジュアライザーを開く (F11)",
            Self::Korean => "3D 유화 오디오 시각화 열기 (F11)",
            Self::PortugueseBrazil => "Abrir visualizador de áudio 3D Paint Pulse (F11)",
            Self::PortuguesePortugal => "Abrir visualizador de áudio 3D Paint Pulse (F11)",
            Self::Indonesian => "Buka Visualizer 3D Paint Pulse (F11)",
            Self::Vietnamese => "Mở Trình hiển thị hình ảnh âm thanh 3D Paint Pulse (F11)",
            Self::Russian => "Открыть 3D визуализатор Paint Pulse (F11)",
            Self::Spanish => "Abrir visualizador 3D Paint Pulse (F11)",
        }
    }

    pub fn tooltip_visualizer_close(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關閉 3D 音訊視覺化",
            Self::SimplifiedChinese => "关闭 3D 音频可视化",
            Self::English => "Close 3D Visualizer",
            Self::French => "Fermer le visualiseur 3D",
            Self::German => "3D-Visualisierer schließen",
            Self::Italian => "Chiudi visualizzatore 3D",
            Self::Japanese => "3D ビジュアライザーを閉じる",
            Self::Korean => "3D 시각화 닫기",
            Self::PortugueseBrazil => "Fechar visualizador 3D",
            Self::PortuguesePortugal => "Fechar visualizador 3D",
            Self::Indonesian => "Tutup Visualizer 3D",
            Self::Vietnamese => "Đóng trình hiển thị 3D",
            Self::Russian => "Закрыть 3D визуализатор",
            Self::Spanish => "Cerrar visualizador 3D",
        }
    }

    pub fn player_no_track(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "目前無播放中的媒體",
            Self::SimplifiedChinese => "当前无播放中的媒体",
            Self::English => "No Media Playing",
            Self::French => "Aucun média en cours",
            Self::German => "Keine Medienwiedergabe",
            Self::Italian => "Nessun file multimediale in riproduzione",
            Self::Japanese => "再生中のメディアはありません",
            Self::Korean => "재생 중인 미디어 없음",
            Self::PortugueseBrazil => "Nenhuma mídia em reprodução",
            Self::PortuguesePortugal => "Nenhum conteúdo multimédia em reprodução",
            Self::Indonesian => "Tidak Ada Media yang Diputar",
            Self::Vietnamese => "Không có phương tiện nào đang phát",
            Self::Russian => "Нет воспроизводимых медиа",
            Self::Spanish => "Sin reproducción de medios",
        }
    }

    pub fn player_unknown_artist(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "未知演出者",
            Self::SimplifiedChinese => "未知艺术家",
            Self::English => "Unknown Artist",
            Self::French => "Artiste inconnu",
            Self::German => "Unbekannter Künstler",
            Self::Italian => "Artista sconosciuto",
            Self::Japanese => "不明なアーティスト",
            Self::Korean => "알 수 없는 아티스트",
            Self::PortugueseBrazil => "Artista Desconhecido",
            Self::PortuguesePortugal => "Artista desconhecido",
            Self::Indonesian => "Artis Tidak Dikenal",
            Self::Vietnamese => "Nghệ sĩ chưa xác định",
            Self::Russian => "Неизвестный исполнитель",
            Self::Spanish => "Artista Desconocido",
        }
    }

    pub fn player_prev(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "上一首",
            Self::SimplifiedChinese => "上一首",
            Self::English => "Previous Track",
            Self::French => "Piste précédente",
            Self::German => "Vorheriger Titel",
            Self::Italian => "Traccia precedente",
            Self::Japanese => "前の曲",
            Self::Korean => "이전 곡",
            Self::PortugueseBrazil => "Faixa Anterior",
            Self::PortuguesePortugal => "Faixa anterior",
            Self::Indonesian => "Trek Sebelumnya",
            Self::Vietnamese => "Bản nhạc trước",
            Self::Russian => "Предыдущий трек",
            Self::Spanish => "Pista anterior",
        }
    }

    pub fn player_play_pause(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "播放 / 暫停",
            Self::SimplifiedChinese => "播放 / 暂停",
            Self::English => "Play / Pause",
            Self::French => "Lecture / Pause",
            Self::German => "Wiedergabe / Pause",
            Self::Italian => "Riproduci / Pausa",
            Self::Japanese => "再生 / 一時停止",
            Self::Korean => "재생 / 일시정지",
            Self::PortugueseBrazil => "Reproduzir / Pausar",
            Self::PortuguesePortugal => "Reproduzir / Pausar",
            Self::Indonesian => "Putar / Jeda",
            Self::Vietnamese => "Phát / Tạm dừng",
            Self::Russian => "Воспроизведение / Пауза",
            Self::Spanish => "Reproducir / Pausa",
        }
    }

    pub fn player_stop(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "停止",
            Self::SimplifiedChinese => "停止",
            Self::English => "Stop",
            Self::French => "Arrêter",
            Self::German => "Stopp",
            Self::Italian => "Ferma",
            Self::Japanese => "停止",
            Self::Korean => "정지",
            Self::PortugueseBrazil => "Parar",
            Self::PortuguesePortugal => "Parar",
            Self::Indonesian => "Berhenti",
            Self::Vietnamese => "Dừng",
            Self::Russian => "Остановить",
            Self::Spanish => "Detener",
        }
    }

    pub fn player_next(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "下一首",
            Self::SimplifiedChinese => "下一首",
            Self::English => "Next Track",
            Self::French => "Piste suivante",
            Self::German => "Nächster Titel",
            Self::Italian => "Traccia successiva",
            Self::Japanese => "次の曲",
            Self::Korean => "다음 곡",
            Self::PortugueseBrazil => "Próxima Faixa",
            Self::PortuguesePortugal => "Faixa seguinte",
            Self::Indonesian => "Trek Berikutnya",
            Self::Vietnamese => "Bản nhạc tiếp theo",
            Self::Russian => "Следующий трек",
            Self::Spanish => "Pista siguiente",
        }
    }

    pub fn player_click_to_open_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "點擊叫出播放來源程式",
            Self::SimplifiedChinese => "点击唤起播放来源程序",
            Self::English => "Click to bring up media player",
            Self::French => "Cliquer pour afficher le lecteur",
            Self::German => "Klicken, um Player aufzurufen",
            Self::Italian => "Clicca per aprire il lettore multimediale",
            Self::Japanese => "クリックして再生元アプリを表示",
            Self::Korean => "클릭하여 재생 앱 열기",
            Self::PortugueseBrazil => "Clique para abrir o reprodutor de mídia",
            Self::PortuguesePortugal => "Clique para abrir o leitor de multimédia",
            Self::Indonesian => "Klik untuk menampilkan pemutar media",
            Self::Vietnamese => "Nhấp để đưa trình phát phương tiện lên trên",
            Self::Russian => "Нажмите, чтобы открыть медиаплеер",
            Self::Spanish => "Haga clic para abrir el reproductor",
        }
    }

    pub fn balance_center(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "◄ 置中 ►",
            Self::SimplifiedChinese => "◄ 居中 ►",
            Self::English => "◄ Center ►",
            Self::French => "◄ Centre ►",
            Self::German => "◄ Mitte ►",
            Self::Italian => "◄ Centro ►",
            Self::Japanese => "◄ 中央 ►",
            Self::Korean => "◄ 중앙 ►",
            Self::PortugueseBrazil => "◄ Centro ►",
            Self::PortuguesePortugal => "◄ Centro ►",
            Self::Indonesian => "◄ Tengah ►",
            Self::Vietnamese => "◄ Giữa ►",
            Self::Russian => "◄ Центр ►",
            Self::Spanish => "◄ Centro ►",
        }
    }

    pub fn balance_left(&self, pct: i32) -> String {
        match self {
            Self::TraditionalChinese => format!("◄ 左 {pct}%"),
            Self::SimplifiedChinese => format!("◄ 左 {pct}%"),
            Self::English => format!("◄ L {pct}%"),
            Self::French => format!("◄ G {pct}%"),
            Self::German => format!("◄ L {pct}%"),
            Self::Italian => format!("◄ S {pct}%"),
            Self::Japanese => format!("◄ 左 {pct}%"),
            Self::Korean => format!("◄ 좌 {pct}%"),
            Self::PortugueseBrazil => format!("◄ E {pct}%"),
            Self::PortuguesePortugal => format!("◄ E {pct}%"),
            Self::Indonesian => format!("◄ Kiri {pct}%"),
            Self::Vietnamese => format!("◄ T {pct}%"),
            Self::Russian => format!("◄ Л {pct}%"),
            Self::Spanish => format!("◄ I {pct}%"),
        }
    }

    pub fn balance_right(&self, pct: i32) -> String {
        match self {
            Self::TraditionalChinese => format!("{pct}% 右 ►"),
            Self::SimplifiedChinese => format!("{pct}% 右 ►"),
            Self::English => format!("{pct}% R ►"),
            Self::French => format!("{pct}% D ►"),
            Self::German => format!("{pct}% R ►"),
            Self::Italian => format!("{pct}% D ►"),
            Self::Japanese => format!("{pct}% 右 ►"),
            Self::Korean => format!("{pct}% 우 ►"),
            Self::PortugueseBrazil => format!("{pct}% D ►"),
            Self::PortuguesePortugal => format!("{pct}% D ►"),
            Self::Indonesian => format!("{pct}% Kanan ►"),
            Self::Vietnamese => format!("{pct}% P ►"),
            Self::Russian => format!("{pct}% П ►"),
            Self::Spanish => format!("{pct}% D ►"),
        }
    }

    pub fn balance_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "左右聲道平衡（按兩下還原置中）",
            Self::SimplifiedChinese => "左右声道平衡（双击还原居中）",
            Self::English => "Stereo Balance (Double-click to reset to center)",
            Self::French => "Balance stéréo (Double-cliquer pour réinitialiser au centre)",
            Self::German => "Stereo-Balance (Doppelklick zum Zurücksetzen auf die Mitte)",
            Self::Italian => "Bilanciamento stereo (Doppio clic per reimpostare al centro)",
            Self::Japanese => "ステレオバランス（ダブルクリックで中央に戻す）",
            Self::Korean => "스테레오 밸런스 (더블 클릭하여 중앙으로 리셋)",
            Self::PortugueseBrazil => "Balanço estéreo (Clique duplo para centralizar)",
            Self::PortuguesePortugal => "Balanço estéreo (Duplo clique para centrar)",
            Self::Indonesian => "Keseimbangan Stereo (Klik dua kali untuk mengatur ulang ke tengah)",
            Self::Vietnamese => "Cân bằng âm thanh nổi (Nhấp đúp để đặt lại về giữa)",
            Self::Russian => "Стерео баланс (Дважды щелкните для сброса по центру)",
            Self::Spanish => "Balance estéreo (Haga doble clic para centrar)",
        }
    }

    pub fn btn_eq(&self, enabled: bool) -> &'static str {
        if enabled {
            match self {
                Self::TraditionalChinese => "♪ EQ:開",
                Self::SimplifiedChinese => "♪ EQ:开",
                Self::English => "♪ EQ:ON",
                Self::French => "♪ EQ:ON",
                Self::German => "♪ EQ:EIN",
                Self::Italian => "♪ EQ:ON",
                Self::Japanese => "♪ EQ:入",
                Self::Korean => "♪ EQ:켜짐",
                Self::PortugueseBrazil => "♪ EQ:LIG",
                Self::PortuguesePortugal => "♪ EQ:LIG",
                Self::Indonesian => "♪ EQ:AKTIF",
                Self::Vietnamese => "♪ EQ:BẬT",
                Self::Russian => "♪ EQ:ВКЛ",
                Self::Spanish => "♪ EQ:ON",
            }
        } else {
            "♪ EQ"
        }
    }

    pub fn eq_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊等化器 (Equalizer)",
            Self::SimplifiedChinese => "音频均衡器 (Equalizer)",
            Self::English => "Audio Equalizer",
            Self::French => "Égaliseur audio",
            Self::German => "Audio-Equalizer",
            Self::Italian => "Equalizzatore audio",
            Self::Japanese => "オーディオイコライザー",
            Self::Korean => "오디오 이퀄라이저",
            Self::PortugueseBrazil => "Equalizador de Áudio",
            Self::PortuguesePortugal => "Equalizador de Áudio",
            Self::Indonesian => "Equalizer Audio",
            Self::Vietnamese => "Bộ cân bằng âm thanh",
            Self::Russian => "Аудио эквалайзер",
            Self::Spanish => "Ecualizador de Audio",
        }
    }

    pub fn eq_enable(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "啟用等化器",
            Self::SimplifiedChinese => "启用均衡器",
            Self::English => "Enable Equalizer",
            Self::French => "Activer l'égaliseur",
            Self::German => "Equalizer aktivieren",
            Self::Italian => "Abilita equalizzatore",
            Self::Japanese => "イコライザーを有効化",
            Self::Korean => "이퀄라이저 활성화",
            Self::PortugueseBrazil => "Ativar equalizador",
            Self::PortuguesePortugal => "Ativar equalizador",
            Self::Indonesian => "Aktifkan Equalizer",
            Self::Vietnamese => "Bật bộ cân bằng",
            Self::Russian => "Включить эквалайзер",
            Self::Spanish => "Activar ecualizador",
        }
    }

    pub fn eq_presets_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音效風格預設：",
            Self::SimplifiedChinese => "音效风格预设：",
            Self::English => "Sound Presets:",
            Self::French => "Préréglages sonores :",
            Self::German => "Klang-Voreinstellungen:",
            Self::Italian => "Preimpostazioni audio:",
            Self::Japanese => "サウンドプリセット：",
            Self::Korean => "사운드 프리셋:",
            Self::PortugueseBrazil => "Predefinições de som:",
            Self::PortuguesePortugal => "Predefinições de som:",
            Self::Indonesian => "Preset Suara:",
            Self::Vietnamese => "Cài đặt âm thanh trước:",
            Self::Russian => "Предустановки звука:",
            Self::Spanish => "Ajustes predefinidos de sonido:",
        }
    }

    pub fn eq_reset(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "↺ 歸零重設",
            Self::SimplifiedChinese => "↺ 归零重置",
            Self::English => "↺ Reset Flat",
            Self::French => "↺ Réinitialiser",
            Self::German => "↺ Zurücksetzen",
            Self::Italian => "↺ Ripristina",
            Self::Japanese => "↺ リセット",
            Self::Korean => "↺ 재설정",
            Self::PortugueseBrazil => "↺ Redefinir",
            Self::PortuguesePortugal => "↺ Repor Plano",
            Self::Indonesian => "↺ Atur Ulang Datar",
            Self::Vietnamese => "↺ Đặt lại phẳng (Flat)",
            Self::Russian => "↺ Сброс",
            Self::Spanish => "↺ Restablecer",
        }
    }

    pub fn eq_spectrum_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "即時音訊頻譜響應",
            Self::SimplifiedChinese => "实时音频频谱响应",
            Self::English => "Real-Time Spectrum Response",
            Self::French => "Réponse spectrale en temps réel",
            Self::German => "Echtzeit-Spektralantwort",
            Self::Italian => "Risposta spettrale in tempo reale",
            Self::Japanese => "リアルタイムオーディオスペクトル",
            Self::Korean => "실시간 오디오 스펙트럼 반응",
            Self::PortugueseBrazil => "Resposta espectral em tempo real",
            Self::PortuguesePortugal => "Resposta de espetro em tempo real",
            Self::Indonesian => "Respons Spektrum Waktu Nyata",
            Self::Vietnamese => "Đáp ứng phổ tần số thời gian thực",
            Self::Russian => "Спектральный отклик в реальном времени",
            Self::Spanish => "Respuesta de espectro en tiempo real",
        }
    }

    pub fn eq_preset_name(&self, preset: crate::controller::EqPreset) -> &'static str {
        use crate::controller::EqPreset;
        match preset {
            EqPreset::Jazz => match self {
                Self::TraditionalChinese => "爵士",
                Self::SimplifiedChinese => "爵士",
                Self::English => "Jazz",
                Self::French => "Jazz",
                Self::German => "Jazz",
                Self::Italian => "Jazz",
                Self::Japanese => "ジャズ",
                Self::Korean => "재즈",
                Self::PortugueseBrazil => "Jazz",
                Self::PortuguesePortugal => "Jazz",
                Self::Indonesian => "Jazz",
                Self::Vietnamese => "Jazz",
                Self::Russian => "Джаз",
                Self::Spanish => "Jazz",
            },
            EqPreset::Custom => match self {
                Self::TraditionalChinese => "自訂",
                Self::SimplifiedChinese => "自定义",
                Self::English => "Custom",
                Self::French => "Personnalisé",
                Self::German => "Benutzerdefiniert",
                Self::Italian => "Personalizzato",
                Self::Japanese => "カスタム",
                Self::Korean => "사용자 지정",
                Self::PortugueseBrazil => "Personalizado",
                Self::PortuguesePortugal => "Personalizado",
                Self::Indonesian => "Kustom",
                Self::Vietnamese => "Tùy chỉnh",
                Self::Russian => "Пользовательский",
                Self::Spanish => "Personalizado",
            },
            EqPreset::BassBoost => match self {
                Self::TraditionalChinese => "重低音",
                Self::SimplifiedChinese => "重低音",
                Self::English => "Bass Boost",
                Self::French => "Amplification des basses",
                Self::German => "Bass-Verstärkung",
                Self::Italian => "Bassi potenziati",
                Self::Japanese => "低音ブースト",
                Self::Korean => "저음 강화",
                Self::PortugueseBrazil => "Reforço de graves",
                Self::PortuguesePortugal => "Reforço de graves",
                Self::Indonesian => "Penguat Bass",
                Self::Vietnamese => "Tăng cường âm trầm (Bass)",
                Self::Russian => "Усиление басов",
                Self::Spanish => "Refuerzo de graves",
            },
            EqPreset::TrebleBoost => match self {
                Self::TraditionalChinese => "高音增強",
                Self::SimplifiedChinese => "高音增强",
                Self::English => "Treble Boost",
                Self::French => "Amplification des aigus",
                Self::German => "Höhen-Verstärkung",
                Self::Italian => "Alti potenziati",
                Self::Japanese => "高音ブースト",
                Self::Korean => "고음 강화",
                Self::PortugueseBrazil => "Reforço de agudos",
                Self::PortuguesePortugal => "Reforço de agudos",
                Self::Indonesian => "Penguat Treble",
                Self::Vietnamese => "Tăng cường âm bổng (Treble)",
                Self::Russian => "Усиление высоких",
                Self::Spanish => "Refuerzo de agudos",
            },
            EqPreset::Vocal => match self {
                Self::TraditionalChinese => "人聲清晰",
                Self::SimplifiedChinese => "人声清晰",
                Self::English => "Vocal",
                Self::French => "Voix",
                Self::German => "Gesang",
                Self::Italian => "Voce",
                Self::Japanese => "ボーカル",
                Self::Korean => "보컬",
                Self::PortugueseBrazil => "Vocal",
                Self::PortuguesePortugal => "Vocal",
                Self::Indonesian => "Vokal",
                Self::Vietnamese => "Giọng hát (Vocal)",
                Self::Russian => "Вокал",
                Self::Spanish => "Voz",
            },
            EqPreset::Pop => match self {
                Self::TraditionalChinese => "流行",
                Self::SimplifiedChinese => "流行",
                Self::English => "Pop",
                Self::French => "Pop",
                Self::German => "Pop",
                Self::Italian => "Pop",
                Self::Japanese => "ポップ",
                Self::Korean => "팝",
                Self::PortugueseBrazil => "Pop",
                Self::PortuguesePortugal => "Pop",
                Self::Indonesian => "Pop",
                Self::Vietnamese => "Pop",
                Self::Russian => "Поп",
                Self::Spanish => "Pop",
            },
            EqPreset::Rock => match self {
                Self::TraditionalChinese => "搖滾",
                Self::SimplifiedChinese => "摇滚",
                Self::English => "Rock",
                Self::French => "Rock",
                Self::German => "Rock",
                Self::Italian => "Rock",
                Self::Japanese => "ロック",
                Self::Korean => "록",
                Self::PortugueseBrazil => "Rock",
                Self::PortuguesePortugal => "Rock",
                Self::Indonesian => "Rock",
                Self::Vietnamese => "Rock",
                Self::Russian => "Рок",
                Self::Spanish => "Rock",
            },
            EqPreset::Electronic => match self {
                Self::TraditionalChinese => "電子樂",
                Self::SimplifiedChinese => "电子乐",
                Self::English => "Electronic",
                Self::French => "Électro",
                Self::German => "Elektro",
                Self::Italian => "Elettronica",
                Self::Japanese => "エレクトロ",
                Self::Korean => "일렉트로닉",
                Self::PortugueseBrazil => "Eletrônica",
                Self::PortuguesePortugal => "Eletrónica",
                Self::Indonesian => "Elektronik",
                Self::Vietnamese => "Điện tử (Electronic)",
                Self::Russian => "Электронная",
                Self::Spanish => "Electrónica",
            },
            EqPreset::Classical => match self {
                Self::TraditionalChinese => "古典",
                Self::SimplifiedChinese => "古典",
                Self::English => "Classical",
                Self::French => "Classique",
                Self::German => "Klassik",
                Self::Italian => "Classica",
                Self::Japanese => "クラシック",
                Self::Korean => "클래식",
                Self::PortugueseBrazil => "Clássica",
                Self::PortuguesePortugal => "Clássica",
                Self::Indonesian => "Klasik",
                Self::Vietnamese => "Cổ điển (Classical)",
                Self::Russian => "Классика",
                Self::Spanish => "Clásica",
            },
        }
    }

    pub fn tray_notification_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod Bridge - 音響連線變更",
            Self::SimplifiedChinese => "Win2Pod Bridge - 音响连接变更",
            Self::English => "Win2Pod Bridge - Audio Status Change",
            Self::French => "Win2Pod Bridge - Statut de connexion",
            Self::German => "Win2Pod Bridge - Verbindungsänderung",
            Self::Italian => "Win2Pod Bridge - Modifica stato audio",
            Self::Japanese => "Win2Pod Bridge - 接続状態の変更",
            Self::Korean => "Win2Pod Bridge - 오디오 연결 상태 변경",
            Self::PortugueseBrazil => "Win2Pod Bridge - Mudança de status de áudio",
            Self::PortuguesePortugal => "Win2Pod Bridge - Alteração de estado de áudio",
            Self::Indonesian => "Win2Pod Bridge - Perubahan Status Audio",
            Self::Vietnamese => "Win2Pod Bridge - Thay đổi trạng thái âm thanh",
            Self::Russian => "Win2Pod Bridge - Изменение состояния звука",
            Self::Spanish => "Win2Pod Bridge - Cambio de estado de audio",
        }
    }
    pub fn sound_enhancer_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "高頻諧波修復 (HE)",
            Self::SimplifiedChinese => "高频谐波修复 (HE)",
            Self::English => "Harmonic Exciter (HE)",
            Self::French => "Excitateur HE",
            Self::German => "Harmonic Exciter (HE)",
            Self::Italian => "Exciter armonico (HE)",
            Self::Japanese => "高調波エキサイター (HE)",
            Self::Korean => "하모닉 익사이터 (HE)",
            Self::PortugueseBrazil => "Exciter harmônico (HE)",
            Self::PortuguesePortugal => "Excitador Harmónico (HE)",
            Self::Indonesian => "Exciter Harmonik (HE)",
            Self::Vietnamese => "Kích thích họa âm (HE)",
            Self::Russian => "Возбудитель HE",
            Self::Spanish => "Exciter armónico (HE)",
        }
    }

    pub fn sound_enhancer_mode_name(&self, mode: airplay_audio::SoundEnhancerMode) -> &'static str {
        use airplay_audio::SoundEnhancerMode;
        match mode {
            SoundEnhancerMode::Off => match self {
                Self::TraditionalChinese => "關閉",
                Self::SimplifiedChinese => "关闭",
                Self::English => "Off",
                Self::French => "Off",
                Self::German => "Aus",
                Self::Italian => "Off",
                Self::Japanese => "オフ",
                Self::Korean => "끄기",
                Self::PortugueseBrazil => "Off",
                Self::PortuguesePortugal => "Desligado",
                Self::Indonesian => "Mati",
                Self::Vietnamese => "Tắt",
                Self::Russian => "Выкл",
                Self::Spanish => "Off",
            },
            SoundEnhancerMode::Natural => match self {
                Self::TraditionalChinese => "自然還原",
                Self::SimplifiedChinese => "自然还原",
                Self::English => "Natural",
                Self::French => "Naturel",
                Self::German => "Natürlich",
                Self::Italian => "Naturale",
                Self::Japanese => "ナチュラル",
                Self::Korean => "자연스러운",
                Self::PortugueseBrazil => "Natural",
                Self::PortuguesePortugal => "Natural",
                Self::Indonesian => "Alami",
                Self::Vietnamese => "Tự nhiên",
                Self::Russian => "Естественный",
                Self::Spanish => "Natural",
            },
            SoundEnhancerMode::Dynamic => match self {
                Self::TraditionalChinese => "動態增強",
                Self::SimplifiedChinese => "动态增强",
                Self::English => "Dynamic",
                Self::French => "Dynamique",
                Self::German => "Dynamisch",
                Self::Italian => "Dinamico",
                Self::Japanese => "ダイナミック",
                Self::Korean => "다이내믹",
                Self::PortugueseBrazil => "Dinâmico",
                Self::PortuguesePortugal => "Dinâmico",
                Self::Indonesian => "Dinamis",
                Self::Vietnamese => "Năng động",
                Self::Russian => "Динамичный",
                Self::Spanish => "Dinámico",
            },
            SoundEnhancerMode::Brilliant => match self {
                Self::TraditionalChinese => "明亮通透",
                Self::SimplifiedChinese => "明亮通透",
                Self::English => "Brilliant",
                Self::French => "Brillant",
                Self::German => "Brillant",
                Self::Italian => "Brillante",
                Self::Japanese => "ブリリアント",
                Self::Korean => "브릴리언트",
                Self::PortugueseBrazil => "Brilhante",
                Self::PortuguesePortugal => "Brilhante",
                Self::Indonesian => "Cemerlang",
                Self::Vietnamese => "Rực rỡ",
                Self::Russian => "Яркий",
                Self::Spanish => "Brillante",
            },
        }
    }

    pub fn sound_enhancer_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "運用諧波激勵器 (Harmonic Exciter, HE) 與參數等化技術，重建有損壓縮（如 Spotify 160k）所流失的超高頻泛音與空氣感。",
            Self::SimplifiedChinese => "运用谐波激励器 (Harmonic Exciter, HE) 与参数均衡技术，重建有损压缩（如 Spotify 160k）所流失的超高频泛音与空气感。",
            Self::English => "Reconstructs high-frequency harmonics and airiness lost in lossy compression (such as Spotify 160kbps) using Harmonic Exciter (HE).",
            Self::French => "Reconstruit les harmoniques aiguës et l'aération perdues dans la compression avec perte via l'excitateur harmonique (HE).",
            Self::German => "Stellt hochfrequente Obertöne und Räumlichkeit wieder her, die bei verlustbehafteter Kompression verloren gingen (Harmonic Exciter, HE).",
            Self::Italian => "Ricostruisce le armoniche ad alta frequenza perse nella compressione con perdita (Harmonic Exciter, HE).",
            Self::Japanese => "高調波エキサイター (HE) により、不可逆圧縮で失われた超高音域の倍音と空気感を再構築します。",
            Self::Korean => "하모닉 익사이터(HE)를 통해 손실 압축으로 사라진 초고역 배음과 공기감을 복원합니다.",
            Self::PortugueseBrazil => "Reconstrói harmônicos de alta frequência perdidos na compressão com perdas (Harmonic Exciter, HE).",
            Self::PortuguesePortugal => "Reconstrói harmónicos de alta frequência e clareza perdidos na compressão com perdas (como Spotify 160kbps) através do Excitador Harmónico (HE).",
            Self::Indonesian => "Merekonstruksi harmonik frekuensi tinggi dan kejernihan yang hilang dalam kompresi lossy (seperti Spotify 160kbps) menggunakan Harmonic Exciter (HE).",
            Self::Vietnamese => "Tái tạo họa âm tần số cao và độ thoáng đãng bị mất trong nén suy hao (như Spotify 160kbps) bằng Harmonic Exciter (HE).",
            Self::Russian => "Восстанавливает высокочастотные гармоники, потерянные при сжатии с потерями (Harmonic Exciter, HE).",
            Self::Spanish => "Reconstruye los armónicos de alta frecuencia perdidos en la compresión con pérdidas (Harmonic Exciter, HE).",
        }
    }

    pub fn sound_enhancer_preset_tooltip(&self, mode: airplay_audio::SoundEnhancerMode) -> &'static str {
        use airplay_audio::SoundEnhancerMode;
        match mode {
            SoundEnhancerMode::Off => match self {
                Self::TraditionalChinese => "【關閉】完全旁路 (Bypass)，保持純淨原始音訊，適合無損高解析音源 (ALAC / FLAC / Hi-Res)。",
                Self::SimplifiedChinese => "【关闭】完全旁路 (Bypass)，保持纯净原始音频，适合无损高解析音源 (ALAC / FLAC / Hi-Res)。",
                Self::English => "[Off] Completely bypassed. Preserves untouched original audio, ideal for lossless Hi-Res sources (ALAC / FLAC).",
                Self::French => "[Désactivé] Totalement contourné. Conserve l'audio original intact, idéal pour les sources sans perte Hi-Res.",
                Self::German => "[Aus] Komplett umgangen. Behält das unberührte Original-Audio bei, ideal für verlustfreie Hi-Res-Quellen.",
                Self::Italian => "[Disattivato] Completamente bypassato. Mantiene l'audio originale intatto, ideale per sorgenti lossless Hi-Res.",
                Self::Japanese => "【オフ】バイパス。オリジナルの音声をそのまま出力します。ハイレゾ無損音源 (ALAC / FLAC) に最適。",
                Self::Korean => "【끄기】바이패스. 원본 오디오를 그대로 유지하며 무손실 고해상도 음원 (ALAC / FLAC)에 적합합니다.",
                Self::PortugueseBrazil => "[Desligado] Totalmente ignorado. Mantém o áudio original intocado, ideal para fontes Hi-Res sem perdas.",
                Self::PortuguesePortugal => "[Desligado] Totalmente desativado. Mantém o áudio original intocado, ideal para fontes Hi-Res sem perdas (ALAC / FLAC).",
                Self::Indonesian => "[Mati] Sepenuhnya dilewati. Mempertahankan audio asli yang tak tersentuh, ideal untuk sumber Hi-Res tanpa penurunan mutu (ALAC / FLAC).",
                Self::Vietnamese => "[Tắt] Bỏ qua hoàn toàn. Giữ nguyên âm thanh gốc không đổi, lý tưởng cho nguồn Hi-Res không nén (ALAC / FLAC).",
                Self::Russian => "[Выкл] Полный байпас. Сохраняет исходный звук без изменений, идеально для lossless Hi-Res (ALAC / FLAC).",
                Self::Spanish => "[Desactivado] Totalmente bypass. Mantiene el audio original intacto, ideal para fuentes Hi-Res sin pérdidas.",
            },
            SoundEnhancerMode::Natural => match self {
                Self::TraditionalChinese => "【自然還原】輕度修復（8kHz+ 高架濾波與柔和諧波）。適合日常流行樂、Podcast、廣播與人聲，細膩自然耐聽。",
                Self::SimplifiedChinese => "【自然还原】轻度修复（8kHz+ 高架滤波与柔和谐波）。适合日常流行乐、播客 Podcast、广播与人声，细腻自然耐听。",
                Self::English => "[Natural] Gentle restoration (8kHz+ shelf & soft harmonics). Ideal for pop music, podcasts, radio, and vocals.",
                Self::French => "[Naturel] Restauration douce (filtrage 8kHz+ et harmoniques subtiles). Idéal pour la pop, les podcasts et les voix.",
                Self::German => "[Natürlich] Sanfte Wiederherstellung (8kHz+ Shelf & dezente Obertöne). Ideal für Pop, Podcasts und Gesang.",
                Self::Italian => "[Naturale] Ripristino delicato (filtro a mensola 8kHz+ e armoniche morbide). Ideale per pop, podcast e voci.",
                Self::Japanese => "【ナチュラル】穏やかな復元（8kHz+ シェルフと繊細な倍音）。ポップス、ポッドキャスト、ラジオ、ボーカルに最適。",
                Self::Korean => "【자연스러운】부드러운 복원 (8kHz+ 쉘프 및 은은한 배음). 팝 음악, 팟캐스트, 라디오 및 보컬에 적합합니다.",
                Self::PortugueseBrazil => "[Natural] Restauração suave (filtro shelf 8kHz+ e harmônicos suaves). Ideal para música pop, podcasts e vocais.",
                Self::PortuguesePortugal => "[Natural] Restauração suave (filtro shelf 8kHz+ e harmónicos suaves). Ideal para música pop, podcasts, rádio e vocais.",
                Self::Indonesian => "[Alami] Pemulihan lembut (rak 8kHz+ & harmonik lembut). Ideal untuk musik pop, podcast, radio, dan vokal.",
                Self::Vietnamese => "[Tự nhiên] Khôi phục nhẹ nhàng (bộ lọc tần số 8kHz+ & họa âm êm dịu). Thích hợp cho nhạc pop, podcast, radio và giọng hát.",
                Self::Russian => "[Естественный] Мягкое восстановление (полка 8кГц+ и тонкие гармоники). Идеально для поп-музыки, подкастов и вокала.",
                Self::Spanish => "[Natural] Restauración suave (filtro shelf 8kHz+ y armónicos suaves). Ideal para música pop, podcasts y voces.",
            },
            SoundEnhancerMode::Dynamic => match self {
                Self::TraditionalChinese => "【動態增強】中度修復（7kHz+ 高架濾波與動態飽和）。專為 Spotify Free (160k)、YouTube 等有損串流打造，顯著重現空氣感與穿透力。",
                Self::SimplifiedChinese => "【动态增强】中度修复（7kHz+ 高架滤波与动态饱和）。专为 Spotify Free (160k)、YouTube 等有损串流打造，显著重现空气感与穿透力。",
                Self::English => "[Dynamic] Moderate restoration (7kHz+ shelf & dynamic saturation). Tailored for Spotify Free 160k and YouTube, reviving airiness and crispness.",
                Self::French => "[Dynamique] Restauration modérée (7kHz+ et saturation dynamique). Conçu pour Spotify Free et YouTube, redonnant aération et présence.",
                Self::German => "[Dynamisch] Moderate Wiederherstellung (7kHz+ Shelf & dynamische Sättigung). Perfekt für Spotify Free (160k) und YouTube.",
                Self::Italian => "[Dinamico] Ripristino moderato (7kHz+ e saturazione dinamica). Creato per Spotify Free e YouTube, ridona aria e brillantezza.",
                Self::Japanese => "【ダイナミック】中程度の復元（7kHz+ シェルフと動的飽和）。Spotify Free (160k) や YouTube などの圧縮音源に空気感と透明感を蘇らせます。",
                Self::Korean => "【다이내믹】중간 강도 복원 (7kHz+ 쉘프 및 동적 배음). Spotify Free(160k), YouTube 등의 손실 스트리밍에 공기감과 선명함을 불어넣습니다.",
                Self::PortugueseBrazil => "[Dinâmico] Restauração moderada (shelf 7kHz+ e saturação dinâmica). Sob medida para Spotify Free e YouTube, revivendo o brilho.",
                Self::PortuguesePortugal => "[Dinâmico] Restauração moderada (shelf 7kHz+ e saturação dinâmica). Especialmente concebido para Spotify Free 160k e YouTube, revivendo o brilho e nitidez.",
                Self::Indonesian => "[Dinamis] Pemulihan sedang (rak 7kHz+ & saturasi dinamis). Disesuaikan untuk Spotify Gratis 160k dan YouTube, menghidupkan kembali kejernihan dan kerenyahan.",
                Self::Vietnamese => "[Năng động] Khôi phục vừa phải (bộ lọc 7kHz+ & bão hòa động). Tối ưu hóa cho Spotify Free 160k và YouTube, làm sống lại độ sáng rõ và chi tiết.",
                Self::Russian => "[Динамичный] Умеренное восстановление (полка 7кГц+ и динамические гармоники). Специально для Spotify Free (160k) и YouTube.",
                Self::Spanish => "[Dinámico] Restauración moderada (shelf 7kHz+ y saturación dinámica). Creado para Spotify Free y YouTube, reviviendo el brillo.",
            },
            SoundEnhancerMode::Brilliant => match self {
                Self::TraditionalChinese => "【明亮通透】深度修復（6kHz+ 高架濾波與豐富高次諧波）。適合古典樂、木吉他、爵士樂與器樂演奏，高頻金屬光澤與空間泛音極其豐富。",
                Self::SimplifiedChinese => "【明亮通透】深度修复（6kHz+ 高架滤波与丰富高次谐波）。适合古典乐、木吉他、爵士乐与器乐演奏，高频金属光泽与空间泛音极其丰富。",
                Self::English => "[Brilliant] Deep restoration (6kHz+ shelf & rich upper harmonics). Ideal for classical, acoustic guitar, jazz, and percussion with shimmering highs.",
                Self::French => "[Brillant] Restauration profonde (6kHz+ et riches harmoniques). Idéal pour le classique, la guitare acoustique, le jazz et les percussions.",
                Self::German => "[Brillant] Tiefe Wiederherstellung (6kHz+ Shelf & reichhaltige Obertöne). Ideal für Klassik, Akustikgitarre, Jazz und Becken.",
                Self::Italian => "[Brillante] Ripristino profondo (6kHz+ e ricche armoniche superiori). Ideale per classica, chitarra acustica, jazz e percussioni.",
                Self::Japanese => "【ブリリアント】高度な復元（6kHz+ シェルフと豊かな高次倍音）。クラシック、アコギ、ジャズ、打楽器のきらびやかな高音域を引き出します。",
                Self::Korean => "【브릴리언트】심층 복원 (6kHz+ 쉘프 및 풍부한 고차 배음). 클래식, 어쿠스틱 기타, 재즈 및 타악기의 화려한 고역 디테일에 이상적입니다.",
                Self::PortugueseBrazil => "[Brilhante] Restauração profunda (shelf 6kHz+ e harmônicos ricos). Ideal para clássica, violão acústico, jazz e pratos.",
                Self::PortuguesePortugal => "[Brilhante] Restauração profunda (shelf 6kHz+ e harmónicos superiores ricos). Ideal para música clássica, guitarra acústica, jazz e percussão com agudos cintilantes.",
                Self::Indonesian => "[Cemerlang] Pemulihan mendalam (rak 6kHz+ & harmonik atas yang kaya). Ideal untuk klasik, gitar akustik, jazz, dan perkusi dengan nada tinggi berkilau.",
                Self::Vietnamese => "[Rực rỡ] Khôi phục sâu sắc (bộ lọc 6kHz+ & họa âm cao phong phú). Tuyệt vời cho nhạc cổ điển, guitar mộc, jazz và bộ gõ với âm cao lung linh.",
                Self::Russian => "[Яркий] Глубокое восстановление (полка 6кГц+ и богатые гармоники). Идеально для классики, акустической гитары, джаза и тарелок.",
                Self::Spanish => "[Brillante] Restauración profunda (shelf 6kHz+ y ricos armónicos). Ideal para clásica, guitarra acústica, jazz y percusión.",
            },
        }
    }

    pub fn warmth_enhancer_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "溫暖音色增強 (WE)",
            Self::SimplifiedChinese => "温暖音色增强 (WE)",
            Self::English => "Warmth Enhancer (WE)",
            Self::French => "Chaleur analogique (WE)",
            Self::German => "Analoge Wärme (WE)",
            Self::Italian => "Calore analogico (WE)",
            Self::Japanese => "ウォームス・エンハンサー (WE)",
            Self::Korean => "웜스 인핸서 (WE)",
            Self::PortugueseBrazil => "Calor analógico (WE)",
            Self::PortuguesePortugal => "Acentuador de Calor (WE)",
            Self::Indonesian => "Peningkat Kehangatan (WE)",
            Self::Vietnamese => "Tăng cường độ ấm (WE)",
            Self::Russian => "Аналоговое тепло (WE)",
            Self::Spanish => "Calidez analógica (WE)",
        }
    }

    pub fn warmth_enhancer_mode_name(&self, mode: airplay_audio::WarmthEnhancerMode) -> &'static str {
        use airplay_audio::WarmthEnhancerMode;
        match mode {
            WarmthEnhancerMode::Off => match self {
                Self::TraditionalChinese => "關閉",
                Self::SimplifiedChinese => "关闭",
                Self::English => "Off",
                Self::French => "Off",
                Self::German => "Aus",
                Self::Italian => "Off",
                Self::Japanese => "オフ",
                Self::Korean => "끄기",
                Self::PortugueseBrazil => "Off",
                Self::PortuguesePortugal => "Desligado",
                Self::Indonesian => "Mati",
                Self::Vietnamese => "Tắt",
                Self::Russian => "Выкл",
                Self::Spanish => "Off",
            },
            WarmthEnhancerMode::Subtle => match self {
                Self::TraditionalChinese => "輕度溫暖",
                Self::SimplifiedChinese => "轻度温暖",
                Self::English => "Subtle",
                Self::French => "Léger",
                Self::German => "Dezent",
                Self::Italian => "Sottile",
                Self::Japanese => "サトル",
                Self::Korean => "미세 온기",
                Self::PortugueseBrazil => "Sutil",
                Self::PortuguesePortugal => "Subtil",
                Self::Indonesian => "Halus",
                Self::Vietnamese => "Tinh tế",
                Self::Russian => "Мягкий",
                Self::Spanish => "Sutil",
            },
            WarmthEnhancerMode::Warm => match self {
                Self::TraditionalChinese => "經典黑膠",
                Self::SimplifiedChinese => "经典黑胶",
                Self::English => "Warm",
                Self::French => "Chaud",
                Self::German => "Warm",
                Self::Italian => "Caldo",
                Self::Japanese => "ウォーム",
                Self::Korean => "클래식 바이닐",
                Self::PortugueseBrazil => "Quente",
                Self::PortuguesePortugal => "Quente",
                Self::Indonesian => "Hangat",
                Self::Vietnamese => "Ấm áp",
                Self::Russian => "Тёплый",
                Self::Spanish => "Cálido",
            },
            WarmthEnhancerMode::Vintage => match self {
                Self::TraditionalChinese => "復古醇厚",
                Self::SimplifiedChinese => "复古醇厚",
                Self::English => "Vintage",
                Self::French => "Rétro",
                Self::German => "Vintage",
                Self::Italian => "Vintage",
                Self::Japanese => "ビンテージ",
                Self::Korean => "빈티지",
                Self::PortugueseBrazil => "Vintage",
                Self::PortuguesePortugal => "Vintage",
                Self::Indonesian => "Klasik (Vintage)",
                Self::Vietnamese => "Cổ kính (Vintage)",
                Self::Russian => "Винтаж",
                Self::Spanish => "Vintage",
            },
        }
    }

    pub fn warmth_enhancer_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "模擬黑膠唱臂轉盤低頻物理共鳴（50~100Hz 低架增益 1.5~2dB）與真空管/磁帶飽和度（Drive 2%~5%），消除數位生硬冷澀感，賦予溫潤醇厚底蘊。",
            Self::SimplifiedChinese => "模拟黑胶唱臂转盘低频物理共鸣（50~100Hz 低架增益 1.5~2dB）与电子管/磁带饱和度（Drive 2%~5%），消除数码生硬冷涩感，赋予温润醇厚底蕴。",
            Self::English => "Simulates analog turntable tonearm resonance (50-100Hz low-shelf +1.5-2dB) and tube/tape saturation (2%-5% drive) to eliminate digital harshness and add vintage vinyl warmth.",
            Self::French => "Simule la résonance d'une platine vinyle (50-100Hz +1.5-2dB) et la saturation à lampes/bande (2%-5%) pour éliminer la froideur numérique.",
            Self::German => "Simuliert Tonarm-Resonanz (50-100Hz +1,5-2dB) und Röhren-/Bandsättigung (2%-5%), um digitale Kälte zu beseitigen und Vinyl-Wärme zu erzeugen.",
            Self::Italian => "Simula la risonanza del giradischi (50-100Hz +1.5-2dB) e la saturazione valvolare/nastro (2%-5%) per eliminare la freddezza digitale.",
            Self::Japanese => "アナログレコードのトーンアーム共振（50〜100Hz ローシェルフ +1.5〜2dB）と真空管/テープ飽和（Drive 2%〜5%）を再現し、デジタルの硬さを和らげます。",
            Self::Korean => "LP 톤암 공진(50~100Hz 로우 쉘프 +1.5~2dB)과 진공관/테이프 채도(2%~5% 드라이브)를 시뮬레이션하여 디지털의 차가움을 없애고 아날로그 온기를 더합니다.",
            Self::PortugueseBrazil => "Simula a ressonância de toca-discos (50-100Hz +1.5-2dB) e a saturação valvulada/fita (2%-5%) para suavizar a frieza digital.",
            Self::PortuguesePortugal => "Simula a ressonância do braço de gira-discos (50-100Hz low-shelf +1.5-2dB) e a saturação de válvulas/fita (2%-5% drive) para eliminar a frieza digital e adicionar calor de vinil vintage.",
            Self::Indonesian => "Mensimulasikan resonansi lengan pemutar piringan hitam (low-shelf 50-100Hz +1,5-2dB) dan saturasi tabung/pita (drive 2%-5%) untuk menghilangkan kekakuan digital dan menambahkan kehangatan vinil klasik.",
            Self::Vietnamese => "Mô phỏng cộng hưởng cần đĩa than analog (low-shelf 50-100Hz +1.5-2dB) và độ bão hòa bóng đèn/băng từ (drive 2%-5%) để loại bỏ cảm giác khô cứng kỹ thuật số và bổ sung độ ấm đĩa than cổ điển.",
            Self::Russian => "Имитирует резонанс тонарма винила (50-100 Гц +1.5-2 дБ) и ламповую/ленточную сатурацию (2%-5%), устраняя цифровую сухость.",
            Self::Spanish => "Simula la resonancia de tocadiscos (50-100Hz +1.5-2dB) y saturación a válvulas/cinta (2%-5%) para eliminar la frialdad digital.",
        }
    }

    pub fn warmth_enhancer_preset_tooltip(&self, mode: airplay_audio::WarmthEnhancerMode) -> &'static str {
        use airplay_audio::WarmthEnhancerMode;
        match mode {
            WarmthEnhancerMode::Off => match self {
                Self::TraditionalChinese => "【關閉】完全旁路 (Bypass)，保持純淨原始音訊，不施加任何低頻染色或類比飽和度。",
                Self::SimplifiedChinese => "【关闭】完全旁路 (Bypass)，保持纯净原始音频，不施加任何低频染色或模拟饱和度。",
                Self::English => "[Off] Completely bypassed. Preserves untouched original audio without low-frequency coloring or saturation.",
                Self::French => "[Désactivé] Totalement contourné. Conserve l'audio original intact sans coloration ni saturation.",
                Self::German => "[Aus] Komplett umgangen. Behält das unberührte Original-Audio ohne Färbung oder Sättigung bei.",
                Self::Italian => "[Disattivato] Completamente bypassato. Mantiene l'audio originale intatto senza colorazioni o saturazione.",
                Self::Japanese => "【オフ】バイパス。低域の着色やアナログ飽和を一切加えず、元の音声をそのまま出力します。",
                Self::Korean => "【끄기】바이패스. 저역 착색이나 아날로그 채도 없이 원본 오디오를 그대로 유지합니다.",
                Self::PortugueseBrazil => "[Desligado] Totalmente ignorado. Mantém o áudio original intocado sem coloração ou saturação.",
                Self::PortuguesePortugal => "[Desligado] Totalmente desativado. Mantém o áudio original intocado sem coloração de baixas frequências ou saturação.",
                Self::Indonesian => "[Mati] Sepenuhnya dilewati. Mempertahankan audio asli tanpa pewarnaan atau saturasi frekuensi rendah.",
                Self::Vietnamese => "[Tắt] Bỏ qua hoàn toàn. Giữ nguyên âm thanh gốc không đổi mà không có màu âm trầm hoặc độ bão hòa.",
                Self::Russian => "[Выкл] Полный байпас. Сохраняет исходный звук без окрашивания и сатурации.",
                Self::Spanish => "[Desactivado] Totalmente bypass. Mantiene el audio original intacto sin coloración ni saturación.",
            },
            WarmthEnhancerMode::Subtle => match self {
                Self::TraditionalChinese => "【輕度溫暖】微量飽和 (Drive 2.5%) 與 60Hz (+1.5dB) 低架增益。自然化解數位生硬感，適合現代流行樂、流行爵士與人聲。",
                Self::SimplifiedChinese => "【轻度温暖】微量饱和 (Drive 2.5%) 与 60Hz (+1.5dB) 低架增益。自然化解数码生硬感，适合现代流行乐、流行爵士与人声。",
                Self::English => "[Subtle] Subtle saturation (2.5% drive) and 60Hz (+1.5dB) low-shelf. Softens digital harshness, ideal for modern pop, jazz, and vocals.",
                Self::French => "[Léger] Saturation subtile (drive 2,5%) et étagère basse 60Hz (+1,5dB). Adoucit la dureté numérique, parfait pour la pop et le jazz.",
                Self::German => "[Dezent] Dezente Sättigung (2,5% Drive) und 60Hz (+1,5dB) Low-Shelf. Mildert digitale Schärfe, ideal für Pop, Jazz und Vocals.",
                Self::Italian => "[Sottile] Saturazione sottile (drive 2,5%) e mensola bassa 60Hz (+1,5dB). Attenua l'asprezza digitale, ideale per pop, jazz e voci.",
                Self::Japanese => "【サトル】微量の飽和（Drive 2.5%）と 60Hz (+1.5dB) ローシェルフ。デジタルの硬さを自然にほぐし、ポップスやボーカルに最適。",
                Self::Korean => "【미세 온기】미세 채도 (2.5% 드라이브) 및 60Hz (+1.5dB) 로우 쉘프. 디지털의 날카로움을 부드럽게 완화하여 팝과 보컬에 적합합니다.",
                Self::PortugueseBrazil => "[Sutil] Saturação sutil (drive 2,5%) e shelf grave 60Hz (+1,5dB). Suaviza a aspereza digital, ideal para pop e vocais.",
                Self::PortuguesePortugal => "[Subtil] Saturação subtil (drive 2,5%) e shelf grave 60Hz (+1,5dB). Suaviza a aspereza digital, ideal para pop moderno, jazz e vocais.",
                Self::Indonesian => "[Halus] Saturasi halus (drive 2,5%) dan low-shelf 60Hz (+1,5dB). Melembutkan kekasaran digital, ideal untuk pop modern, jazz, dan vokal.",
                Self::Vietnamese => "[Tinh tế] Độ bão hòa nhẹ (drive 2.5%) và low-shelf 60Hz (+1.5dB). Làm dịu sự chói gắt kỹ thuật số, phù hợp cho nhạc pop hiện đại, jazz và giọng hát.",
                Self::Russian => "[Мягкий] Тонкая сатурация (драйв 2,5%) и полка 60 Гц (+1,5 дБ). Смягчает цифровую резкость, отлично для поп-музыки и вокала.",
                Self::Spanish => "[Sutil] Saturación sutil (drive 2,5%) y shelf grave 60Hz (+1,5dB). Suaviza la aspereza digital, ideal para pop y voces.",
            },
            WarmthEnhancerMode::Warm => match self {
                Self::TraditionalChinese => "【經典黑膠】標準飽和 (Drive 3.8%) 與 75Hz (+1.8dB) 低架增益。再現黑膠唱片轉盤共鳴與磁帶溫潤底蘊，聲音寬鬆飽滿，適合搖滾、流行與串流。",
                Self::SimplifiedChinese => "【经典黑胶】标准饱和 (Drive 3.8%) 与 75Hz (+1.8dB) 低架增益。再现黑胶唱片转盘共鸣与磁带温润底蕴，声音宽松饱满，适合摇滚、流行与串流。",
                Self::English => "[Warm] Classic vinyl warmth (3.8% drive) and 75Hz (+1.8dB) low-shelf. Emulates turntable resonance and tape warmth, ideal for rock, pop, and streaming.",
                Self::French => "[Chaleureux] Chaleur vinyle classique (drive 3,8%) et 75Hz (+1,8dB). Émule la résonance du vinyle et la rondeur de la bande magnétique.",
                Self::German => "[Warm] Klassische Vinyl-Wärme (3,8% Drive) und 75Hz (+1,8dB) Low-Shelf. Emuliert Plattenspieler-Resonanz und Band-Wärme.",
                Self::Italian => "[Caldo] Calore vinilico classico (drive 3,8%) e mensola 75Hz (+1,8dB). Ricrea la risonanza del vinile e il calore del nastro.",
                Self::Japanese => "【ウォーム】定番のアナログ感（Drive 3.8%）と 75Hz (+1.8dB) ローシェルフ。ターンテーブルの豊かな低域共鳴とテープの温かみを再現。",
                Self::Korean => "【클래식 바이닐】표준 아날로그 온기 (3.8% 드라이브) 및 75Hz (+1.8dB) 로우 쉘프. 턴테이블 공진과 테이프의 포근함을 재현합니다.",
                Self::PortugueseBrazil => "[Quente] Calor clássico do vinil (drive 3,8%) e 75Hz (+1,8dB). Emula a ressonância do prato e o calor da fita analógica.",
                Self::PortuguesePortugal => "[Quente] Calor clássico do vinil (drive 3,8%) e 75Hz (+1,8dB) low-shelf. Emula a ressonância do gira-discos e o calor da fita, ideal para rock, pop e streaming.",
                Self::Indonesian => "[Hangat] Kehangatan vinil klasik (drive 3,8%) dan low-shelf 75Hz (+1,8dB). Meniru resonansi piringan hitam dan kehangatan pita kaset, ideal untuk rock, pop, dan streaming.",
                Self::Vietnamese => "[Ấm áp] Độ ấm đĩa than kinh điển (drive 3.8%) và low-shelf 75Hz (+1.8dB). Tái hiện độ cộng hưởng đĩa than và độ ấm băng từ, hoàn hảo cho rock, pop và phát trực tuyến.",
                Self::Russian => "[Тёплый] Классическое виниловое тепло (драйв 3,8%) и полка 75 Гц (+1,8 дБ). Воссоздаёт резонанс вертушки и мягкость плёнки.",
                Self::Spanish => "[Cálido] Calidez de vinilo clásica (drive 3,8%) y shelf 75Hz (+1,8dB). Emula la resonancia del tocadiscos y la calidez de la cinta.",
            },
            WarmthEnhancerMode::Vintage => match self {
                Self::TraditionalChinese => "【復古醇厚】濃郁真空管飽和 (Drive 5.0%) 與 90Hz (+2.0dB) 低架增益。深沉厚實的低頻物理包圍感與類比泛音，適合老歌經典、黑膠發燒友與爵士大樂隊。",
                Self::SimplifiedChinese => "【复古醇厚】浓郁电子管饱和 (Drive 5.0%) 与 90Hz (+2.0dB) 低架增益。深沉厚实的低频物理包围感与模拟泛音，适合老歌经典、黑胶发烧友与爵士大乐队。",
                Self::English => "[Vintage] Rich tube saturation (5.0% drive) and 90Hz (+2.0dB) low-shelf. Deep physical low-end embrace and rich analog warmth, ideal for vintage classics and jazz.",
                Self::French => "[Rétro] Saturation tube riche (drive 5,0%) et 90Hz (+2,0dB). Des basses profondes et enveloppantes, idéal pour les classiques rétro et le jazz.",
                Self::German => "[Vintage] Satte Röhrensättigung (5,0% Drive) und 90Hz (+2,0dB) Low-Shelf. Tiefer, fülliger Bass und analoge Obertöne für Klassiker und Jazz.",
                Self::Italian => "[Vintage] Ricca saturazione valvolare (drive 5,0%) e mensola 90Hz (+2,0dB). Basso profondo e avvolgente con armoniche analogiche per jazz e classici.",
                Self::Japanese => "【ビンテージ】濃厚な真空管飽和（Drive 5.0%）と 90Hz (+2.0dB) ローシェルフ。重厚で包み込むような低音と豊かなアナログの余韻を創出。",
                Self::Korean => "【빈티지】진한 진공관 채도 (5.0% 드라이브) 및 90Hz (+2.0dB) 로우 쉘프. 깊고 풍성한 저역 포만감과 풍부한 아날로그 배음으로 재즈와 명곡에 최적입니다.",
                Self::PortugueseBrazil => "[Vintage] Saturação valvulada encorpada (drive 5,0%) e 90Hz (+2,0dB). Graves profundos e envolventes para clássicos retrô e jazz.",
                Self::PortuguesePortugal => "[Vintage] Saturação rica de válvulas (drive 5,0%) e 90Hz (+2,0dB) low-shelf. Graves profundos e envolventes com rico calor analógico, ideal para clássicos vintage e jazz.",
                Self::Indonesian => "[Klasik] Saturasi tabung kaya (drive 5,0%) dan low-shelf 90Hz (+2,0dB). Dekapan fisik nada rendah yang dalam dan kehangatan analog kaya, ideal untuk musik klasik vintage dan jazz.",
                Self::Vietnamese => "[Cổ kính] Độ bão hòa bóng đèn ấm nồng (drive 5.0%) và low-shelf 90Hz (+2.0dB). Âm trầm dày dặn ôm trọn không gian cùng chất âm analog mộc mạc, lý tưởng cho nhạc cổ điển và jazz.",
                Self::Russian => "[Винтаж] Плотная ламповая сатурация (драйв 5,0%) и полка 90 Гц (+2,0 дБ). Глубокий аналоговый бас и бархатные гармоники для классики и джаза.",
                Self::Spanish => "[Vintage] Rica saturación a válvulas (drive 5,0%) y shelf 90Hz (+2,0dB). Graves profundos y envolventes con armónicos analógicos para jazz y clásicos.",
            },
        }
    }

    pub fn btn_start_ao(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "啟動本機增強",
            Self::SimplifiedChinese => "启动本机增强",
            Self::English => "Start Local Enhancement",
            Self::French => "Activer le traitement local",
            Self::German => "Lokale Optimierung starten",
            Self::Italian => "Avvia elaborazione locale",
            Self::Japanese => "ローカル強化を開始",
            Self::Korean => "로컬 오디오 향상 시작",
            Self::PortugueseBrazil => "Iniciar aprimoramento local",
            Self::PortuguesePortugal => "Iniciar melhoramento local",
            Self::Indonesian => "Mulai Peningkatan Lokal",
            Self::Vietnamese => "Bắt đầu nâng cao âm thanh cục bộ",
            Self::Russian => "Включить локальную обработку",
            Self::Spanish => "Iniciar mejora local",
        }
    }

    pub fn btn_stop_ao(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "停止本機增強",
            Self::SimplifiedChinese => "停止本机增强",
            Self::English => "Stop Local Enhancement",
            Self::French => "Arrêter le traitement local",
            Self::German => "Lokale Optimierung stoppen",
            Self::Italian => "Arresta elaborazione locale",
            Self::Japanese => "ローカル強化を停止",
            Self::Korean => "로컬 오디오 향상 중지",
            Self::PortugueseBrazil => "Parar aprimoramento local",
            Self::PortuguesePortugal => "Parar melhoramento local",
            Self::Indonesian => "Hentikan Peningkatan Lokal",
            Self::Vietnamese => "Dừng nâng cao âm thanh cục bộ",
            Self::Russian => "Остановить локальную обработку",
            Self::Spanish => "Detener mejora local",
        }
    }

    pub fn ao_output_device_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "本機輸出裝置",
            Self::SimplifiedChinese => "本机输出设备",
            Self::English => "Local Output Device",
            Self::French => "Périphérique de sortie",
            Self::German => "Ausgabegerät",
            Self::Italian => "Dispositivo di uscita",
            Self::Japanese => "出力先デバイス",
            Self::Korean => "로컬 출력 장치",
            Self::PortugueseBrazil => "Dispositivo de saída",
            Self::PortuguesePortugal => "Dispositivo de saída local",
            Self::Indonesian => "Perangkat Output Lokal",
            Self::Vietnamese => "Thiết bị đầu ra cục bộ",
            Self::Russian => "Устройство вывода",
            Self::Spanish => "Dispositivo de salida",
        }
    }

    pub fn auto_ao_mode_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 未連線時自動啟用本機音訊增強",
            Self::SimplifiedChinese => "AirPlay 未连接时自动启用本机音频增强",
            Self::English => "Auto-enable local audio enhancement when AirPlay is disconnected",
            Self::French => "Activer l'amélioration locale automatiquement si AirPlay est inactif",
            Self::German => "Lokale Audio-Optimierung automatisch aktivieren wenn AirPlay getrennt",
            Self::Italian => "Attiva ottimizzazione audio locale se AirPlay non è connesso",
            Self::Japanese => "AirPlay 未接続時にローカル音声強化を自動有効化",
            Self::Korean => "AirPlay 미연결 시 로컬 오디오 향상 자동 활성화",
            Self::PortugueseBrazil => "Ativar aprimoramento local automaticamente quando AirPlay estiver ocioso",
            Self::PortuguesePortugal => "Ativar melhoramento de áudio local automaticamente quando o AirPlay estiver desligado",
            Self::Indonesian => "Aktifkan peningkatan audio lokal otomatis saat AirPlay terputus",
            Self::Vietnamese => "Tự động bật nâng cao âm thanh cục bộ khi AirPlay ngắt kết nối",
            Self::Russian => "Автоматически включать локальную обработку, когда AirPlay отключен",
            Self::Spanish => "Activar mejora de audio local automáticamente cuando AirPlay esté desconectado",
        }
    }

    pub fn ao_mode_active_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "● 增強運作中",
            Self::SimplifiedChinese => "● 增强运行中",
            Self::English => "● Enhancing",
            Self::French => "● Actif",
            Self::German => "● Aktiv",
            Self::Italian => "● Attivo",
            Self::Japanese => "● 稼働中",
            Self::Korean => "● 작동 중",
            Self::PortugueseBrazil => "● Ativo",
            Self::PortuguesePortugal => "● A melhorar",
            Self::Indonesian => "● Meningkatkan",
            Self::Vietnamese => "● Đang nâng cao",
            Self::Russian => "● Активно",
            Self::Spanish => "● Activo",
        }
    }

    pub fn airpods_section_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPods 專用設定",
            Self::SimplifiedChinese => "AirPods 专用设置",
            Self::English => "AirPods Settings",
            Self::French => "Paramètres AirPods",
            Self::German => "AirPods-Einstellungen",
            Self::Italian => "Impostazioni AirPods",
            Self::Japanese => "AirPods 設定",
            Self::Korean => "AirPods 설정",
            Self::PortugueseBrazil => "Configurações do AirPods",
            Self::PortuguesePortugal => "Definições dos AirPods",
            Self::Indonesian => "Pengaturan AirPods",
            Self::Vietnamese => "Cài đặt AirPods",
            Self::Russian => "Настройки AirPods",
            Self::Spanish => "Ajustes de AirPods",
        }
    }

    pub fn airpods_auto_routing_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPods 自動音路切換（戴上接管音訊，收盒釋放音路）",
            Self::SimplifiedChinese => "AirPods 自动音路切换（戴上接管音频，收盒释放音路）",
            Self::English => "AirPods Auto Audio Routing (Route on wear, release on case close)",
            Self::French => "Routage audio auto AirPods (Activer au port, libérer à la fermeture)",
            Self::German => "AirPods Auto-Audioweiterleitung (Beim Tragen aktivieren, beim Schließen freigeben)",
            Self::Italian => "Instradamento audio auto AirPods (Attiva all'uso, rilascia alla chiusura)",
            Self::Japanese => "AirPods 自動音声切り替え（装着時に引き継ぎ、ケース収納時に解除）",
            Self::Korean => "AirPods 자동 오디오 전환 (착용 시 연결, 케이스 수납 시 해제)",
            Self::PortugueseBrazil => "Roteamento de áudio auto AirPods (Conectar ao usar, liberar ao fechar estojo)",
            Self::PortuguesePortugal => "Encaminhamento de áudio auto para AirPods (Ligar ao colocar, libertar ao fechar caixa)",
            Self::Indonesian => "Perutean Audio Otomatis AirPods (Rute saat dipakai, lepaskan saat wadah ditutup)",
            Self::Vietnamese => "Tự động định tuyến âm thanh AirPods (Định tuyến khi đeo, giải phóng khi đóng hộp)",
            Self::Russian => "Авто-маршрутизация AirPods (переключение при надевании, сброс при закрытии)",
            Self::Spanish => "Enrutamiento de audio auto AirPods (Conectar al usar, liberar al cerrar estuche)",
        }
    }

    pub fn airpods_auto_routing_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "系統依據 Windows 藍牙播放裝置名稱自動辨識並切換音路。若曾自訂耳機名稱，請確保名稱中包含「AirPods」或「Pods」。",
            Self::SimplifiedChinese => "系统依据 Windows 蓝牙播放设备名称自动识别并切换音路。若曾自定义耳机名称，请确保名称中包含“AirPods”或“Pods”。",
            Self::English => "Audio route switching matches Windows Bluetooth playback device names. If you renamed your earbuds, ensure the name contains \"AirPods\" or \"Pods\".",
            Self::French => "Le routage audio correspond aux noms d'appareils Bluetooth sous Windows. Si vous avez renommé vos écouteurs, assurez-vous que le nom contient « AirPods » ou « Pods ».",
            Self::German => "Die Audio-Umschaltung erfolgt anhand der Windows-Bluetooth-Gerätenamen. Falls Sie Ihre Ohrhörer umbenannt haben, stellen Sie sicher, dass der Name „AirPods“ oder „Pods“ enthält.",
            Self::Italian => "Il reindirizzamento audio si basa sui nomi dei dispositivi Bluetooth di Windows. Se hai rinominato gli auricolari, assicurati che il nome contenga \"AirPods\" o \"Pods\".",
            Self::Japanese => "音声ルーティングは Windows の Bluetooth デバイス名に基づいて一致判定を行います。イヤホンの名前を変更した場合は、名前に「AirPods」または「Pods」が含まれていることを確認してください。",
            Self::Korean => "오디오 전환은 Windows Bluetooth 재생 장치 이름을 기반으로 일치 항목을 찾습니다. 이어폰 이름을 변경한 경우 이름에 'AirPods' 또는 'Pods'가 포함되어 있는지 확인하세요.",
            Self::PortugueseBrazil => "A alternância de rota de áudio depende dos nomes dos dispositivos Bluetooth no Windows. Se você renomeou seus fones, certifique-se de que o nome contenha \"AirPods\" ou \"Pods\".",
            Self::PortuguesePortugal => "A mudança de rota de áudio corresponde aos nomes dos dispositivos Bluetooth no Windows. Se renomeou os seus auriculares, certifique-se de que o nome contém \"AirPods\" ou \"Pods\".",
            Self::Indonesian => "Peralihan rute audio cocok dengan nama perangkat pemutaran Bluetooth Windows. Jika Anda mengganti nama earbud, pastikan nama berisi \"AirPods\" atau \"Pods\".",
            Self::Vietnamese => "Chuyển đổi luồng âm thanh khớp theo tên thiết bị phát Bluetooth của Windows. Nếu bạn đổi tên tai nghe, hãy đảm bảo tên vẫn giữ từ \"AirPods\" hoặc \"Pods\".",
            Self::Russian => "Переключение аудиомаршрута сопоставляется с именами Bluetooth-устройств в Windows. Если вы переименовали наушники, убедитесь, что имя содержит «AirPods» или «Pods».",
            Self::Spanish => "El enrutamiento de audio coincide con los nombres de dispositivos Bluetooth en Windows. Si ha cambiado el nombre de sus auriculares, asegúrese de que contenga \"AirPods\" o \"Pods\".",
        }
    }

    pub fn airpods_naming_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 提示：若曾自訂耳機名稱，請保留「AirPods」或「Pods」關鍵字以確保自動切換",
            Self::SimplifiedChinese => "💡 提示：若曾自定义耳机名称，请保留“AirPods”或“Pods”关键字以确保自动切换",
            Self::English => "💡 Tip: If you renamed your earbuds, keep \"AirPods\" or \"Pods\" in the name for auto-routing",
            Self::French => "💡 Astuce : conservez « AirPods » ou « Pods » dans le nom pour le routage auto",
            Self::German => "💡 Tipp: Behalten Sie „AirPods“ oder „Pods“ im Namen für automatisches Umschalten",
            Self::Italian => "💡 Suggerimento: mantieni \"AirPods\" o \"Pods\" nel nome per l'instradamento automatico",
            Self::Japanese => "💡 ヒント：イヤホン名を変更した場合は、名前に「AirPods」または「Pods」を残してください",
            Self::Korean => "💡 팁: 이어폰 이름을 변경한 경우 자동 전환을 위해 'AirPods' 또는 'Pods'를 유지하세요",
            Self::PortugueseBrazil => "💡 Dica: mantenha \"AirPods\" ou \"Pods\" no nome para o roteamento automático",
            Self::PortuguesePortugal => "💡 Dica: Se renomeou os seus auriculares, mantenha \"AirPods\" ou \"Pods\" no nome para o encaminhamento automático",
            Self::Indonesian => "💡 Tips: Jika Anda mengganti nama earbud, pertahankan \"AirPods\" atau \"Pods\" dalam nama untuk perutean otomatis",
            Self::Vietnamese => "💡 Mẹo: Nếu bạn đã đổi tên tai nghe, hãy giữ \"AirPods\" hoặc \"Pods\" trong tên để định tuyến tự động",
            Self::Russian => "💡 Совет: сохраните «AirPods» или «Pods» в названии для автопереключения",
            Self::Spanish => "💡 Consejo: mantenga \"AirPods\" o \"Pods\" en el nombre para el enrutamiento automático",
        }
    }

    pub fn airpods_smart_playback_control_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPods 智慧播放控制（取下自動暫停，收盒接力播放）",
            Self::SimplifiedChinese => "AirPods 智慧播放控制（取下自动暂停，收盒接力播放）",
            Self::English => "AirPods Smart Playback Control (Pause on removal, handoff on case close)",
            Self::French => "Contrôle intelligent AirPods (Pause au retrait, relais à la fermeture)",
            Self::German => "AirPods Intelligente Wiedergabe (Pause beim Abnehmen, Weitergabe beim Schließen)",
            Self::Italian => "Controllo intelligente AirPods (Pausa alla rimozione, passaggio alla chiusura)",
            Self::Japanese => "AirPods スマート再生制御（取り外し時一時停止・ケース収納時引き継ぎ再生）",
            Self::Korean => "AirPods 스마트 재생 제어 (착용 해제 시 일시정지, 케이스 수납 시 이어듣기)",
            Self::PortugueseBrazil => "Controle inteligente dos AirPods (Pausar ao retirar, continuar ao fechar estojo)",
            Self::PortuguesePortugal => "Controlo inteligente de reprodução dos AirPods (Pausar ao retirar, transferir ao fechar caixa)",
            Self::Indonesian => "Kontrol Pemutaran Pintar AirPods (Jeda saat dilepas, serahkan saat wadah ditutup)",
            Self::Vietnamese => "Điều khiển phát nhạc thông minh AirPods (Tạm dừng khi tháo ra, chuyển tiếp khi đóng hộp)",
            Self::Russian => "Умное воспроизведение AirPods (авто-пауза при снятии, передача при закрытии футляра)",
            Self::Spanish => "Control inteligente AirPods (Pausa al quitar, reanudar al cerrar estuche)",
        }
    }

    pub fn airpods_smart_playback_control_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "聆聽時取下耳機會自動暫停；若在 30 秒內收回充電盒切換至音響、或 60 秒內重新戴上耳機，都會自動為您繼續播放音樂。",
            Self::SimplifiedChinese => "聆听时取下耳机会自动暂停；若在 30 秒内收回充电盒切换至音响、或 60 秒内重新戴上耳机，都会自动为您继续播放音乐。",
            Self::English => "Auto-pauses when removing AirPods. Automatically resumes playback if reinserted within 60s or stored in case within 30s to hand off to speakers.",
            Self::French => "Met en pause automatique au retrait des AirPods. Reprend la lecture si réinséré sous 60 s ou rangé dans le boîtier sous 30 s pour relayer aux enceintes.",
            Self::German => "Pausiert automatisch beim Abnehmen der AirPods. Setzt die Wiedergabe fort, wenn sie innerhalb von 60 s wieder eingesetzt oder innerhalb von 30 s im Case verstaut werden.",
            Self::Italian => "Mette in pausa automatica quando rimuovi gli AirPods. Riprende la riproduzione se reinseriti entro 60 s o riposti nella custodia entro 30 s.",
            Self::Japanese => "AirPods を外すと自動で一時停止します。60秒以内に再装着するか、30秒以内にケースに収納してスピーカーに切り替わると自動で再生を再開します。",
            Self::Korean => "AirPods를 빼면 자동으로 일시정지됩니다. 60초 이내에 다시 착용하거나 30초 이내에 케이스에 넣어 스피커로 전환되면 자동으로 재생을 이어갑니다.",
            Self::PortugueseBrazil => "Pausa automaticamente ao retirar os AirPods. Retoma a reprodução ao recolocar em até 60 s ou guardar no estojo em até 30 s.",
            Self::PortuguesePortugal => "Pausa automaticamente ao retirar os AirPods. Retoma a reprodução se recolocados dentro de 60s ou guardados na caixa dentro de 30s para transferir para os altifalantes.",
            Self::Indonesian => "Otomatis menjeda saat melepas AirPods. Otomatis melanjutkan pemutaran jika dimasukkan kembali dalam 60 detik atau disimpan dalam wadah dalam 30 detik untuk diserahkan ke speaker.",
            Self::Vietnamese => "Tự động tạm dừng khi tháo AirPods. Tự động tiếp tục phát nếu đeo lại trong vòng 60 giây hoặc cất vào hộp trong 30 giây để chuyển tiếp âm thanh sang loa.",
            Self::Russian => "Автоматически приостанавливает при снятии AirPods. Возобновляет воспроизведение, если надеть их обратно в течение 60 с или убрать в футляр в течение 30 с.",
            Self::Spanish => "Pausa automáticamente al quitarse los AirPods. Reanuda la reproducción si se vuelven a poner en 60 s o se guardan en el estuche en 30 s.",
        }
    }

    pub fn window_snapping_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🧲 視窗磁吸貼齊",
            Self::SimplifiedChinese => "🧲 窗口磁吸贴齐",
            Self::English => "🧲 Window Snapping",
            Self::French => "🧲 Magnétisme des fenêtres",
            Self::German => "🧲 Fenster-Andocken",
            Self::Italian => "🧲 Finestre magnetiche",
            Self::Japanese => "🧲 ウィンドウ吸着",
            Self::Korean => "🧲 창 자석 맞춤",
            Self::PortugueseBrazil => "🧲 Janelas Magnéticas",
            Self::PortuguesePortugal => "🧲 Janelas Magnéticas",
            Self::Indonesian => "🧲 Penempelan Jendela",
            Self::Vietnamese => "🧲 Hút dính cửa sổ",
            Self::Russian => "🧲 Магнитные окна",
            Self::Spanish => "🧲 Ventanas Magnéticas",
        }
    }

    pub fn window_snapping_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "拖曳視窗靠近螢幕工作區邊緣或 3D 視覺化視窗時自動無縫貼齊 (貼齊距離: 12px)",
            Self::SimplifiedChinese => "拖拽窗口靠近屏幕工作区边缘或 3D 可视化窗口时自动无缝贴齐 (贴齐距离: 12px)",
            Self::English => "Automatically snap to screen edges and 3D Visualizer when dragging nearby (Snap distance: 12px)",
            Self::French => "Aimante automatiquement les fenêtres aux bords de l'écran et au visualiseur 3D (Distance: 12px)",
            Self::German => "Dockt Fenster beim Ziehen automatisch an Bildschirmrändern und am 3D-Visualisierer an (Abstand: 12px)",
            Self::Italian => "Aggancia automaticamente le finestre ai bordi dello schermo e al visualizzatore 3D (Distanza: 12px)",
            Self::Japanese => "画面の端や 3D ビジュアライザーの近くにドラッグした際にピタッと吸着 (吸着距離: 12px)",
            Self::Korean => "화면 가장자리나 3D 시각화 창 근처로 드래그 시 자석처럼 자동 맞춤 (맞춤 거리: 12px)",
            Self::PortugueseBrazil => "Encaixa automaticamente nas bordas da tela e no visualizador 3D (Distância: 12px)",
            Self::PortuguesePortugal => "Encaixa automaticamente nas margens do ecrã e no visualizador 3D ao arrastar para perto (Distância: 12px)",
            Self::Indonesian => "Secara otomatis menempel ke tepi layar dan Visualizer 3D saat menyeret di dekatnya (Jarak tempel: 12px)",
            Self::Vietnamese => "Tự động hút dính vào các cạnh màn hình và Trình trực quan 3D khi kéo lại gần (Khoảng cách hút: 12px)",
            Self::Russian => "Автоматически примагничивает окна к краям экрана и окну 3D-визуализатора (Дистанция: 12px)",
            Self::Spanish => "Alinea automáticamente con los bordes de la pantalla y la ventana del visualizador 3D (Distancia: 12px)",
        }
    }

    pub fn hint_window_snapping_on(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🧲 已啟用磁吸與連動：拖曳主視窗時帶動吸附的視窗同步移動。",
            Self::SimplifiedChinese => "🧲 已启用磁吸与联动：拖拽主窗口时带动吸附的窗口同步移动。",
            Self::English => "🧲 Snapping & linked movement active: dragging the main window moves attached windows together.",
            Self::French => "🧲 Magnétisme et déplacement lié activés : le déplacement de la fenêtre principale entraîne les fenêtres liées.",
            Self::German => "🧲 Magnetisches Andocken aktiv: Bewegen des Hauptfensters bewegt angedockte Fenster mit.",
            Self::Italian => "🧲 Movimento collegato attivo: trascinare la finestra principale sposta anche le finestre agganciate.",
            Self::Japanese => "🧲 吸着・連動移動が有効：メインウィンドウを動かすと吸着したウィンドウも連動します。",
            Self::Korean => "🧲 자석 맞춤 및 연동 이동 활성화: 메인 창 이동 시 결합된 창이 함께 이동합니다.",
            Self::PortugueseBrazil => "🧲 Encaixe e movimento vinculado ativado: mover a janela principal move as janelas fixadas.",
            Self::PortuguesePortugal => "🧲 Encaixe e movimento vinculado ativo: arrastar a janela principal move as janelas anexadas em conjunto.",
            Self::Indonesian => "🧲 Penempelan & pergerakan tertaut aktif: menyeret jendela utama memindahkan jendela yang terpasang bersama-sama.",
            Self::Vietnamese => "🧲 Đang bật hút dính & di chuyển liên kết: kéo cửa sổ chính sẽ di chuyển các cửa sổ gắn liền cùng nhau.",
            Self::Russian => "🧲 Сцепка активна: перетаскивание главного окна перемещает прикрепленные окна.",
            Self::Spanish => "🧲 Alineación y movimiento vinculado activos: mover la ventana principal mueve las ventanas acopladas.",
        }
    }

    pub fn hint_window_snapping_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "視窗可自由擺放，不進行邊界磁吸或連動移動。",
            Self::SimplifiedChinese => "窗口可自由摆放，不进行边界磁吸或联动移动。",
            Self::English => "Windows move freely without magnetic snapping or linked movement.",
            Self::French => "Les fenêtres se déplacent librement sans magnétisme ni mouvement lié.",
            Self::German => "Fenster bewegen sich frei ohne magnetisches Andocken oder Verknüpfung.",
            Self::Italian => "Le finestre si muovono liberamente senza aggancio magnetico o collegamenti.",
            Self::Japanese => "ウィンドウは磁石吸着や連動なしで自由に配置できます。",
            Self::Korean => "창이 자석 맞춤이나 연동 없이 자유롭게 이동합니다.",
            Self::PortugueseBrazil => "As janelas se movem livremente sem encaixe magnético ou movimento vinculado.",
            Self::PortuguesePortugal => "As janelas movem-se livremente sem encaixe magnético ou movimento vinculado.",
            Self::Indonesian => "Jendela bergerak bebas tanpa penempelan magnetik atau pergerakan tertaut.",
            Self::Vietnamese => "Các cửa sổ di chuyển tự do mà không có hút dính từ tính hay liên kết chuyển động.",
            Self::Russian => "Окна перемещаются свободно без примагничивания и сцепки.",
            Self::Spanish => "Las ventanas se mueven libremente sin alineación magnética ni movimiento vinculado.",
        }
    }

    pub fn airpods_both_in_ear_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎧 雙耳配戴中",
            Self::SimplifiedChinese => "🎧 双耳佩戴中",
            Self::English => "🎧 Both In-Ear",
            Self::French => "🎧 Deux dans l'oreille",
            Self::German => "🎧 Beide im Ohr",
            Self::Italian => "🎧 Entrambi nell'orecchio",
            Self::Japanese => "🎧 両耳装着中",
            Self::Korean => "🎧 양쪽 착용 중",
            Self::PortugueseBrazil => "🎧 Ambos no ouvido",
            Self::PortuguesePortugal => "🎧 Ambos no ouvido",
            Self::Indonesian => "🎧 Keduanya di Telinga",
            Self::Vietnamese => "🎧 Cả hai tai đang đeo",
            Self::Russian => "🎧 Оба в ушах",
            Self::Spanish => "🎧 Ambos en el oído",
        }
    }

    pub fn airpods_left_only_in_ear_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎧 左耳配戴中",
            Self::SimplifiedChinese => "🎧 左耳佩戴中",
            Self::English => "🎧 Left In-Ear",
            Self::French => "🎧 Gauche dans l'oreille",
            Self::German => "🎧 Links im Ohr",
            Self::Italian => "🎧 Sinistro nell'orecchio",
            Self::Japanese => "🎧 左耳装着中",
            Self::Korean => "🎧 왼쪽 착용 중",
            Self::PortugueseBrazil => "🎧 Esquerdo no ouvido",
            Self::PortuguesePortugal => "🎧 Esquerdo no ouvido",
            Self::Indonesian => "🎧 Kiri di Telinga",
            Self::Vietnamese => "🎧 Chỉ tai trái đang đeo",
            Self::Russian => "🎧 Левый в ухе",
            Self::Spanish => "🎧 Izquierdo en el oído",
        }
    }

    pub fn airpods_right_only_in_ear_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎧 右耳配戴中",
            Self::SimplifiedChinese => "🎧 右耳佩戴中",
            Self::English => "🎧 Right In-Ear",
            Self::French => "🎧 Droit dans l'oreille",
            Self::German => "🎧 Rechts im Ohr",
            Self::Italian => "🎧 Destro nell'orecchio",
            Self::Japanese => "🎧 右耳装着中",
            Self::Korean => "🎧 오른쪽 착용 중",
            Self::PortugueseBrazil => "🎧 Direito no ouvido",
            Self::PortuguesePortugal => "🎧 Direito no ouvido",
            Self::Indonesian => "🎧 Kanan di Telinga",
            Self::Vietnamese => "🎧 Chỉ tai phải đang đeo",
            Self::Russian => "🎧 Правый в ухе",
            Self::Spanish => "🎧 Derecho en el oído",
        }
    }

    pub fn airpods_in_ear_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎧 配戴中",
            Self::SimplifiedChinese => "🎧 佩戴中",
            Self::English => "🎧 In-Ear",
            Self::French => "🎧 Dans l'oreille",
            Self::German => "🎧 Im Ohr",
            Self::Italian => "🎧 Nell'orecchio",
            Self::Japanese => "🎧 装着中",
            Self::Korean => "🎧 착용 중",
            Self::PortugueseBrazil => "🎧 No ouvido",
            Self::PortuguesePortugal => "🎧 No ouvido",
            Self::Indonesian => "🎧 Di Telinga",
            Self::Vietnamese => "🎧 Đang đeo",
            Self::Russian => "🎧 В ухе",
            Self::Spanish => "🎧 En el oído",
        }
    }

    pub fn airpods_out_of_ear_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🎧 未配戴",
            Self::SimplifiedChinese => "🎧 未佩戴",
            Self::English => "🎧 Out of Ear",
            Self::French => "🎧 Retirés",
            Self::German => "🎧 Nicht im Ohr",
            Self::Italian => "🎧 Non indossato",
            Self::Japanese => "🎧 未装着",
            Self::Korean => "🎧 미착용",
            Self::PortugueseBrazil => "🎧 Fora do ouvido",
            Self::PortuguesePortugal => "🎧 Fora do ouvido",
            Self::Indonesian => "🎧 Di Luar Telinga",
            Self::Vietnamese => "🎧 Đã tháo ra",
            Self::Russian => "🎧 Не в ухе",
            Self::Spanish => "🎧 Fuera del oído",
        }
    }

    pub fn airpods_battery_live_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPods 連線中（即時電量資料）",
            Self::SimplifiedChinese => "AirPods 连接中（实时电量数据）",
            Self::English => "AirPods connected (live battery data)",
            Self::French => "AirPods connectés (données de batterie en direct)",
            Self::German => "AirPods verbunden (aktuelle Akkudaten)",
            Self::Italian => "AirPods connessi (dati batteria in tempo reale)",
            Self::Japanese => "AirPods 接続中（リアルタイムバッテリー情報）",
            Self::Korean => "AirPods 연결됨 (실시간 배터리 정보)",
            Self::PortugueseBrazil => "AirPods conectados (dados de bateria em tempo real)",
            Self::PortuguesePortugal => "AirPods ligados (dados de bateria em tempo real)",
            Self::Indonesian => "AirPods terhubung (data baterai langsung)",
            Self::Vietnamese => "AirPods đã kết nối (dữ liệu pin thời gian thực)",
            Self::Russian => "AirPods подключены (актуальный заряд)",
            Self::Spanish => "AirPods conectados (datos de batería en tiempo real)",
        }
    }

    pub fn airpods_battery_stale_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPods 已斷線（可能為離線前最後記錄的電量）",
            Self::SimplifiedChinese => "AirPods 已断开（可能为离线前最后记录的电量）",
            Self::English => "AirPods disconnected (cached last-known battery)",
            Self::French => "AirPods déconnectés (dernier niveau de batterie mémorisé)",
            Self::German => "AirPods getrennt (zwischengespeicherter Akkustand)",
            Self::Italian => "AirPods disconnessi (ultimo livello di batteria registrato)",
            Self::Japanese => "AirPods 切断済み（前回接続時のキャッシュ情報）",
            Self::Korean => "AirPods 연결 끊김 (이전 연결 시 캐시된 정보)",
            Self::PortugueseBrazil => "AirPods desconectados (último nível de bateria registrado)",
            Self::PortuguesePortugal => "AirPods desligados (último nível de bateria conhecido em cache)",
            Self::Indonesian => "AirPods terputus (baterai terakhir yang diketahui dalam cache)",
            Self::Vietnamese => "AirPods đã ngắt kết nối (bộ nhớ cache mức pin cuối cùng ghi nhận)",
            Self::Russian => "AirPods отключены (сохраненный уровень заряда)",
            Self::Spanish => "AirPods desconectados (último nivel de batería registrado)",
        }
    }

    pub fn airpods_connected_status(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線中",
            Self::SimplifiedChinese => "连接中",
            Self::English => "Connected",
            Self::French => "Connecté",
            Self::German => "Verbunden",
            Self::Italian => "Connesso",
            Self::Japanese => "接続中",
            Self::Korean => "연결됨",
            Self::PortugueseBrazil => "Conectado",
            Self::PortuguesePortugal => "Ligado",
            Self::Indonesian => "Terhubung",
            Self::Vietnamese => "Đã kết nối",
            Self::Russian => "Подключено",
            Self::Spanish => "Conectado",
        }
    }

    pub fn airpods_disconnected_status(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "已斷線",
            Self::SimplifiedChinese => "已断开",
            Self::English => "Disconnected",
            Self::French => "Déconnecté",
            Self::German => "Getrennt",
            Self::Italian => "Disconnesso",
            Self::Japanese => "切断",
            Self::Korean => "연결 끊김",
            Self::PortugueseBrazil => "Desconectado",
            Self::PortuguesePortugal => "Desligado",
            Self::Indonesian => "Terputus",
            Self::Vietnamese => "Đã ngắt kết nối",
            Self::Russian => "Отключено",
            Self::Spanish => "Desconectado",
        }
    }

    pub fn airplay_receiver_section(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "📡 接收端設定",
            Self::SimplifiedChinese => "📡 接收端设置",
            Self::English => "📡 Receiver Settings",
            Self::French => "📡 Paramètres du récepteur",
            Self::German => "📡 Empfängereinstellungen",
            Self::Italian => "📡 Impostazioni ricevitore",
            Self::Japanese => "📡 受信機設定",
            Self::Korean => "📡 수신기 설정",
            Self::PortugueseBrazil => "📡 Configurações do receptor",
            Self::PortuguesePortugal => "📡 Definições do Recetor",
            Self::Indonesian => "📡 Pengaturan Penerima",
            Self::Vietnamese => "📡 Cài đặt Bộ nhận",
            Self::Russian => "📡 Настройки приемника",
            Self::Spanish => "📡 Configuración del receptor",
        }
    }

    pub fn airplay_receiver_enable(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "啟用無線音訊接收端",
            Self::SimplifiedChinese => "启用无线音频接收端",
            Self::English => "Enable Wireless Audio Receiver",
            Self::French => "Activer le récepteur audio sans fil",
            Self::German => "Kabellosen Audioempfänger aktivieren",
            Self::Italian => "Abilita ricevitore audio wireless",
            Self::Japanese => "ワイヤレス音声受信を有効にする",
            Self::Korean => "무선 오디오 수신기 활성화",
            Self::PortugueseBrazil => "Ativar receptor de áudio sem fio",
            Self::PortuguesePortugal => "Ativar Recetor de Áudio Sem Fios",
            Self::Indonesian => "Aktifkan Penerima Audio Nirkabel",
            Self::Vietnamese => "Bật Bộ nhận âm thanh không dây",
            Self::Russian => "Включить беспроводной аудиоприемник",
            Self::Spanish => "Habilitar receptor de audio inalámbrico",
        }
    }

    pub fn airplay_receiver_name_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🏷️ 廣播名稱：",
            Self::SimplifiedChinese => "🏷️ 广播名称：",
            Self::English => "🏷️ Broadcast Name:",
            Self::French => "🏷️ Nom de diffusion :",
            Self::German => "🏷️ Broadcast-Name:",
            Self::Italian => "🏷️ Nome di trasmissione:",
            Self::Japanese => "🏷️ ブロードキャスト名：",
            Self::Korean => "🏷️ 브로드캐스트 이름:",
            Self::PortugueseBrazil => "🏷️ Nome de transmissão:",
            Self::PortuguesePortugal => "🏷️ Nome de Transmissão:",
            Self::Indonesian => "🏷️ Nama Siaran:",
            Self::Vietnamese => "🏷️ Tên phát sóng:",
            Self::Russian => "🏷️ Имя вещания:",
            Self::Spanish => "🏷️ Nombre de difusión:",
        }
    }

    pub fn airplay_receiver_status_idle(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚪ 等待 AirPlay 裝置投放音訊...",
            Self::SimplifiedChinese => "⚪ 等待 AirPlay 设备投放音频...",
            Self::English => "⚪ Waiting for AirPlay connection...",
            Self::French => "⚪ En attente de connexion AirPlay...",
            Self::German => "⚪ Warte auf AirPlay-Verbindung...",
            Self::Italian => "⚪ In attesa di connessione AirPlay...",
            Self::Japanese => "⚪ AirPlay 音声の受信を待機中...",
            Self::Korean => "⚪ AirPlay 연결 대기 중...",
            Self::PortugueseBrazil => "⚪ Aguardando conexão AirPlay...",
            Self::PortuguesePortugal => "⚪ A aguardar ligação AirPlay...",
            Self::Indonesian => "⚪ Menunggu koneksi AirPlay...",
            Self::Vietnamese => "⚪ Đang chờ kết nối AirPlay...",
            Self::Russian => "⚪ Ожидание подключения AirPlay...",
            Self::Spanish => "⚪ Esperando conexión AirPlay...",
        }
    }

    pub fn airplay_receiver_status_streaming(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🟢 正在接收並播放 iPhone / Mac 音訊",
            Self::SimplifiedChinese => "🟢 正在接收并播放 iPhone / Mac 音频",
            Self::English => "🟢 Receiving & playing AirPlay audio stream",
            Self::French => "🟢 Réception et lecture du flux AirPlay en cours",
            Self::German => "🟢 AirPlay-Audiostream wird empfangen und wiedergegeben",
            Self::Italian => "🟢 Ricezione e riproduzione flusso AirPlay in corso",
            Self::Japanese => "🟢 AirPlay 音声を受信・再生中",
            Self::Korean => "🟢 AirPlay 오디오 스트림 수신 및 재생 중",
            Self::PortugueseBrazil => "🟢 Recebendo e reproduzindo áudio AirPlay",
            Self::PortuguesePortugal => "🟢 A receber e a reproduzir fluxo de áudio AirPlay",
            Self::Indonesian => "🟢 Menerima & memutar streaming audio AirPlay",
            Self::Vietnamese => "🟢 Đang nhận & phát luồng âm thanh AirPlay",
            Self::Russian => "🟢 Приём и воспроизведение аудиопотока AirPlay",
            Self::Spanish => "🟢 Recibiendo y reproduciendo audio AirPlay",
        }
    }

    pub fn airplay_receiver_sync_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⏱️ 影音同步補償：",
            Self::SimplifiedChinese => "⏱️ 音画同步补偿：",
            Self::English => "⏱️ A/V Sync Compensation:",
            Self::French => "⏱️ Compensation synchro audio/vidéo :",
            Self::German => "⏱️ Audio/Video-Synchronisationsausgleich:",
            Self::Italian => "⏱️ Compensazione sincro audio/video:",
            Self::Japanese => "⏱️ 音声・映像の同期補正：",
            Self::Korean => "⏱️ 음성·영상 싱크 보정:",
            Self::PortugueseBrazil => "⏱️ Compensação de sincronia A/V:",
            Self::PortuguesePortugal => "⏱️ Compensação de Sincronização A/V:",
            Self::Indonesian => "⏱️ Kompensasi Sinkronisasi A/V:",
            Self::Vietnamese => "⏱️ Bù trừ đồng bộ âm thanh/hình ảnh (A/V):",
            Self::Russian => "⏱️ Синхронизация звука и видео:",
            Self::Spanish => "⏱️ Compensación de sincronización A/V:",
        }
    }

    pub fn airplay_receiver_sync_none(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "無 (即時反應)",
            Self::SimplifiedChinese => "无 (实时响应)",
            Self::English => "None (Instant)",
            Self::French => "Aucun (Instané)",
            Self::German => "Keine (Sofort)",
            Self::Italian => "Nessuna (Istantanea)",
            Self::Japanese => "なし (リアルタイム)",
            Self::Korean => "없음 (즉시 반응)",
            Self::PortugueseBrazil => "Nenhuma (Instantânea)",
            Self::PortuguesePortugal => "Nenhuma (Instantânea)",
            Self::Indonesian => "Tidak Ada (Instan)",
            Self::Vietnamese => "Không (Tức thì)",
            Self::Russian => "Нет (мгновенно)",
            Self::Spanish => "Ninguna (Instantánea)",
        }
    }

    pub fn airplay_receiver_sync_standard(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "適中 (推薦)",
            Self::SimplifiedChinese => "适中 (推荐)",
            Self::English => "Medium (Recommended)",
            Self::French => "Moyen (Recommandé)",
            Self::German => "Mittel (Empfohlen)",
            Self::Italian => "Medio (Consigliato)",
            Self::Japanese => "標準 (おすすめ)",
            Self::Korean => "표준 (권장)",
            Self::PortugueseBrazil => "Médio (Recomendado)",
            Self::PortuguesePortugal => "Média (Recomendada)",
            Self::Indonesian => "Sedang (Disarankan)",
            Self::Vietnamese => "Trung bình (Khuyên dùng)",
            Self::Russian => "Средняя (рекомендуется)",
            Self::Spanish => "Medio (Recomendado)",
        }
    }

    pub fn airplay_receiver_sync_strong(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "較強",
            Self::SimplifiedChinese => "较强",
            Self::English => "Strong",
            Self::French => "Fort",
            Self::German => "Stark",
            Self::Italian => "Forte",
            Self::Japanese => "強め",
            Self::Korean => "강하게",
            Self::PortugueseBrazil => "Forte",
            Self::PortuguesePortugal => "Forte",
            Self::Indonesian => "Kuat",
            Self::Vietnamese => "Mạnh",
            Self::Russian => "Сильная",
            Self::Spanish => "Fuerte",
        }
    }

    pub fn airplay_receiver_sync_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 提示：看 YouTube 等影片若聲音跑在字幕前面，請選「適中」使聲音等待畫面；純聽音樂可選「無」獲得最即時反應。",
            Self::SimplifiedChinese => "💡 提示：看 YouTube 等视频若声音跑在字幕前面，请选“适中”使声音等待画面；纯听音乐可选“无”获得最即时响应。",
            Self::English => "💡 Tip: If audio leads subtitles on YouTube/videos, select 'Medium' to sync with video; select 'None' for instant response when playing music.",
            Self::French => "💡 Astuce : Si le son précède les sous-titres sur YouTube/vidéos, choisissez 'Moyen' ; choisissez 'Aucun' pour écouter de la musique.",
            Self::German => "💡 Tipp: Eilt der Ton auf YouTube/Videos den Untertiteln voraus, wählen Sie 'Mittel' ; für Musik wählen Sie 'Keine' für sofortige Reaktion.",
            Self::Italian => "💡 Suggerimento: Se l'audio anticipa i sottotitoli su YouTube/video, scegli 'Medio' ; per la musica scegli 'Nessuna' per reazione immediata.",
            Self::Japanese => "💡 ヒント：YouTube 等の動画で音声が字幕より早く進む場合は「標準」で映像に合わせます。音楽鑑賞時は「なし」で即時反応します。",
            Self::Korean => "💡 팁: YouTube 등 영상 시청 시 소리가 자막보다 빠르면 '표준'으로 맞춥니다. 음악 감상 시에는 '없음'으로 즉시 반응합니다.",
            Self::PortugueseBrazil => "💡 Dica: Se o áudio adiantar legendas no YouTube/vídeos, escolha 'Médio' ; para música, escolha 'Nenhuma' para resposta imediata.",
            Self::PortuguesePortugal => "💡 Dica: Se o áudio adiantar legendas no YouTube/vídeos, selecione \"Média\" para sincronizar com o vídeo; selecione \"Nenhuma\" para resposta instantânea ao ouvir música.",
            Self::Indonesian => "💡 Tips: Jika audio mendahului subtitle di YouTube/video, pilih \"Sedang\" untuk menyinkronkan dengan video; pilih \"Tidak Ada\" untuk respons instan saat memutar musik.",
            Self::Vietnamese => "💡 Mẹo: Nếu âm thanh đi trước phụ đề trên YouTube/video, hãy chọn \"Trung bình\" để đồng bộ với video; chọn \"Không\" để phản hồi tức thì khi nghe nhạc.",
            Self::Russian => "💡 Примечание: Если звук опережает субтитры на YouTube/видео, выберите «Средняя» ; для музыки выберите «Нет» для мгновенного отклика.",
            Self::Spanish => "💡 Consejo: Si el audio se adelanta a los subtítulos en YouTube/vídeos, elija 'Medio' ; para música elija 'Ninguna' para respuesta inmediata.",
        }
    }

    pub fn receiver_guide_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 使用指南與疑難排解",
            Self::SimplifiedChinese => "💡 使用指南与疑难解答",
            Self::English => "💡 Guide & Troubleshooting",
            Self::French => "💡 Guide et dépannage",
            Self::German => "💡 Anleitung & Fehlerbehebung",
            Self::Italian => "💡 Guida e risoluzione dei problemi",
            Self::Japanese => "💡 使用ガイドとトラブルシューティング",
            Self::Korean => "💡 사용 가이드 및 문제 해결",
            Self::PortugueseBrazil => "💡 Guia e solução de problemas",
            Self::PortuguesePortugal => "💡 Guia e Resolução de Problemas",
            Self::Indonesian => "💡 Panduan & Pemecahan Masalah",
            Self::Vietnamese => "💡 Hướng dẫn & Khắc phục sự cố",
            Self::Russian => "💡 Руководство и устранение неполадок",
            Self::Spanish => "💡 Guía y solución de problemas",
        }
    }

    pub fn receiver_guide_basic_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "📱 基本連線：",
            Self::SimplifiedChinese => "📱 基本连接：",
            Self::English => "📱 Basic Connection:",
            Self::French => "📱 Connexion de base :",
            Self::German => "📱 Grundlegende Verbindung:",
            Self::Italian => "📱 Connessione di base:",
            Self::Japanese => "📱 基本接続：",
            Self::Korean => "📱 기본 연결:",
            Self::PortugueseBrazil => "📱 Conexão básica:",
            Self::PortuguesePortugal => "📱 Ligação Básica:",
            Self::Indonesian => "📱 Koneksi Dasar:",
            Self::Vietnamese => "📱 Kết nối cơ bản:",
            Self::Russian => "📱 Базовое подключение:",
            Self::Spanish => "📱 Conexión básica:",
        }
    }

    pub fn receiver_guide_basic_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "請將 iPhone / iPad 與本電腦連接至同一 Wi-Fi 網路。在 iOS「控制中心」點擊音訊播放卡片右上角的投播圖示，點選本裝置即可連線。",
            Self::SimplifiedChinese => "请将 iPhone / iPad 与本电脑连接至同一 Wi-Fi 网络。在 iOS「控制中心」点击音频播放卡片右上角的投播图标，点击本设备即可连接。",
            Self::English => "Connect your iPhone / iPad to the same Wi-Fi network as this PC. In the iOS Control Center, tap the AirPlay icon on the Now Playing card and select this device.",
            Self::French => "Connectez votre iPhone / iPad au même réseau Wi-Fi que ce PC. Dans le Centre de contrôle iOS, appuyez sur l'icône de diffusion et sélectionnez cet appareil.",
            Self::German => "Verbinden Sie Ihr iPhone / iPad mit demselben WLAN wie diesen PC. Tippen Sie im iOS-Kontrollzentrum auf das AirPlay-Symbol und wählen Sie dieses Gerät aus.",
            Self::Italian => "Connetti iPhone / iPad alla stessa rete Wi-Fi di questo PC. Nel Centro di controllo iOS, tocca l'icona di trasmissione e seleziona questo dispositivo.",
            Self::Japanese => "iPhone / iPad と本 PC を同じ Wi-Fi ネットワークに接続してください。iOS の「コントロールセンター」で再生カード右上のキャストアイコンをタップし、本デバイスを選択して接続します。",
            Self::Korean => "iPhone / iPad를 이 PC와 동일한 Wi-Fi 네트워크에 연결하세요. iOS '제어 센터'에서 재생 카드의 캐스트 아이콘을 탭하고 이 기기를 선택하여 연결합니다.",
            Self::PortugueseBrazil => "Conecte seu iPhone / iPad à mesma rede Wi-Fi deste PC. Na Central de Controle do iOS, toque no ícone de transmissão e selecione este dispositivo.",
            Self::PortuguesePortugal => "Ligue o seu iPhone / iPad à mesma rede Wi-Fi deste PC. Na Central de Controlo do iOS, toque no ícone AirPlay no cartão A Reproduzir e selecione este dispositivo.",
            Self::Indonesian => "Hubungkan iPhone / iPad Anda ke jaringan Wi-Fi yang sama dengan PC ini. Di Pusat Kontrol iOS, ketuk ikon AirPlay pada kartu Sedang Diputar dan pilih perangkat ini.",
            Self::Vietnamese => "Kết nối iPhone / iPad của bạn vào cùng mạng Wi-Fi với PC này. Trong Trung tâm điều khiển iOS, chạm vào biểu tượng AirPlay trên thẻ Đang phát và chọn thiết bị này.",
            Self::Russian => "Подключите iPhone / iPad к той же сети Wi-Fi, что и этот ПК. В Пункте управления iOS коснитесь значка трансляции и выберите это устройство.",
            Self::Spanish => "Conecte su iPhone / iPad a la misma red Wi-Fi que este PC. En el Centro de control de iOS, toque el icono de transmisión y seleccione este dispositivo.",
        }
    }

    pub fn receiver_guide_troubleshoot_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔄 播放異常排解：",
            Self::SimplifiedChinese => "🔄 播放异常排查：",
            Self::English => "🔄 Playback Troubleshooting:",
            Self::French => "🔄 Dépannage de la lecture :",
            Self::German => "🔄 Fehlerbehebung bei der Wiedergabe:",
            Self::Italian => "🔄 Risoluzione problemi riproduzione:",
            Self::Japanese => "🔄 再生トラブルの解消：",
            Self::Korean => "🔄 재생 오류 해결:",
            Self::PortugueseBrazil => "🔄 Solução de problemas de reprodução:",
            Self::PortuguesePortugal => "🔄 Resolução de Problemas de Reprodução:",
            Self::Indonesian => "🔄 Pemecahan Masalah Pemutaran:",
            Self::Vietnamese => "🔄 Khắc phục sự cố phát lại:",
            Self::Russian => "🔄 Устранение ошибок воспроизведения:",
            Self::Spanish => "🔄 Solución de problemas de reproducción:",
        }
    }

    pub fn receiver_guide_troubleshoot_desc(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "若應用程式重啟後，iOS 提示「播放錯誤」或無法出聲，請在 iOS 控制中心先切換回「iPhone 喇叭」，再重新點選連接本裝置，即可立即恢復正常。",
            Self::SimplifiedChinese => "若应用程序重启后，iOS 提示「播放错误」或无法出声，请在 iOS 控制中心先切换回「iPhone 扬声器」，再重新点击连接本设备，即可立即恢复正常。",
            Self::English => "If iOS shows 'Playback Error' or there is no sound after restarting this app, switch audio back to 'iPhone Speaker' in the iOS Control Center, then re-select this device to resume immediately.",
            Self::French => "Si iOS affiche 'Erreur de lecture' après le redémarrage, basculez vers le 'Haut-parleur iPhone' dans le Centre de contrôle, puis sélectionnez à nouveau cet appareil.",
            Self::German => "Falls iOS nach dem Neustart 'Wiedergabefehler' anzeigt, wechseln Sie im Kontrollzentrum kurz zu 'iPhone-Lautsprecher' und wählen Sie dieses Gerät erneut aus.",
            Self::Italian => "Se iOS mostra 'Errore di riproduzione' dopo il riavvio, torna a 'Altoparlante iPhone' nel Centro di controllo, quindi riseleziona questo dispositivo.",
            Self::Japanese => "本アプリの再起動後に iOS で「再生エラー」が表示されたり音が出ない場合は、iOS のコントロールセンターで一度「iPhone スピーカー」に戻してから、再度本デバイスを選択して接続し直してください。",
            Self::Korean => "앱 재시작 후 iOS에서 '재생 오류'가 표시되거나 소리가 나지 않는 경우, iOS 제어 센터에서 먼저 'iPhone 스피커'로 전환한 다음 이 기기를 다시 선택하면 즉시 복구됩니다.",
            Self::PortugueseBrazil => "Se o iOS exibir 'Erro de reprodução' após reiniciar o aplicativo, volte para o 'Alto-falante do iPhone' na Central de Controle e selecione este dispositivo novamente.",
            Self::PortuguesePortugal => "Se o iOS apresentar \"Erro de Reprodução\" ou não houver som após reiniciar esta aplicação, mude o áudio de volta para \"Altifalante do iPhone\" na Central de Controlo do iOS e selecione novamente este dispositivo para retomar imediatamente.",
            Self::Indonesian => "Jika iOS menampilkan \"Kesalahan Pemutaran\" atau tidak ada suara setelah memulai ulang aplikasi ini, alihkan audio kembali ke \"Speaker iPhone\" di Pusat Kontrol iOS, lalu pilih kembali perangkat ini untuk melanjutkan segera.",
            Self::Vietnamese => "Nếu iOS báo \"Lỗi phát lại\" hoặc không có âm thanh sau khi khởi động lại ứng dụng này, hãy chuyển âm thanh về \"Loa iPhone\" trong Trung tâm điều khiển iOS, sau đó chọn lại thiết bị này để tiếp tục ngay lập tức.",
            Self::Russian => "Если после перезапуска приложения iOS показывает «Ошибка воспроизведения», переключитесь на «Динамик iPhone» в Пункте управления, затем снова выберите это устройство.",
            Self::Spanish => "Si iOS muestra 'Error de reproducción' tras reiniciar la app, cambie a 'Altavoz del iPhone' en el Centro de control y vuelva a seleccionar este dispositivo.",
        }
    }

    pub fn airplay_receiver_section_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 音訊投放",
            Self::SimplifiedChinese => "AirPlay 音频投送",
            Self::English => "AirPlay Audio Casting",
            Self::French => "Diffusion audio AirPlay",
            Self::German => "AirPlay-Audio-Streaming",
            Self::Italian => "Streaming audio AirPlay",
            Self::Japanese => "AirPlay 音声ストリーミング",
            Self::Korean => "AirPlay 오디오 캐스팅",
            Self::PortugueseBrazil => "Transmissão de áudio AirPlay",
            Self::PortuguesePortugal => "Transmissão de Áudio AirPlay",
            Self::Indonesian => "Pemeran Audio AirPlay",
            Self::Vietnamese => "Truyền phát âm thanh AirPlay",
            Self::Russian => "Аудиотрансляция AirPlay",
            Self::Spanish => "Transmisión de audio AirPlay",
        }
    }

    pub fn airplay_receiver_standby_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 接收端待命中",
            Self::SimplifiedChinese => "AirPlay 接收端待命中",
            Self::English => "AirPlay Receiver Standby",
            Self::French => "Récepteur AirPlay en attente",
            Self::German => "AirPlay-Empfänger bereit",
            Self::Italian => "Ricevitore AirPlay in standby",
            Self::Japanese => "AirPlay 受信待機中",
            Self::Korean => "AirPlay 수신 대기 중",
            Self::PortugueseBrazil => "Receptor AirPlay em espera",
            Self::PortuguesePortugal => "Recetor AirPlay em Espera",
            Self::Indonesian => "Penerima AirPlay Siaga",
            Self::Vietnamese => "Bộ nhận AirPlay đang chờ",
            Self::Russian => "Приемник AirPlay ожидает",
            Self::Spanish => "Receptor AirPlay en espera",
        }
    }

    pub fn airplay_receiver_streaming_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "播放中",
            Self::SimplifiedChinese => "播放中",
            Self::English => "Playing",
            Self::French => "Lecture",
            Self::German => "Wiedergabe",
            Self::Italian => "In riproduzione",
            Self::Japanese => "再生中",
            Self::Korean => "재생 중",
            Self::PortugueseBrazil => "Reproduzindo",
            Self::PortuguesePortugal => "Em reprodução",
            Self::Indonesian => "Memutar",
            Self::Vietnamese => "Đang phát",
            Self::Russian => "Воспроизведение",
            Self::Spanish => "Reproduciendo",
        }
    }

    pub fn airplay_receiver_standby_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "待命",
            Self::SimplifiedChinese => "待命",
            Self::English => "Standby",
            Self::French => "En attente",
            Self::German => "Bereit",
            Self::Italian => "Standby",
            Self::Japanese => "待機",
            Self::Korean => "대기",
            Self::PortugueseBrazil => "Espera",
            Self::PortuguesePortugal => "Em espera",
            Self::Indonesian => "Siaga",
            Self::Vietnamese => "Chế độ chờ",
            Self::Russian => "Ожидание",
            Self::Spanish => "Espera",
        }
    }

    pub fn airplay_receiver_connected_badge(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "已連線",
            Self::SimplifiedChinese => "已连接",
            Self::English => "Connected",
            Self::French => "Connecté",
            Self::German => "Verbunden",
            Self::Italian => "Connesso",
            Self::Japanese => "接続済み",
            Self::Korean => "연결됨",
            Self::PortugueseBrazil => "Conectado",
            Self::PortuguesePortugal => "Ligado",
            Self::Indonesian => "Terhubung",
            Self::Vietnamese => "Đã kết nối",
            Self::Russian => "Подключено",
            Self::Spanish => "Conectado",
        }
    }

    pub fn airplay_receiver_paused_suffix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => " (已暫停)",
            Self::SimplifiedChinese => " (已暂停)",
            Self::English => " (Paused)",
            Self::French => " (En pause)",
            Self::German => " (Pausiert)",
            Self::Italian => " (In pausa)",
            Self::Japanese => " (一時停止中)",
            Self::Korean => " (일시 중지됨)",
            Self::PortugueseBrazil => " (Pausado)",
            Self::PortuguesePortugal => " (Em pausa)",
            Self::Indonesian => " (Dijeda)",
            Self::Vietnamese => " (Đã tạm dừng)",
            Self::Russian => " (Пауза)",
            Self::Spanish => " (En pausa)",
        }
    }

    pub fn airplay_receiver_connect_name_prefix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線名稱: ",
            Self::SimplifiedChinese => "连接名称: ",
            Self::English => "Connect name: ",
            Self::French => "Nom de connexion : ",
            Self::German => "Verbindungsname: ",
            Self::Italian => "Nome connessione: ",
            Self::Japanese => "接続名: ",
            Self::Korean => "연결 이름: ",
            Self::PortugueseBrazil => "Nome de conexão: ",
            Self::PortuguesePortugal => "Nome de ligação: ",
            Self::Indonesian => "Nama koneksi: ",
            Self::Vietnamese => "Tên kết nối: ",
            Self::Russian => "Имя для подключения: ",
            Self::Spanish => "Nombre de conexión: ",
        }
    }

    pub fn airplay_receiver_casting_suffix(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => " 投放中",
            Self::SimplifiedChinese => " 投送中",
            Self::English => " Casting",
            Self::French => " en diffusion",
            Self::German => " streamt",
            Self::Italian => " in streaming",
            Self::Japanese => " からキャスト中",
            Self::Korean => " 캐스팅 중",
            Self::PortugueseBrazil => " transmitindo",
            Self::PortuguesePortugal => " a transmitir",
            Self::Indonesian => " Mentransmisikan",
            Self::Vietnamese => " Đang truyền phát",
            Self::Russian => " транслирует",
            Self::Spanish => " transmitiendo",
        }
    }

    pub fn airplay_receiver_casting_default_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "AirPlay 裝置投放中",
            Self::SimplifiedChinese => "AirPlay 设备投送中",
            Self::English => "AirPlay Device Casting",
            Self::French => "Appareil AirPlay en diffusion",
            Self::German => "AirPlay-Gerät streamt",
            Self::Italian => "Dispositivo AirPlay in streaming",
            Self::Japanese => "AirPlay デバイスからキャスト中",
            Self::Korean => "AirPlay 기기 캐스팅 중",
            Self::PortugueseBrazil => "Dispositivo AirPlay transmitindo",
            Self::PortuguesePortugal => "Dispositivo AirPlay a transmitir",
            Self::Indonesian => "Pemeran Perangkat AirPlay",
            Self::Vietnamese => "Thiết bị AirPlay đang phát",
            Self::Russian => "Устройство AirPlay транслирует",
            Self::Spanish => "Dispositivo AirPlay transmitiendo",
        }
    }

    pub fn airplay_receiver_phone_volume(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "手機端音量: ",
            Self::SimplifiedChinese => "手机端音量: ",
            Self::English => "Phone volume: ",
            Self::French => "Volume appareil : ",
            Self::German => "Gerätelautstärke: ",
            Self::Italian => "Volume dispositivo: ",
            Self::Japanese => "端末の音量: ",
            Self::Korean => "스마트폰 볼륨: ",
            Self::PortugueseBrazil => "Volume do aparelho: ",
            Self::PortuguesePortugal => "Volume do telemóvel: ",
            Self::Indonesian => "Volume ponsel: ",
            Self::Vietnamese => "Âm lượng điện thoại: ",
            Self::Russian => "Громкость устройства: ",
            Self::Spanish => "Volumen del dispositivo: ",
        }
    }

    pub fn airplay_receiver_max_dynamic_range(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "✨ 最大動態範圍",
            Self::SimplifiedChinese => "✨ 最大动态范围",
            Self::English => "✨ Max Dynamic Range",
            Self::French => "✨ Plage dynamique max",
            Self::German => "✨ Max. Dynamikbereich",
            Self::Italian => "✨ Max gamma dinamica",
            Self::Japanese => "✨ 最大ダイナミックレンジ",
            Self::Korean => "✨ 최대 다이내믹 레인지",
            Self::PortugueseBrazil => "✨ Alcance dinâmico máx.",
            Self::PortuguesePortugal => "✨ Alcance Dinâmico Máx.",
            Self::Indonesian => "✨ Rentang Dinamis Maks",
            Self::Vietnamese => "✨ Dải động tối đa",
            Self::Russian => "✨ Макс. динамика",
            Self::Spanish => "✨ Rango dinámico máx.",
        }
    }

    pub fn airplay_receiver_max_dynamic_tooltip(&self, phone_vol: Option<u8>) -> String {
        match self {
            Self::TraditionalChinese => match phone_vol {
                Some(v) => format!("以 100% 原始最大動態範圍播出中（已忽略手機端音量減損，當前手機音量為 {}%）", v),
                None => "以 100% 原始最大動態範圍播出中（已忽略手機端音量減損）".to_string(),
            },
            Self::SimplifiedChinese => match phone_vol {
                Some(v) => format!("以 100% 原始最大动态范围播出中（已忽略手机端音量减损，当前手机音量为 {}%）", v),
                None => "以 100% 原始最大动态范围播出中（已忽略手机端音量减损）".to_string(),
            },
            Self::English => match phone_vol {
                Some(v) => format!("Playing in 100% maximum dynamic range (phone attenuation ignored, current phone volume: {}%)", v),
                None => "Playing in 100% maximum dynamic range (phone attenuation ignored)".to_string(),
            },
            Self::French => match phone_vol {
                Some(v) => format!("Lecture en plage dynamique maximale à 100 % (atténuation ignorée, volume actuel : {} %)", v),
                None => "Lecture en plage dynamique maximale à 100 % (atténuation du téléphone ignorée)".to_string(),
            },
            Self::German => match phone_vol {
                Some(v) => format!("Wiedergabe mit 100 % max. Dynamikbereich (Lautstärkedämpfung ignoriert, Smartphone: {} %)", v),
                None => "Wiedergabe mit 100 % max. Dynamikbereich (Lautstärkedämpfung ignoriert)".to_string(),
            },
            Self::Italian => match phone_vol {
                Some(v) => format!("Riproduzione al 100% della gamma dinamica (attenuazione ignorata, volume attuale: {}%)", v),
                None => "Riproduzione al 100% della gamma dinamica massima (attenuazione ignorata)".to_string(),
            },
            Self::Japanese => match phone_vol {
                Some(v) => format!("100% 最大ダイナミックレンジで再生中（端末音量の減衰を無視、現在の端末音量: {}%）", v),
                None => "100% 最大ダイナミックレンジで再生中（端末音量の減衰を無視）".to_string(),
            },
            Self::Korean => match phone_vol {
                Some(v) => format!("100% 최대 다이내믹 레인지로 재생 중 (휴대폰 볼륨 감쇠 무시, 현재 볼륨: {}%)", v),
                None => "100% 최대 다이내믹 레인지로 재생 중 (휴대폰 볼륨 감쇠 무시)".to_string(),
            },
            Self::PortugueseBrazil => match phone_vol {
                Some(v) => format!("Reproduzindo em 100% de alcance dinâmico máximo (atenuação ignorada, volume atual: {}%)", v),
                None => "Reproduzindo em 100% de alcance dinâmico máximo (atenuação ignorada)".to_string(),
            },
            Self::PortuguesePortugal => match phone_vol {
                    Some(v) => format!("A reproduzir com 100% de alcance dinâmico máximo (atenuação ignorada, volume atual: {}%)", v),
                    None => "A reproduzir com 100% de alcance dinâmico máximo (atenuação ignorada)".to_string(),
                },
            Self::Indonesian => match phone_vol {
                    Some(v) => format!("Memutar pada rentang dinamis maks 100% (atenuasi diabaikan, volume ponsel saat ini: {}%)", v),
                    None => "Memutar pada rentang dinamis maks 100% (atenuasi ponsel diabaikan)".to_string(),
                },
            Self::Vietnamese => match phone_vol {
                    Some(v) => format!("Đang phát ở dải động tối đa 100% (bỏ qua suy giảm âm lượng, âm lượng điện thoại: {}%)", v),
                    None => "Đang phát ở dải động tối đa 100% (bỏ qua suy giảm âm lượng của điện thoại)".to_string(),
                },
            Self::Russian => match phone_vol {
                Some(v) => format!("Воспроизведение со 100% макс. динамическим диапазоном (ослабление игнорируется, телефон: {}%)", v),
                None => "Воспроизведение со 100% максимальным динамическим диапазоном (ослабление игнорируется)".to_string(),
            },
            Self::Spanish => match phone_vol {
                Some(v) => format!("Reproduciendo con rango dinámico máximo al 100% (atenuación ignorada, volumen actual: {}%)", v),
                None => "Reproduciendo con rango dinámico máximo al 100% (atenuación del dispositivo ignorada)".to_string(),
            },
        }
    }

    pub fn airplay_receiver_resume_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "點擊以嘗試喚醒手機繼續播放",
            Self::SimplifiedChinese => "点击以尝试唤醒手机继续播放",
            Self::English => "Click to wake phone and resume playback",
            Self::French => "Cliquez pour réveiller le téléphone et reprendre",
            Self::German => "Klicken, um Smartphone zu wecken und fortzusetzen",
            Self::Italian => "Fai clic per riattivare il telefono e riprendere",
            Self::Japanese => "クリックして端末を復帰し再生を再開",
            Self::Korean => "클릭하여 휴대폰을 깨우고 재생 재개",
            Self::PortugueseBrazil => "Clique para acordar o aparelho e retomar a reprodução",
            Self::PortuguesePortugal => "Clique para acordar o telemóvel e retomar a reprodução",
            Self::Indonesian => "Klik untuk membangunkan ponsel dan melanjutkan pemutaran",
            Self::Vietnamese => "Nhấp để đánh thức điện thoại và tiếp tục phát lại",
            Self::Russian => "Нажмите, чтобы разбудить телефон и возобновить",
            Self::Spanish => "Haz clic para reactivar el teléfono y reanudar",
        }
    }

    pub fn airplay_receiver_no_device_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "尚未連線，請先由 iPhone 控制中心選取本裝置",
            Self::SimplifiedChinese => "尚未连线，请先由 iPhone 控制中心选取本装置",
            Self::English => "Not connected. Please select this device in iPhone Control Center",
            Self::French => "Non connecté. Veuillez sélectionner cet appareil dans le Centre de contrôle de l'iPhone",
            Self::German => "Nicht verbunden. Bitte dieses Gerät im iPhone-Kontrollzentrum auswählen",
            Self::Italian => "Non connesso. Seleziona questo dispositivo nel Centro di controllo dell'iPhone",
            Self::Japanese => "未接続です。iPhoneのコントロールセンターから本端末を選択してください",
            Self::Korean => "연결되지 않음. iPhone 제어 센터에서 이 기기를 선택하세요",
            Self::PortugueseBrazil => "Não conectado. Selecione este dispositivo na Central de Controle do iPhone",
            Self::PortuguesePortugal => "Não ligado. Selecione este dispositivo na Central de Controlo do iPhone",
            Self::Indonesian => "Tidak terhubung. Silakan pilih perangkat ini di Pusat Kontrol iPhone",
            Self::Vietnamese => "Chưa kết nối. Vui lòng chọn thiết bị này trong Trung tâm điều khiển iPhone",
            Self::Russian => "Не подключено. Выберите это устройство в Пункте управления iPhone",
            Self::Spanish => "No conectado. Selecciona este dispositivo en el Centro de control del iPhone",
        }
    }

    pub fn airplay_receiver_wake_failed(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚠️ 無法喚醒手機，請至手機重新點選投放",
            Self::SimplifiedChinese => "⚠️ 无法唤醒手机，请至手机重新点选投放",
            Self::English => "⚠️ Could not wake device, please re-cast from phone",
            Self::French => "⚠️ Impossible de réveiller l'appareil, veuillez relancer depuis le téléphone",
            Self::German => "⚠️ Gerät konnte nicht geweckt werden, bitte vom Smartphone neu übertragen",
            Self::Italian => "⚠️ Impossibile riattivare il dispositivo, ritrasmetti dal telefono",
            Self::Japanese => "⚠️ 端末を復帰できません。スマホから再度キャストしてください",
            Self::Korean => "⚠️ 기기를 깨울 수 없습니다. 휴대폰에서 다시 전송하세요",
            Self::PortugueseBrazil => "⚠️ Não foi possível acordar o aparelho, retransmita pelo celular",
            Self::PortuguesePortugal => "⚠️ Não foi possível acordar o dispositivo, volte a transmitir a partir do telemóvel",
            Self::Indonesian => "⚠️ Tidak dapat membangunkan perangkat, silakan transmisikan ulang dari ponsel",
            Self::Vietnamese => "⚠️ Không thể đánh thức thiết bị, vui lòng truyền phát lại từ điện thoại",
            Self::Russian => "⚠️ Не удалось разбудить устройство, запустите трансляцию заново с телефона",
            Self::Spanish => "⚠️ No se pudo reactivar el dispositivo, vuelve a transmitir desde el teléfono",
        }
    }

    pub fn airplay_receiver_require_connect(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "📱 請先由手機控制中心點選投放至本裝置",
            Self::SimplifiedChinese => "📱 请先由手机控制中心点选投放至本装置",
            Self::English => "📱 Please select this device in iPhone Control Center first",
            Self::French => "📱 Veuillez d'abord sélectionner cet appareil dans le Centre de contrôle",
            Self::German => "📱 Bitte zuerst dieses Gerät im iPhone-Kontrollzentrum auswählen",
            Self::Italian => "📱 Seleziona prima questo dispositivo nel Centro di controllo",
            Self::Japanese => "📱 先にiPhoneのコントロールセンターから本端末を選択してください",
            Self::Korean => "📱 먼저 iPhone 제어 센터에서 이 기기를 선택하세요",
            Self::PortugueseBrazil => "📱 Selecione primeiro este dispositivo na Central de Controle",
            Self::PortuguesePortugal => "📱 Selecione primeiro este dispositivo na Central de Controlo do iPhone",
            Self::Indonesian => "📱 Silakan pilih perangkat ini di Pusat Kontrol iPhone terlebih dahulu",
            Self::Vietnamese => "📱 Vui lòng chọn thiết bị này trong Trung tâm điều khiển iPhone trước",
            Self::Russian => "📱 Сначала выберите это устройство в Пункте управления iPhone",
            Self::Spanish => "📱 Selecciona primero este dispositivo en el Centro de control del iPhone",
        }
    }

    pub fn btn_prev_track(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "上一首",
            Self::SimplifiedChinese => "上一首",
            Self::English => "Previous Track",
            Self::French => "Piste précédente",
            Self::German => "Vorheriger Titel",
            Self::Italian => "Traccia precedente",
            Self::Japanese => "前の曲",
            Self::Korean => "이전 곡",
            Self::PortugueseBrazil => "Faixa anterior",
            Self::PortuguesePortugal => "Faixa anterior",
            Self::Indonesian => "Trek Sebelumnya",
            Self::Vietnamese => "Bản nhạc trước",
            Self::Russian => "Предыдущий трек",
            Self::Spanish => "Pista anterior",
        }
    }

    pub fn btn_play_pause(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "播放 / 暫停",
            Self::SimplifiedChinese => "播放 / 暂停",
            Self::English => "Play / Pause",
            Self::French => "Lecture / Pause",
            Self::German => "Wiedergabe / Pause",
            Self::Italian => "Riproduci / Pausa",
            Self::Japanese => "再生 / 一時停止",
            Self::Korean => "재생 / 일시정지",
            Self::PortugueseBrazil => "Reproduzir / Pausar",
            Self::PortuguesePortugal => "Reproduzir / Pausar",
            Self::Indonesian => "Putar / Jeda",
            Self::Vietnamese => "Phát / Tạm dừng",
            Self::Russian => "Воспроизведение / Пауза",
            Self::Spanish => "Reproducir / Pausa",
        }
    }

    pub fn btn_next_track(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "下一首",
            Self::SimplifiedChinese => "下一首",
            Self::English => "Next Track",
            Self::French => "Piste suivante",
            Self::German => "Nächster Titel",
            Self::Italian => "Traccia successiva",
            Self::Japanese => "次の曲",
            Self::Korean => "다음 곡",
            Self::PortugueseBrazil => "Próxima faixa",
            Self::PortuguesePortugal => "Faixa seguinte",
            Self::Indonesian => "Trek Berikutnya",
            Self::Vietnamese => "Bản nhạc tiếp theo",
            Self::Russian => "Следующий трек",
            Self::Spanish => "Pista siguiente",
        }
    }

    pub fn btn_phone_volume_max(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔊 設為最大",
            Self::SimplifiedChinese => "🔊 设为最大",
            Self::English => "🔊 Set to Max",
            Self::French => "🔊 Régler au max",
            Self::German => "🔊 Auf Max stellen",
            Self::Italian => "🔊 Imposta al massimo",
            Self::Japanese => "🔊 最大にする",
            Self::Korean => "🔊 최대로 설정",
            Self::PortugueseBrazil => "🔊 Ajustar ao máximo",
            Self::PortuguesePortugal => "🔊 Ajustar ao máximo",
            Self::Indonesian => "🔊 Atur ke Maksimal",
            Self::Vietnamese => "🔊 Đặt mức tối đa",
            Self::Russian => "🔊 На максимум",
            Self::Spanish => "🔊 Poner al máximo",
        }
    }

    pub fn btn_phone_volume_max_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "透過 DACP 將手機端投放音量調至 100%（最大聲）",
            Self::SimplifiedChinese => "通过 DACP 将手机端投送音量调至 100%（最大声）",
            Self::English => "Set phone casting volume to 100% via DACP",
            Self::French => "Régler le volume de l'appareil à 100 % via DACP",
            Self::German => "Telefon-Streaming-Lautstärke über DACP auf 100 % einstellen",
            Self::Italian => "Imposta il volume di streaming del telefono al 100% tramite DACP",
            Self::Japanese => "DACP経由でスマートフォンのキャスト音量を100%（最大）に設定",
            Self::Korean => "DACP를 통해 스마트폰 캐스팅 볼륨을 100%(최대)로 설정",
            Self::PortugueseBrazil => "Ajustar o volume de transmissão do celular para 100% via DACP",
            Self::PortuguesePortugal => "Ajustar o volume de transmissão do telemóvel para 100% via DACP",
            Self::Indonesian => "Atur volume transmisi ponsel ke 100% melalui DACP",
            Self::Vietnamese => "Đặt âm lượng truyền phát của điện thoại thành 100% qua DACP",
            Self::Russian => "Установить громкость трансляции на 100% через DACP",
            Self::Spanish => "Ajustar el volumen de transmisión del teléfono al 100% mediante DACP",
        }
    }

    pub fn airplay_receiver_auto_max_volume_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "忽略手機音量調節，保持音訊最大動態範圍",
            Self::SimplifiedChinese => "忽略手机音量调节，保持音频最大动态范围",
            Self::English => "Ignore phone volume adjustments, preserve maximum dynamic range",
            Self::French => "Ignorer le réglage du volume du téléphone, préserver la plage dynamique maximale",
            Self::German => "Telefon-Lautstärkeanpassung ignorieren, maximale Dynamik beibehalten",
            Self::Italian => "Ignora la regolazione del volume del telefono, mantieni la gamma dinamica massima",
            Self::Japanese => "スマホの音量調節を無視し、音声の最大ダイナミックレンジを維持",
            Self::Korean => "스마트폰 볼륨 조절을 무시하고 오디오 최대 다이내믹 레인지 유지",
            Self::PortugueseBrazil => "Ignorar ajuste de volume do celular, manter o alcance dinâmico máximo",
            Self::PortuguesePortugal => "Ignorar ajuste de volume do telemóvel, manter o alcance dinâmico máximo",
            Self::Indonesian => "Abaikan penyesuaian volume ponsel, pertahankan rentang dinamis audio maksimum",
            Self::Vietnamese => "Bỏ qua điều chỉnh âm lượng điện thoại, duy trì dải động âm thanh tối đa",
            Self::Russian => "Игнорировать громкость телефона, сохраняя максимальный динамический диапазон",
            Self::Spanish => "Ignorar el ajuste de volumen del teléfono, mantener el rango dinámico máximo",
        }
    }

    pub fn airplay_receiver_output_device_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音訊轉發輸出裝置",
            Self::SimplifiedChinese => "音频转发输出设备",
            Self::English => "Audio Forwarding Device",
            Self::French => "Périphérique de transfert audio",
            Self::German => "Audio-Weiterleitungsgerät",
            Self::Italian => "Dispositivo di inoltro audio",
            Self::Japanese => "音声転送出力デバイス",
            Self::Korean => "오디오 전달 출력 장치",
            Self::PortugueseBrazil => "Dispositivo de encaminhamento de áudio",
            Self::PortuguesePortugal => "Dispositivo de reencaminhamento de áudio",
            Self::Indonesian => "Perangkat Penerusan Audio",
            Self::Vietnamese => "Thiết bị chuyển tiếp âm thanh",
            Self::Russian => "Устройство перенаправления звука",
            Self::Spanish => "Dispositivo de reenvío de audio",
        }
    }

    pub fn airplay_receiver_output_device_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "可選系統預設或指定虛擬音效卡 (如 VB-Cable、VoiceMeeter) 供其他軟體進行聲音後製處理",
            Self::SimplifiedChinese => "可选系统默认或指定虚拟声卡 (如 VB-Cable、VoiceMeeter) 供其他软件进行声音后期处理",
            Self::English => "Select system default or virtual cables (e.g. VB-Cable, VoiceMeeter) to route audio into DAW, OBS, or DSP tools",
            Self::French => "Choisissez le périphérique par défaut ou virtuel (VB-Cable, VoiceMeeter) pour le traitement audio externe",
            Self::German => "Wählen Sie das Standardgerät oder virtuelle Audiokabel (z. B. VB-Cable, VoiceMeeter) für externe Effekte",
            Self::Italian => "Scegli il dispositivo predefinito o virtuale (VB-Cable, VoiceMeeter) per l'elaborazione audio esterna",
            Self::Japanese => "システム既定または仮想オーディオ (VB-Cable、VoiceMeeter 等) を指定し、DAW や外部ソフトで音響処理が可能",
            Self::Korean => "시스템 기본값 또는 가상 오디오 케이블(VB-Cable, VoiceMeeter 등)로 지정하여 DAW나 외부 소프트웨어에서 후처리 가능",
            Self::PortugueseBrazil => "Selecione o dispositivo padrão ou cabo virtual (ex: VB-Cable, VoiceMeeter) para processamento em outros softwares",
            Self::PortuguesePortugal => "Selecione o dispositivo predefinido ou cabo virtual (ex: VB-Cable, VoiceMeeter) para processamento em software externo",
            Self::Indonesian => "Pilih default sistem atau kabel virtual (mis. VB-Cable, VoiceMeeter) untuk pemrosesan efek audio di aplikasi lain",
            Self::Vietnamese => "Chọn mặc định hệ thống hoặc cáp âm thanh ảo (như VB-Cable, VoiceMeeter) để xử lý hiệu ứng âm thanh qua phần mềm khác",
            Self::Russian => "Выберите устройство по умолчанию или виртуальный кабель (VB-Cable, VoiceMeeter) для внешней обработки звука",
            Self::Spanish => "Seleccione el dispositivo predeterminado o virtual (VB-Cable, VoiceMeeter) para posprocesamiento de audio",
        }
    }

    pub fn airplay_receiver_apply_effects_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "轉發時套用專屬音效處理 (等化器、諧波激勵與溫暖潤色)",
            Self::SimplifiedChinese => "转发时应用专属音效处理 (均衡器、谐波激励与温暖润色)",
            Self::English => "Apply dedicated audio effects when forwarding (EQ, Harmonic Exciter & Warmth)",
            Self::French => "Appliquer les effets audio dédiés lors du transfert (égaliseur, exciteur et chaleur)",
            Self::German => "Dedizierte Audioeffekte bei Weiterleitung anwenden (EQ, Harmonic Exciter & Wärme)",
            Self::Italian => "Applica effetti audio dedicati durante l'inoltro (equalizzatore, eccitatore e calore)",
            Self::Japanese => "転送時に専用サウンド効果を適用 (EQ、ハーモニックエキサイター、温かみ)",
            Self::Korean => "전달 시 전용 음향 효과 적용 (EQ, 하모닉 익사이터 및 온기 강화)",
            Self::PortugueseBrazil => "Aplicar efeitos de áudio dedicados ao encaminhar (equalizador, excitador e calor)",
            Self::PortuguesePortugal => "Aplicar efeitos de áudio dedicados ao reencaminhar (equalizador, excitador e calor)",
            Self::Indonesian => "Terapkan efek audio khusus saat meneruskan (EQ, Harmonic Exciter & Warmth)",
            Self::Vietnamese => "Áp dụng hiệu ứng âm thanh riêng khi chuyển tiếp (EQ, Tăng cường sóng hài & Độ ấm)",
            Self::Russian => "Применять выделенные звуковые эффекты при перенаправлении (эквалайзер, сатурация и теплота)",
            Self::Spanish => "Aplicar efectos de audio dedicados al reenviar (ecualizador, excitador y calidez)",
        }
    }

    pub fn airplay_receiver_btn_adjust_eq(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "調整接收專屬等化器 ➜",
            Self::SimplifiedChinese => "调整接收专属均衡器 ➜",
            Self::English => "Configure Receiver Audio Effects ➜",
            Self::French => "Configurer les effets du récepteur ➜",
            Self::German => "Empfänger-Audioeffekte anpassen ➜",
            Self::Italian => "Configura effetti del ricevitore ➜",
            Self::Japanese => "受信専用イコライザーを調整 ➜",
            Self::Korean => "수신 전용 이퀄라이저 설정 ➜",
            Self::PortugueseBrazil => "Ajustar efeitos do receptor ➜",
            Self::PortuguesePortugal => "Ajustar efeitos do recetor ➜",
            Self::Indonesian => "Atur Efek Audio Penerima ➜",
            Self::Vietnamese => "Tùy chỉnh hiệu ứng bộ thu ➜",
            Self::Russian => "Настроить эквалайзер приёмника ➜",
            Self::Spanish => "Ajustar efectos del receptor ➜",
        }
    }

    pub fn eq_profile_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "音效設定檔",
            Self::SimplifiedChinese => "音效配置文件",
            Self::English => "Audio Profile",
            Self::French => "Profil audio",
            Self::German => "Audioprofil",
            Self::Italian => "Profilo audio",
            Self::Japanese => "オーディオプロファイル",
            Self::Korean => "오디오 프로필",
            Self::PortugueseBrazil => "Perfil de áudio",
            Self::PortuguesePortugal => "Perfil de áudio",
            Self::Indonesian => "Profil Audio",
            Self::Vietnamese => "Hồ sơ âm thanh",
            Self::Russian => "Профиль звука",
            Self::Spanish => "Perfil de audio",
        }
    }

    pub fn eq_profile_normal(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🖥️ 一般播放 / HomePod",
            Self::SimplifiedChinese => "🖥️ 普通播放 / HomePod",
            Self::English => "🖥️ Playback / HomePod",
            Self::French => "🖥️ Lecture / HomePod",
            Self::German => "🖥️ Wiedergabe / HomePod",
            Self::Italian => "🖥️ Riproduzione / HomePod",
            Self::Japanese => "🖥️ 通常再生 / HomePod",
            Self::Korean => "🖥️ 일반 재생 / HomePod",
            Self::PortugueseBrazil => "🖥️ Reprodução / HomePod",
            Self::PortuguesePortugal => "🖥️ Reprodução / HomePod",
            Self::Indonesian => "🖥️ Pemutaran / HomePod",
            Self::Vietnamese => "🖥️ Phát lại / HomePod",
            Self::Russian => "🖥️ Воспроизведение / HomePod",
            Self::Spanish => "🖥️ Reproducción / HomePod",
        }
    }

    pub fn eq_profile_receiver(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "📲 音訊接收轉發",
            Self::SimplifiedChinese => "📲 音频接收转发",
            Self::English => "📲 Receiver Forwarding",
            Self::French => "📲 Transfert récepteur",
            Self::German => "📲 Empfänger-Weiterleitung",
            Self::Italian => "📲 Inoltro ricevitore",
            Self::Japanese => "📲 音声受信・転送",
            Self::Korean => "📲 오디오 수신 전달",
            Self::PortugueseBrazil => "📲 Encaminhamento de recepção",
            Self::PortuguesePortugal => "📲 Reencaminhamento de receção",
            Self::Indonesian => "📲 Penerusan Penerima",
            Self::Vietnamese => "📲 Chuyển tiếp nhận âm",
            Self::Russian => "📲 Перенаправление приёма",
            Self::Spanish => "📲 Reenvío de receptor",
        }
    }

    pub fn eq_profile_receiver_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 正在調整 AirPlay 接收轉發時的專屬等化器與聲音增強效果，此設定完全獨立於一般播放。",
            Self::SimplifiedChinese => "💡 正在调整 AirPlay 接收转发时的专属均衡器与声音增强效果，此设置完全独立于普通播放。",
            Self::English => "💡 Tuning dedicated EQ and sound enhancers for AirPlay Receiver forwarding. These settings are completely independent of normal playback.",
            Self::French => "💡 Réglage de l'égaliseur et des effets dédiés au transfert AirPlay. Ces réglages sont indépendants de la lecture standard.",
            Self::German => "💡 Einstellen der dedizierten EQ- und Effekteinstellungen für die AirPlay-Weiterleitung. Diese Einstellungen sind unabhängig von der normalen Wiedergabe.",
            Self::Italian => "💡 Regolazione dell'equalizzatore e degli effetti per l'inoltro AirPlay. Queste impostazioni sono indipendenti dalla riproduzione normale.",
            Self::Japanese => "💡 AirPlay 受信転送時の専用イコライザーとサウンド効果を調整中。通常再生とは完全に独立しています。",
            Self::Korean => "💡 AirPlay 수신 전달 시 전용 이퀄라이저와 음향 효과를 조정 중입니다. 일반 재생 설정과 완전히 독립적입니다.",
            Self::PortugueseBrazil => "💡 Ajustando o equalizador e efeitos dedicados para o encaminhamento AirPlay. Essas configurações são independentes da reprodução padrão.",
            Self::PortuguesePortugal => "💡 A ajustar o equalizador e efeitos dedicados para o reencaminhamento AirPlay. Estas definições são independentes da reprodução predefinida.",
            Self::Indonesian => "💡 Menyesuaikan EQ dan efek audio khusus untuk penerusan AirPlay. Pengaturan ini sepenuhnya independen dari pemutaran biasa.",
            Self::Vietnamese => "💡 Đang điều chỉnh EQ và hiệu ứng riêng cho chuyển tiếp nhận AirPlay. Cài đặt này hoàn toàn độc lập với phát lại thông thường.",
            Self::Russian => "💡 Настройка выделенного эквалайзера и эффектов для перенаправления AirPlay. Эти параметры не влияют на обычное воспроизведение.",
            Self::Spanish => "💡 Ajustando ecualizador y efectos dedicados para el reenvío de AirPlay. Estos ajustes son independientes de la reproducción normal.",
        }
    }


    pub fn status_ao_active(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "本機音訊增強中",
            Self::SimplifiedChinese => "本机音频增强中",
            Self::English => "Local Audio Enhanced",
            Self::French => "Traitement audio local actif",
            Self::German => "Lokale Audio-Optimierung aktiv",
            Self::Italian => "Elaborazione audio locale attiva",
            Self::Japanese => "ローカル音声強化中",
            Self::Korean => "로컬 오디오 향상 중",
            Self::PortugueseBrazil => "Aprimoramento de áudio local ativo",
            Self::PortuguesePortugal => "Melhoramento de áudio local ativo",
            Self::Indonesian => "Peningkatan Audio Lokal Aktif",
            Self::Vietnamese => "Đang nâng cao âm thanh cục bộ",
            Self::Russian => "Локальная обработка активна",
            Self::Spanish => "Mejora de audio local activa",
        }
    }

    pub fn ao_mode_desc_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 應用程式音訊增強中：增強的音訊輸出至選定輸出裝置。若選定輸出裝置與系統音效裝置為同一設備，建議將系統音效路由至不同裝置以避免原音重疊。",
            Self::SimplifiedChinese => "💡 应用程序音频增强中：增强的音频输出至选定输出设备。若选定输出设备与系统音频设备为同一设备，建议将系统音频路由至不同设备以避免原音叠加。",
            Self::English => "💡 Application Audio Enhancement: Enhanced audio is output to the selected device. If the selected output is the same as the system audio device, routing system audio to a different device is recommended to prevent audio overlapping.",
            Self::French => "💡 Amélioration audio de l'application : Le flux audio amélioré est envoyé vers le périphérique sélectionné. Si la sortie sélectionnée et le périphérique audio système sont identiques, il est recommandé de router l'audio système vers un autre périphérique pour éviter la superposition sonore.",
            Self::German => "💡 App-Audio-Optimierung: Der optimierte Ton wird an das ausgewählte Ausgabegerät gesendet. Wenn das gewählte Ausgabegerät mit dem System-Audiogerät identisch ist, wird empfohlen, den Systemton auf ein anderes Gerät umzuleiten, um Tonüberlagerungen zu vermeiden.",
            Self::Italian => "💡 Ottimizzazione audio applicazione: L'audio elaborato è inviato al dispositivo di uscita selezionato. Se il dispositivo selezionato coincide con quello di sistema, si consiglia di instradare l'audio di sistema su un dispositivo diverso per evitare sovrapposizioni sonore.",
            Self::Japanese => "💡 アプリ音声強化中：強化された音声を指定の出力デバイスへ送信します。選択した出力デバイスがシステム既定デバイスと同一の場合、原音の重複を防ぐためにシステム音声を別のデバイスへルーティングすることをおすすめします。",
            Self::Korean => "💡 애플리케이션 오디오 향상 중: 향상된 오디오가 선택한 출력 장치로 전달됩니다. 선택한 출력 장치가 시스템 오디오 장치와 동일한 경우, 소리 겹침을 방지하기 위해 시스템 오디오를 다른 장치로 라우팅하는 것을 권장합니다.",
            Self::PortugueseBrazil => "💡 Aprimoramento de áudio do app: O áudio aprimorado é enviado ao dispositivo de saída selecionado. Se a saída selecionada for o mesmo dispositivo de áudio do sistema, recomenda-se rotear o áudio do sistema para um dispositivo diferente para evitar sobreposição.",
            Self::PortuguesePortugal => "💡 Melhoramento de áudio da aplicação: O áudio melhorado é enviado para o dispositivo selecionado. Se a saída selecionada for o mesmo dispositivo de áudio do sistema, recomenda-se encaminhar o áudio do sistema para um dispositivo diferente para evitar sobreposição de áudio.",
            Self::Indonesian => "💡 Peningkatan Audio Aplikasi: Audio yang ditingkatkan dikeluarkan ke perangkat yang dipilih. Jika output yang dipilih sama dengan perangkat audio sistem, merutekan audio sistem ke perangkat yang berbeda disarankan untuk mencegah audio tumpang tindih.",
            Self::Vietnamese => "💡 Nâng cao âm thanh ứng dụng: Âm thanh nâng cao được phát ra thiết bị đã chọn. Nếu đầu ra được chọn trùng với thiết bị âm thanh hệ thống, bạn nên định tuyến âm thanh hệ thống sang thiết bị khác để tránh âm thanh bị chồng chéo.",
            Self::Russian => "💡 Улучшение звука приложения: Обработанный звук выводится на выбранное устройство. Если выбранное устройство совпадает с системным аудиоустройством, рекомендуется перенаправить системный звук на другое устройство для устранения наложения.",
            Self::Spanish => "💡 Mejora de audio de la aplicación: El audio procesado se envía al dispositivo de salida seleccionado. Si la salida seleccionada es el mismo dispositivo que el de audio del sistema, se recomienda enrutar el audio del sistema a un dispositivo distinto para evitar la superposición.",
        }
    }

    pub fn ao_mode_desc_system(&self) -> &'static str {
        self.ao_mode_desc_system_same()
    }

    pub fn ao_mode_desc_system_same(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 全系統頻譜視覺化中：目前「音訊來源」與「本機輸出裝置」為同一設備，已自動抑制本地聲音輸出以避免迴授嘯叫；DSP 與 EQ 正即時驅動 Paint Pulse 視覺頻譜。建議將本機輸出切換至其他耳機或喇叭以聆聽 10 段 EQ 增強音質。",
            Self::SimplifiedChinese => "💡 全系统频谱可视化中：当前「音频来源」与「本机输出设备」为同一设备，已自动抑制本地声音输出以避免回授啸叫；DSP 与 EQ 实时驱动 Paint Pulse 视觉频谱。建议将本机输出切换至其他耳机或音箱以聆听 10 段 EQ 增强音质。",
            Self::English => "💡 System Audio Spectrum Visualizer: Audio source and local output share the same physical device. Local audio output is automatically suppressed to prevent feedback loops; DSP & EQ are actively driving Paint Pulse. Select a different local output (e.g. headphones) to hear 10-band EQ enhancement.",
            Self::French => "💡 Visualisation du spectre système : La source et la sortie locale partagent le même périphérique. La sortie audio locale est automatiquement coupée pour éviter l'effet Larsen ; le DSP anime Paint Pulse. Sélectionnez une autre sortie locale (ex. un casque) pour écouter l'égaliseur.",
            Self::German => "💡 System-Audiospektrum-Visualisierung: Audioquelle und lokaler Ausgang nutzen dasselbe Gerät. Die lokale Tonausgabe wird automatisch unterdrückt, um Rückkopplungen zu verhindern. Wählen Sie ein anderes lokales Ausgabegerät (z. B. Kopfhörer), um die 10-Band-EQ-Optimierung zu hören.",
            Self::Italian => "💡 Visualizzatore di spettro audio di sistema: Sorgente e uscita locale condividono lo stesso dispositivo. L'uscita locale è soppressa per prevenire feedback; DSP anima Paint Pulse. Seleziona un'altra uscita locale (es. cuffie) per ascoltare l'equalizzatore a 10 bande.",
            Self::Japanese => "💡 システム音声スペクトラム可視化中：音源とローカル出力先が同一デバイスです。ハウリング防止のためローカル音声出力は自動抑制され、DSPとEQはPaint Pulseビジュアライザーを駆動します。10バンドEQの音質強化を聴くには別の出力先（ヘッドホン等）を選択してください。",
            Self::Korean => "💡 시스템 오디오 스펙트럼 시각화 중: 오디오 소스와 로컬 출력이 동일한 장치입니다. 피드백(하울링) 방지를 위해 로컬 오디오 출력이 자동 차단되며 DSP와 EQ는 Paint Pulse 시각화를 구동합니다. 10밴드 EQ 향상 음질을 청취하려면 다른 출력 장치(헤드폰 등)를 선택하세요.",
            Self::PortugueseBrazil => "💡 Visualizador de espectro do sistema: A fonte de áudio e a saída local compartilham o mesmo dispositivo. A saída local é suprimida para evitar microfonia/feedback; DSP anima o Paint Pulse. Selecione outra saída local (ex.: fones) para ouvir o equalizador.",
            Self::PortuguesePortugal => "💡 Visualizador de espetro do sistema: A fonte de áudio e a saída local partilham o mesmo dispositivo físico. A saída de áudio local é suprimida automaticamente para evitar ciclos de feedback/microfonia; o DSP e o equalizador estão a animar o Paint Pulse. Selecione outra saída local (ex.: auscultadores) para ouvir o melhoramento do equalizador de 10 bandas.",
            Self::Indonesian => "💡 Visualizer Spektrum Audio Sistem: Sumber audio dan output lokal berbagi perangkat fisik yang sama. Output audio lokal otomatis ditekan untuk mencegah loop umpan balik; DSP & EQ secara aktif menggerakkan Paint Pulse. Pilih output lokal yang berbeda (misalnya headphone) untuk mendengar peningkatan EQ 10-band.",
            Self::Vietnamese => "💡 Trực quan hóa phổ âm thanh hệ thống: Nguồn âm thanh và đầu ra cục bộ dùng chung một thiết bị vật lý. Đầu ra âm thanh cục bộ sẽ tự động bị khử để ngăn chặn vòng lặp phản hồi âm (hú); DSP & EQ đang kích hoạt hiệu ứng Paint Pulse. Hãy chọn đầu ra cục bộ khác (ví dụ: tai nghe) để nghe âm thanh nâng cao từ EQ 10 băng tần.",
            Self::Russian => "💡 Визуализация спектра системного звука: Источник и локальный выход используют одно устройство. Локальный вывод звука автоматически заблокирован для предотвращения обратной связи; DSP управляет Paint Pulse. Выберите другое локальное устройство (например, наушники), чтобы слушать 10-полосный EQ.",
            Self::Spanish => "💡 Visualizador de espectro del sistema: La fuente de audio y la salida local comparten el mismo dispositivo. La salida local se suprime para evitar acoples/retroalimentación; el DSP anima Paint Pulse. Selecciona otra salida local (ej. auriculares) para escuchar el ecualizador.",
        }
    }

    pub fn ao_mode_desc_system_diff(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 全系統音訊增強中：已將經 DSP 增強的全系統音訊即時輸出至選定裝置（如耳機）。若系統音效裝置同時發出聲音，建議系統音效靜音以獲得純淨增強體驗。",
            Self::SimplifiedChinese => "💡 全系统音频增强中：已将经 DSP 增强的全系统音频实时输出至选定设备（如耳机）。若系统音频设备同时发声，建议系统音频静音以获得纯净增强体验。",
            Self::English => "💡 System Audio Enhancement: Enhanced system audio is actively output to your selected device (e.g. headphones). If the system audio device also emits sound, muting system audio is recommended for a pure enhanced experience.",
            Self::French => "💡 Amélioration audio du système : Le flux audio système amélioré est envoyé en temps réel vers le périphérique sélectionné (ex. casque). Si le périphérique audio système émet aussi du son, couper l'audio système est recommandé pour une expérience optimale.",
            Self::German => "💡 System-Audio-Optimierung: Der optimierte Systemton wird in Echtzeit an das ausgewählte Gerät (z. B. Kopfhörer) ausgegeben. Falls das System-Audiogerät ebenfalls Ton wiedergibt, empfiehlt sich das Stummschalten des Systemtons für ein optimales Klangerlebnis.",
            Self::Italian => "💡 Ottimizzazione audio di sistema: L'audio di sistema elaborato viene inviato in tempo reale al dispositivo selezionato (es. cuffie). Se anche il dispositivo audio di sistema riproduce il suono, si consiglia di disattivare l'audio di sistema per un ascolto ottimale.",
            Self::Japanese => "💡 システム音声強化中：DSP強化後の全システム音声を指定デバイス（ヘッドホン等）へリアルタイム出力しています。システム音声デバイスからも音が出る場合は、システム音声をミュートにすると純粋な強化音声をお楽しみいただけます。",
            Self::Korean => "💡 시스템 오디오 향상 중: DSP로 향상된 전체 시스템 오디오가 선택한 장치(헤드폰 등)로 실시간 출력됩니다. 시스템 오디오 장치에서도 소리가 나는 경우, 시스템 오디오를 음소거하면 더욱 순수한 향상 음질을 감상할 수 있습니다.",
            Self::PortugueseBrazil => "💡 Aprimoramento de áudio do sistema: O áudio aprimorado do sistema é enviado em tempo real ao dispositivo selecionado (ex.: fones). Se o dispositivo de áudio do sistema também emitir som, recomenda-se silenciar o áudio do sistema para uma melhor experiência.",
            Self::PortuguesePortugal => "💡 Melhoramento de áudio do sistema: O áudio melhorado do sistema é enviado ativamente para o dispositivo selecionado (ex.: auscultadores). Se o dispositivo de áudio do sistema também emitir som, recomenda-se cortar o som do áudio do sistema para uma experiência pura melhorada.",
            Self::Indonesian => "💡 Peningkatan Audio Sistem: Audio sistem yang ditingkatkan dikeluarkan secara aktif ke perangkat yang Anda pilih (misalnya headphone). Jika perangkat audio sistem juga mengeluarkan suara, disarankan untuk membisukan audio sistem untuk pengalaman yang ditingkatkan murni.",
            Self::Vietnamese => "💡 Nâng cao âm thanh hệ thống: Âm thanh hệ thống nâng cao đang được xuất trực tiếp ra thiết bị bạn đã chọn (ví dụ: tai nghe). Nếu thiết bị âm thanh hệ thống cũng phát ra tiếng, bạn nên tắt tiếng âm thanh hệ thống để có trải nghiệm thưởng thức trọn vẹn nhất.",
            Self::Russian => "💡 Улучшение звука системы: Обработанный системный звук выводится в реальном времени на выбранное устройство (например, наушники). Если системное аудиоустройство также воспроизводит звук, рекомендуется отключить звук системы.",
            Self::Spanish => "💡 Mejora de audio del sistema: El audio mejorado del sistema se envía en tiempo real al dispositivo seleccionado (ej. auriculares). Si el dispositivo de audio del sistema también emite sonido, se recomienda silenciar el audio del sistema para una experiencia óptima.",
        }
    }

    pub fn btn_popout_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⧉ 獨立視窗",
            Self::SimplifiedChinese => "⧉ 独立窗口",
            Self::English => "⧉ Pop out window",
            Self::French => "⧉ Fenêtre séparée",
            Self::German => "⧉ Eigenes Fenster",
            Self::Italian => "⧉ Finestra separata",
            Self::Japanese => "⧉ 独立ウィンドウ",
            Self::Korean => "⧉ 독립 창으로 열기",
            Self::PortugueseBrazil => "⧉ Destacar janela",
            Self::PortuguesePortugal => "⧉ Janela independente",
            Self::Indonesian => "⧉ Pisahkan jendela",
            Self::Vietnamese => "⧉ Tách cửa sổ riêng",
            Self::Russian => "⧉ Отдельное окно",
            Self::Spanish => "⧉ Separar ventana",
        }
    }

    pub fn btn_dock_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⤓ 嵌入主視窗",
            Self::SimplifiedChinese => "⤓ 嵌入主窗口",
            Self::English => "⤓ Embed in main window",
            Self::French => "⤓ Intégrer à la fenêtre principale",
            Self::German => "⤓ Im Hauptfenster einbetten",
            Self::Italian => "⤓ Incorpora nella finestra principale",
            Self::Japanese => "⤓ メインウィンドウに埋め込む",
            Self::Korean => "⤓ 메인 창으로 넣기",
            Self::PortugueseBrazil => "⤓ Fixar na janela principal",
            Self::PortuguesePortugal => "⤓ Incorporar na janela principal",
            Self::Indonesian => "⤓ Sematkan di jendela utama",
            Self::Vietnamese => "⤓ Nhúng vào cửa sổ chính",
            Self::Russian => "⤓ Встроить в главное окно",
            Self::Spanish => "⤓ Incrustar en ventana principal",
        }
    }

    pub fn eq_window_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod - 音訊等化器",
            Self::SimplifiedChinese => "Win2Pod - 音频均衡器",
            Self::English => "Win2Pod - Audio Equalizer",
            Self::French => "Win2Pod - Égaliseur audio",
            Self::German => "Win2Pod - Audio-Equalizer",
            Self::Italian => "Win2Pod - Equalizzatore audio",
            Self::Japanese => "Win2Pod - オーディオイコライザー",
            Self::Korean => "Win2Pod - 오디오 이퀄라이저",
            Self::PortugueseBrazil => "Win2Pod - Equalizador de áudio",
            Self::PortuguesePortugal => "Win2Pod - Equalizador de áudio",
            Self::Indonesian => "Win2Pod - Equalizer Audio",
            Self::Vietnamese => "Win2Pod - Bộ chỉnh âm Equalizer",
            Self::Russian => "Win2Pod - Аудио эквалайзер",
            Self::Spanish => "Win2Pod - Ecualizador de audio",
        }
    }

    pub fn mini_equalizer_window_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod - 迷你等化器",
            Self::SimplifiedChinese => "Win2Pod - 迷你均衡器",
            Self::English => "Win2Pod - Mini Equalizer",
            Self::French => "Win2Pod - Mini Égaliseur",
            Self::German => "Win2Pod - Mini-Equalizer",
            Self::Italian => "Win2Pod - Mini Equalizzatore",
            Self::Japanese => "Win2Pod - ミニイコライザー",
            Self::Korean => "Win2Pod - 미니 이퀄라이저",
            Self::PortugueseBrazil => "Win2Pod - Mini Equalizador",
            Self::PortuguesePortugal => "Win2Pod - Mini Equalizador",
            Self::Indonesian => "Win2Pod - Equalizer Mini",
            Self::Vietnamese => "Win2Pod - Bộ cân bằng mini",
            Self::Russian => "Win2Pod - Мини-эквалайзер",
            Self::Spanish => "Win2Pod - Mini Ecualizador",
        }
    }

    pub fn setting_detach_equalizer_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🪟 以獨立視窗開啟音訊等化器（支援視窗串右側磁吸）",
            Self::SimplifiedChinese => "🪟 以独立窗口开启音频均衡器（支持窗口串右侧磁吸）",
            Self::English => "🪟 Open equalizer in standalone window (snaps to window chain)",
            Self::French => "🪟 Ouvrir l'égaliseur dans une fenêtre séparée (aimantation)",
            Self::German => "🪟 Equalizer im eigenen Fenster öffnen (an Fensterkette andocken)",
            Self::Italian => "🪟 Apri equalizzatore in finestra separata (aggancio alla catena)",
            Self::Japanese => "🪟 イコライザーを独立ウィンドウで開く（ウィンドウ連の右端へ磁着）",
            Self::Korean => "🪟 이퀄라이저를 독립 창으로 열기 (창 체인 우측에 자석 결합)",
            Self::PortugueseBrazil => "🪟 Abrir equalizador em janela separada (encaixa na cadeia de janelas)",
            Self::PortuguesePortugal => "🪟 Abrir equalizador em janela independente (encaixa na cadeia de janelas)",
            Self::Indonesian => "🪟 Buka equalizer di jendela terpisah (menempel ke rantai jendela)",
            Self::Vietnamese => "🪟 Mở bộ chỉnh âm ở cửa sổ riêng biệt (hút dính vào chuỗi cửa sổ)",
            Self::Russian => "🪟 Открывать эквалайзер в отдельном окне (примагничивание)",
            Self::Spanish => "🪟 Abrir ecualizador en ventana separada (acoplar a cadena)",
        }
    }

    pub fn btn_open_mini_player(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "♫ 迷你播放器",
            Self::SimplifiedChinese => "♫ 迷你播放器",
            Self::English => "♫ Mini Player",
            Self::French => "♫ Mini Lecteur",
            Self::German => "♫ Mini-Player",
            Self::Italian => "♫ Mini Lettore",
            Self::Japanese => "♫ ミニプレーヤー",
            Self::Korean => "♫ 미니 플레이어",
            Self::PortugueseBrazil => "♫ Mini Player",
            Self::PortuguesePortugal => "♫ Mini Leitor",
            Self::Indonesian => "♫ Mini Player",
            Self::Vietnamese => "♫ Trình phát thu nhỏ",
            Self::Russian => "♫ Мини-плеер",
            Self::Spanish => "♫ Mini Reproductor",
        }
    }

    pub fn btn_open_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⦀ 等化器",
            Self::SimplifiedChinese => "⦀ 均衡器",
            Self::English => "⦀ Equalizer",
            Self::French => "⦀ Égaliseur",
            Self::German => "⦀ Equalizer",
            Self::Italian => "⦀ Equalizzatore",
            Self::Japanese => "⦀ イコライザー",
            Self::Korean => "⦀ 이퀄라이저",
            Self::PortugueseBrazil => "⦀ Equalizador",
            Self::PortuguesePortugal => "⦀ Equalizador",
            Self::Indonesian => "⦀ Equalizer",
            Self::Vietnamese => "⦀ Bộ chỉnh âm",
            Self::Russian => "⦀ Эквалайзер",
            Self::Spanish => "⦀ Ecualizador",
        }
    }

    pub fn mini_player_window_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "Win2Pod - 迷你播放器",
            Self::SimplifiedChinese => "Win2Pod - 迷你播放器",
            Self::English => "Win2Pod - Mini Player",
            Self::French => "Win2Pod - Mini Lecteur",
            Self::German => "Win2Pod - Mini-Player",
            Self::Italian => "Win2Pod - Mini Lettore",
            Self::Japanese => "Win2Pod - ミニプレーヤー",
            Self::Korean => "Win2Pod - 미니 플레이어",
            Self::PortugueseBrazil => "Win2Pod - Mini Player",
            Self::PortuguesePortugal => "Win2Pod - Mini Leitor",
            Self::Indonesian => "Win2Pod - Mini Player",
            Self::Vietnamese => "Win2Pod - Trình phát thu nhỏ",
            Self::Russian => "Win2Pod - Мини-плеер",
            Self::Spanish => "Win2Pod - Mini Reproductor",
        }
    }

    pub fn tooltip_open_mini_player(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟迷你播放器並磁吸於等化器上方",
            Self::SimplifiedChinese => "开启迷你播放器并磁吸于均衡器上方",
            Self::English => "Open Mini Player docked above Equalizer",
            Self::French => "Ouvrir le mini lecteur aimanté au-dessus de l'égaliseur",
            Self::German => "Mini-Player öffnen (über Equalizer andocken)",
            Self::Italian => "Apri mini lettore agganciato sopra l'equalizzatore",
            Self::Japanese => "イコライザー上部に磁着するミニプレーヤーを開く",
            Self::Korean => "이퀄라이저 상단에 자석 결합되는 미니 플레이어 열기",
            Self::PortugueseBrazil => "Abrir mini player fixado acima do equalizador",
            Self::PortuguesePortugal => "Abrir mini leitor fixado acima do equalizador",
            Self::Indonesian => "Buka Mini Player menempel di atas Equalizer",
            Self::Vietnamese => "Mở trình phát thu nhỏ hút dính phía trên bộ chỉnh âm",
            Self::Russian => "Открыть мини-плеер с примагничиванием сверху эквалайзера",
            Self::Spanish => "Abrir mini reproductor acoplado arriba del ecualizador",
        }
    }

    pub fn tooltip_open_equalizer(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟音訊等化器並磁吸於播放器下方",
            Self::SimplifiedChinese => "开启音频均衡器并磁吸于播放器下方",
            Self::English => "Open Equalizer docked below Mini Player",
            Self::French => "Ouvrir l'égaliseur aimanté sous le lecteur",
            Self::German => "Equalizer öffnen (unter Player andocken)",
            Self::Italian => "Apri equalizzatore agganciato sotto il lettore",
            Self::Japanese => "プレーヤー下部に磁着するイコライザーを開く",
            Self::Korean => "플레이어 하단에 자석 결합되는 이퀄라이저 열기",
            Self::PortugueseBrazil => "Abrir equalizador fixado abaixo do player",
            Self::PortuguesePortugal => "Abrir equalizador fixado abaixo do leitor",
            Self::Indonesian => "Buka Equalizer menempel di bawah Player",
            Self::Vietnamese => "Mở bộ chỉnh âm hút dính phía dưới trình phát",
            Self::Russian => "Открыть эквалайзер с примагничиванием снизу плеера",
            Self::Spanish => "Abrir ecualizador acoplado debajo del reproductor",
        }
    }

    pub fn mini_player_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "迷你播放器",
            Self::SimplifiedChinese => "迷你播放器",
            Self::English => "Mini Player",
            Self::French => "Mini Lecteur",
            Self::German => "Mini-Player",
            Self::Italian => "Mini Lettore",
            Self::Japanese => "ミニプレーヤー",
            Self::Korean => "미니 플레이어",
            Self::PortugueseBrazil => "Mini Player",
            Self::PortuguesePortugal => "Mini Leitor",
            Self::Indonesian => "Pemutar Mini",
            Self::Vietnamese => "Trình phát mini",
            Self::Russian => "Мини-плеер",
            Self::Spanish => "Mini Reproductor",
        }
    }

    pub fn mini_equalizer_title(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "迷你等化器",
            Self::SimplifiedChinese => "迷你均衡器",
            Self::English => "Mini Equalizer",
            Self::French => "Mini Égaliseur",
            Self::German => "Mini-Equalizer",
            Self::Italian => "Mini Equalizzatore",
            Self::Japanese => "ミニイコライザー",
            Self::Korean => "미니 이퀄라이저",
            Self::PortugueseBrazil => "Mini Equalizador",
            Self::PortuguesePortugal => "Mini Equalizador",
            Self::Indonesian => "Equalizer Mini",
            Self::Vietnamese => "Bộ cân bằng mini",
            Self::Russian => "Мини-эквалайзер",
            Self::Spanish => "Mini Ecualizador",
        }
    }

    pub fn tooltip_show_main_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "叫出 Win2Pod 主視窗",
            Self::SimplifiedChinese => "调出 Win2Pod 主窗口",
            Self::English => "Show Win2Pod Main Window",
            Self::French => "Afficher la fenêtre principale Win2Pod",
            Self::German => "Win2Pod-Hauptfenster anzeigen",
            Self::Italian => "Mostra la finestra principale di Win2Pod",
            Self::Japanese => "Win2Pod メインウィンドウを表示",
            Self::Korean => "Win2Pod 메인 창 표시",
            Self::PortugueseBrazil => "Mostrar janela principal do Win2Pod",
            Self::PortuguesePortugal => "Mostrar janela principal do Win2Pod",
            Self::Indonesian => "Tampilkan Jendela Utama Win2Pod",
            Self::Vietnamese => "Hiện cửa sổ chính Win2Pod",
            Self::Russian => "Показать главное окно Win2Pod",
            Self::Spanish => "Mostrar la ventana principal de Win2Pod",
        }
    }

    pub fn tooltip_dock_main_window(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🧲 將主視窗磁吸至此處",
            Self::SimplifiedChinese => "🧲 将主窗口磁吸至此处",
            Self::English => "🧲 Dock Main Window here",
            Self::French => "🧲 Aimante la fenêtre principale ici",
            Self::German => "🧲 Hauptfenster hier andocken",
            Self::Italian => "🧲 Aggancia la finestra principale qui",
            Self::Japanese => "🧲 メインウィンドウをここに磁着",
            Self::Korean => "🧲 메인 창을 여기에 자석 결합",
            Self::PortugueseBrazil => "🧲 Encaixar a janela principal aqui",
            Self::PortuguesePortugal => "🧲 Encaixar a janela principal aqui",
            Self::Indonesian => "🧲 Tempelkan Jendela Utama ke sini",
            Self::Vietnamese => "🧲 Hút dính Cửa sổ chính vào đây",
            Self::Russian => "🧲 Примагнитить главное окно сюда",
            Self::Spanish => "🧲 Acoplar la ventana principal aquí",
        }
    }

    pub fn tooltip_main_window_docked_focus(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "主視窗已吸附至此群組（點擊聚焦）",
            Self::SimplifiedChinese => "主窗口已吸附至此群组（点击聚焦）",
            Self::English => "Main window is docked here (Click to focus)",
            Self::French => "La fenêtre principale est aimantée ici (Cliquer pour focaliser)",
            Self::German => "Hauptfenster ist hier angedockt (Klicken zum Fokussieren)",
            Self::Italian => "La finestra principale è agganciata qui (Fai clic per mettere a fuoco)",
            Self::Japanese => "メインウィンドウはここに磁着中（クリックでフォーカス）",
            Self::Korean => "메인 창이 여기에 자석 결합됨 (클릭하여 포커스)",
            Self::PortugueseBrazil => "A janela principal está encaixada aqui (Clique para focar)",
            Self::PortuguesePortugal => "A janela principal está encaixada aqui (Clique para focar)",
            Self::Indonesian => "Jendela utama telah ditempelkan di sini (Klik untuk fokus)",
            Self::Vietnamese => "Cửa sổ chính đã được hút dính tại đây (Nhấp để lấy tiêu điểm)",
            Self::Russian => "Главное окно примагничено сюда (Нажмите для фокуса)",
            Self::Spanish => "La ventana principal está acoplada aquí (Haz clic para enfocar)",
        }
    }

    pub fn tooltip_popout_mini_player(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟迷你播放器獨立視窗",
            Self::SimplifiedChinese => "打开迷你播放器独立窗口",
            Self::English => "Open Mini Player window",
            Self::French => "Ouvrir la fenêtre du mini lecteur",
            Self::German => "Mini-Player-Fenster öffnen",
            Self::Italian => "Apri finestra mini lettore",
            Self::Japanese => "ミニプレーヤーウィンドウを開く",
            Self::Korean => "미니 플레이어 창 열기",
            Self::PortugueseBrazil => "Abrir janela do mini player",
            Self::PortuguesePortugal => "Abrir janela do mini leitor",
            Self::Indonesian => "Buka jendela pemutar mini",
            Self::Vietnamese => "Mở cửa sổ trình phát mini",
            Self::Russian => "Открыть окно мини-плеера",
            Self::Spanish => "Abrir ventana del mini reproductor",
        }
    }

    pub fn tooltip_popout_open(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "彈出獨立視窗",
            Self::SimplifiedChinese => "弹出独立窗口",
            Self::English => "Pop out into standalone window",
            Self::French => "Ouvrir dans une fenêtre séparée",
            Self::German => "In eigenem Fenster öffnen",
            Self::Italian => "Apri in finestra separata",
            Self::Japanese => "独立ウィンドウで開く",
            Self::Korean => "독립 창으로 열기",
            Self::PortugueseBrazil => "Destacar em janela separada",
            Self::PortuguesePortugal => "Abrir em janela independente",
            Self::Indonesian => "Buka di jendela terpisah",
            Self::Vietnamese => "Mở trong cửa sổ riêng",
            Self::Russian => "Открыть в отдельном окне",
            Self::Spanish => "Abrir en ventana independiente",
        }
    }

    pub fn tooltip_popout_close(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關閉獨立視窗",
            Self::SimplifiedChinese => "关闭独立窗口",
            Self::English => "Close standalone window",
            Self::French => "Fermer la fenêtre séparée",
            Self::German => "Eigenes Fenster schließen",
            Self::Italian => "Chiudi finestra separata",
            Self::Japanese => "独立ウィンドウを閉じる",
            Self::Korean => "독립 창 닫기",
            Self::PortugueseBrazil => "Fechar janela separada",
            Self::PortuguesePortugal => "Fechar janela independente",
            Self::Indonesian => "Tutup jendela terpisah",
            Self::Vietnamese => "Đóng cửa sổ riêng",
            Self::Russian => "Закрыть отдельное окно",
            Self::Spanish => "Cerrar ventana independiente",
        }
    }

    pub fn eq_profile_normal_hint(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "💡 正在調整本機投放至音訊裝置時的等化器與聲音增強效果。",
            Self::SimplifiedChinese => "💡 正在调整本机投放至音频设备时的均衡器与声音增强效果。",
            Self::English => "💡 Tuning EQ and sound enhancers for standard playback to audio devices.",
            Self::French => "💡 Réglage de l'égaliseur et des effets pour la lecture standard vers les périphériques audio.",
            Self::German => "💡 Optimieren von EQ und Soundeffekten für die Standardwiedergabe auf Audiogeräten.",
            Self::Italian => "💡 Regolazione di EQ ed effetti sonori per la riproduzione standard sui dispositivi audio.",
            Self::Japanese => "💡 オーディオデバイスへの通常再生時のイコライザーとサウンド拡張を調整中。",
            Self::Korean => "💡 오디오 장치로의 일반 재생용 EQ 및 사운드 향상 효과를 조정하는 중입니다.",
            Self::PortugueseBrazil => "💡 Ajustando EQ e efeitos sonoros para reprodução padrão em dispositivos de áudio.",
            Self::PortuguesePortugal => "💡 A ajustar o equalizador e os efeitos sonoros para a reprodução padrão nos dispositivos de áudio.",
            Self::Indonesian => "💡 Menyesuaikan EQ dan penyempurna suara untuk pemutaran standar ke perangkat audio.",
            Self::Vietnamese => "💡 Đang điều chỉnh EQ và hiệu ứng âm thanh cho phát lại tiêu chuẩn đến thiết bị âm thanh.",
            Self::Russian => "💡 Настройка эквалайзера и звуковых эффектов для стандартного воспроизведения.",
            Self::Spanish => "💡 Ajustando el ecualizador y los efectos para la reproducción estándar en dispositivos de audio.",
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum NoticeKind {
    ConnectionLostReconnecting(String),
    ConnectionRestored(String),
    Preempted(String),
    Disconnected(String),
    ResumeReconnecting(String),
    ResumeRestored(String),
    ResumePreempted(String),
    ResumeFailed(String),
    AirPodsOccupied,
    AirPodsOccupiedFallbackHomePod,
}

impl NoticeKind {
    pub fn render(&self, lang: Language) -> String {
        match self {
            Self::ConnectionLostReconnecting(name) => match lang {
                Language::TraditionalChinese => format!("音響「{name}」連線中斷，正在自動重新連線..."),
                Language::SimplifiedChinese => format!("音响「{name}」连接中断，正在自动重新连接..."),
                Language::English => format!("Connection lost to \"{name}\", reconnecting automatically..."),
                Language::French => format!("Connexion perdue avec \"{name}\", reconnexion automatique..."),
                Language::German => format!("Verbindung zu \"{name}\" unterbrochen, automatische Wiederverbindung..."),
                Language::Italian => format!("Connessione persa con \"{name}\", riconnessione automatica in corso..."),
                Language::Japanese => format!("スピーカー「{name}」の接続が切断されました。自動再接続中..."),
                Language::Korean => format!("스피커 \"{name}\" 연결이 끊어졌습니다. 자동 재연결 중..."),
                Language::PortugueseBrazil => format!("Conexão perdida com \"{name}\", reconectando automaticamente..."),
                Language::PortuguesePortugal => format!("Ligação perdida com \"{name}\", a religar automaticamente..."),
                Language::Indonesian => format!("Koneksi terputus ke \"{name}\", menghubungkan kembali otomatis..."),
                Language::Vietnamese => format!("Mất kết nối với \"{name}\", đang tự động kết nối lại..."),
                Language::Russian => format!("Связь с \"{name}\" потеряна, автоматическое переподключение..."),
                Language::Spanish => format!("Conexión perdida con \"{name}\", reconectando automáticamente..."),
            },
            Self::ConnectionRestored(name) => match lang {
                Language::TraditionalChinese => format!("音響「{name}」已恢復連線"),
                Language::SimplifiedChinese => format!("音响「{name}」已恢复连接"),
                Language::English => format!("Connection restored to \"{name}\""),
                Language::French => format!("Connexion rétablie avec \"{name}\""),
                Language::German => format!("Verbindung zu \"{name}\" wiederhergestellt"),
                Language::Italian => format!("Connessione ripristinata con \"{name}\""),
                Language::Japanese => format!("スピーカー「{name}」の接続が復旧しました"),
                Language::Korean => format!("스피커 \"{name}\" 연결이 복구되었습니다"),
                Language::PortugueseBrazil => format!("Conexão restabelecida com \"{name}\""),
                Language::PortuguesePortugal => format!("Ligação restabelecida com \"{name}\""),
                Language::Indonesian => format!("Koneksi dipulihkan ke \"{name}\""),
                Language::Vietnamese => format!("Đã khôi phục kết nối với \"{name}\""),
                Language::Russian => format!("Связь с \"{name}\" восстановлена"),
                Language::Spanish => format!("Conexión restablecida con \"{name}\""),
            },
            Self::Preempted(name) => match lang {
                Language::TraditionalChinese => format!("音響「{name}」已被其他裝置接管，已自動切回靜音。"),
                Language::SimplifiedChinese => format!("音响「{name}」已被其他设备接管，已自动切回静音。"),
                Language::English => format!("Speaker \"{name}\" was taken over by another device, muted."),
                Language::French => format!("L'enceinte \"{name}\" a été reprise par un autre appareil, mise en sourdine."),
                Language::German => format!("Lautsprecher \"{name}\" wurde von einem anderen Gerät übernommen, stummgeschaltet."),
                Language::Italian => format!("L'altoparlante \"{name}\" è stato acquisito da un altro dispositivo, audio disattivato."),
                Language::Japanese => format!("スピーカー「{name}」が他のデバイスに乗っ取られたため、自動ミュートしました。"),
                Language::Korean => format!("스피커 \"{name}\"을(를) 다른 기기에서 사용하여 자동으로 음소거되었습니다."),
                Language::PortugueseBrazil => format!("O alto-falante \"{name}\" foi assumido por outro dispositivo, silenciado."),
                Language::PortuguesePortugal => format!("O altifalante \"{name}\" foi assumido por outro dispositivo, sem som."),
                Language::Indonesian => format!("Speaker \"{name}\" diambil alih oleh perangkat lain, dibisukan."),
                Language::Vietnamese => format!("Loa \"{name}\" đã bị thiết bị khác chiếm quyền, đã tắt tiếng."),
                Language::Russian => format!("Колонка \"{name}\" занята другим устройством, звук выключен."),
                Language::Spanish => format!("El altavoz \"{name}\" fue tomado por otro dispositivo, silenciado."),
            },
            Self::Disconnected(name) => match lang {
                Language::TraditionalChinese => format!("音響「{name}」連線中斷，已自動切回靜音。"),
                Language::SimplifiedChinese => format!("音响「{name}」连接中断，已自动切回静音。"),
                Language::English => format!("Connection lost to \"{name}\", muted."),
                Language::French => format!("Connexion perdue avec \"{name}\", son coupé."),
                Language::German => format!("Verbindung zu \"{name}\" verloren, stummgeschaltet."),
                Language::Italian => format!("Connessione persa con \"{name}\", audio disattivato."),
                Language::Japanese => format!("スピーカー「{name}」の接続が切断されたため、自動ミュートしました。"),
                Language::Korean => format!("스피커 \"{name}\" 연결이 끊겨 자동으로 음소거되었습니다."),
                Language::PortugueseBrazil => format!("Conexão perdida com \"{name}\", silenciado."),
                Language::PortuguesePortugal => format!("Ligação perdida com \"{name}\", sem som."),
                Language::Indonesian => format!("Koneksi terputus ke \"{name}\", dibisukan."),
                Language::Vietnamese => format!("Mất kết nối với \"{name}\", đã tắt tiếng."),
                Language::Russian => format!("Связь с \"{name}\" прервана, звук выключен."),
                Language::Spanish => format!("Conexión perdida con \"{name}\", silenciado."),
            },
            Self::ResumeReconnecting(name) => match lang {
                Language::TraditionalChinese => format!("電腦已甦醒，正在重新連線音響「{name}」..."),
                Language::SimplifiedChinese => format!("电脑已唤醒，正在重新连接音响「{name}」..."),
                Language::English => format!("PC resumed, reconnecting to \"{name}\"...") ,
                Language::French => format!("Reprise de veille du PC, reconnexion à \"{name}\"..."),
                Language::German => format!("PC aufgewacht, verbinde wieder mit \"{name}\"..."),
                Language::Italian => format!("PC riattivato, riconnessione a \"{name}\" in corso..."),
                Language::Japanese => format!("PCがスリープから復帰しました。「{name}」に再接続中..."),
                Language::Korean => format!("PC가 절전 모드에서 해제되었습니다. \"{name}\"에 다시 연결 중..."),
                Language::PortugueseBrazil => format!("PC reativado, reconectando a \"{name}\"..."),
                Language::PortuguesePortugal => format!("PC reativado, a religar a \"{name}\"..."),
                Language::Indonesian => format!("PC bangun, menghubungkan kembali ke \"{name}\"..."),
                Language::Vietnamese => format!("PC đã đánh thức, đang kết nối lại với \"{name}\"..."),
                Language::Russian => format!("ПК вышел из сна, переподключение к \"{name}\"..."),
                Language::Spanish => format!("PC reanudado, reconectando a \"{name}\"..."),
            },
            Self::ResumeRestored(name) => match lang {
                Language::TraditionalChinese => format!("電腦已甦醒，已成功恢復音響「{name}」連線"),
                Language::SimplifiedChinese => format!("电脑已唤醒，已成功恢复音响「{name}」连接"),
                Language::English => format!("PC resumed, successfully restored connection to \"{name}\""),
                Language::French => format!("Reprise de veille du PC, connexion rétablie avec \"{name}\""),
                Language::German => format!("PC aufgewacht, Verbindung zu \"{name}\" erfolgreich wiederhergestellt"),
                Language::Italian => format!("PC riattivato, connessione a \"{name}\" ripristinata con successo"),
                Language::Japanese => format!("PCが復帰し、「{name}」への接続を正常に再開しました"),
                Language::Korean => format!("PC가 해제되어 \"{name}\" 연결이 성공적으로 복구되었습니다"),
                Language::PortugueseBrazil => format!("PC reativado, conexão restabelecida com sucesso a \"{name}\""),
                Language::PortuguesePortugal => format!("PC reativado, ligação restabelecida com sucesso a \"{name}\""),
                Language::Indonesian => format!("PC bangun, koneksi ke \"{name}\" berhasil dipulihkan"),
                Language::Vietnamese => format!("PC đã đánh thức, đã khôi phục thành công kết nối tới \"{name}\""),
                Language::Russian => format!("ПК вышел из сна, связь с \"{name}\" успешно восстановлена"),
                Language::Spanish => format!("PC reanudado, conexión con \"{name}\" restablecida con éxito"),
            },
            Self::ResumePreempted(name) => match lang {
                Language::TraditionalChinese => format!("音響「{name}」已被其他裝置佔用，已切回靜音。"),
                Language::SimplifiedChinese => format!("音响「{name}」已被其他设备占用，已切回静音。"),
                Language::English => format!("Speaker \"{name}\" occupied by another device, muted."),
                Language::French => format!("L'enceinte \"{name}\" est occupée par un autre appareil, son coupé."),
                Language::German => format!("Lautsprecher \"{name}\" von anderem Gerät belegt, stummgeschaltet."),
                Language::Italian => format!("L'altoparlante \"{name}\" è occupato da un altro dispositivo, audio disattivato."),
                Language::Japanese => format!("スピーカー「{name}」は他のデバイスに使用中のため、ミュートに設定しました。"),
                Language::Korean => format!("스피커 \"{name}\"이(가) 다른 기기에서 사용 중이므로 음소거되었습니다."),
                Language::PortugueseBrazil => format!("O alto-falante \"{name}\" está ocupado por outro dispositivo, silenciado."),
                Language::PortuguesePortugal => format!("O altifalante \"{name}\" está ocupado por outro dispositivo, sem som."),
                Language::Indonesian => format!("Speaker \"{name}\" sedang digunakan oleh perangkat lain, dibisukan."),
                Language::Vietnamese => format!("Loa \"{name}\" đang bận bởi thiết bị khác, đã tắt tiếng."),
                Language::Russian => format!("Колонка \"{name}\" занята другим устройством, звук заглушен."),
                Language::Spanish => format!("El altavoz \"{name}\" está ocupado por otro dispositivo, silenciado."),
            },
            Self::ResumeFailed(name) => match lang {
                Language::TraditionalChinese => format!("電腦甦醒後無法重新連線至「{name}」，已切回靜音。"),
                Language::SimplifiedChinese => format!("电脑唤醒后无法重新连接至「{name}」，已切回静音。"),
                Language::English => format!("Could not reconnect to \"{name}\" after waking, muted."),
                Language::French => format!("Impossible de reconnecter \"{name}\" après la veille, son coupé."),
                Language::German => format!("Konnte nach Aufwachen nicht mit \"{name}\" verbinden, stummgeschaltet."),
                Language::Italian => format!("Impossibile riconnettersi a \"{name}\" dopo la riattivazione, audio disattivato."),
                Language::Japanese => format!("復帰後に「{name}」へ再接続できなかったため、ミュートに設定しました。"),
                Language::Korean => format!("절전 해제 후 \"{name}\"에 다시 연결할 수 없어 음소거되었습니다."),
                Language::PortugueseBrazil => format!("Não foi possível reconectar a \"{name}\" após suspensão, silenciado."),
                Language::PortuguesePortugal => format!("Não foi possível religar a \"{name}\" após suspensão, sem som."),
                Language::Indonesian => format!("Tidak dapat menghubungkan kembali ke \"{name}\" setelah bangun, dibisukan."),
                Language::Vietnamese => format!("Không thể kết nối lại với \"{name}\" sau khi đánh thức, đã tắt tiếng."),
                Language::Russian => format!("Не удалось переподключиться к \"{name}\" после сна, звук отключен."),
                Language::Spanish => format!("No se pudo reconectar a \"{name}\" tras reactivar, silenciado."),
            },
            Self::AirPodsOccupied => match lang {
                Language::TraditionalChinese => "等待 AirPods 藍牙音訊通道就緒中...（若正被 iPhone 等其他裝置使用，請至 Windows 藍牙設定點擊連線）".to_string(),
                Language::SimplifiedChinese => "等待 AirPods 蓝牙音频通道就绪中...（若正被 iPhone 等其他设备使用，请至 Windows 蓝牙设置点击连接）".to_string(),
                Language::English => "Waiting for AirPods Bluetooth audio channel... (If in use by another device like iPhone, connect via Windows Bluetooth Settings)".to_string(),
                Language::French => "En attente du canal audio Bluetooth des AirPods... (S'ils sont utilisés par un autre appareil comme un iPhone, connectez via les paramètres Bluetooth Windows)".to_string(),
                Language::German => "Warte auf AirPods-Bluetooth-Audiokanal... (Falls von einem anderen Gerät wie dem iPhone verwendet, in den Windows-Bluetooth-Einstellungen verbinden)".to_string(),
                Language::Italian => "In attesa del canale audio Bluetooth degli AirPods... (Se in uso da un altro dispositivo come iPhone, connettere nelle impostazioni Bluetooth di Windows)".to_string(),
                Language::Japanese => "AirPods の Bluetooth 音声チャンネルの準備を待機中...（iPhone 等の別機器で使用中の場合は、Windows の Bluetooth 設定から接続してください）".to_string(),
                Language::Korean => "AirPods Bluetooth 오디오 채널 준비 대기 중... (iPhone 등 다른 기기에서 사용 중인 경우 Windows Bluetooth 설정에서 연결하세요)".to_string(),
                Language::PortugueseBrazil => "Aguardando canal de áudio Bluetooth dos AirPods... (Se em uso por outro dispositivo como iPhone, conecte nas configurações de Bluetooth do Windows)".to_string(),
                Language::PortuguesePortugal => "A aguardar pelo canal de áudio Bluetooth dos AirPods... (Se em utilização por outro dispositivo como o iPhone, ligue nas Definições de Bluetooth do Windows)".to_string(),
                Language::Indonesian => "Menunggu saluran audio Bluetooth AirPods siap... (Jika sedang digunakan oleh perangkat lain seperti iPhone, hubungkan melalui Pengaturan Bluetooth Windows)".to_string(),
                Language::Vietnamese => "Đang chờ kênh âm thanh Bluetooth của AirPods sẵn sàng... (Nếu đang được thiết bị khác như iPhone sử dụng, hãy kết nối trong Cài đặt Bluetooth của Windows)".to_string(),
                Language::Russian => "Ожидание готовности аудиоканала Bluetooth AirPods... (Если используется другим устройством, например iPhone, подключитесь в настройках Bluetooth Windows)".to_string(),
                Language::Spanish => "Esperando el canal de audio Bluetooth de los AirPods... (Si se usa en otro dispositivo como un iPhone, conéctelo en la configuración de Bluetooth de Windows)".to_string(),
            },
            Self::AirPodsOccupiedFallbackHomePod => match lang {
                Language::TraditionalChinese => "AirPods 尚未就緒或被其他裝置佔用，正在連線 HomePod。如欲於電腦使用，請至 Windows 藍牙設定點擊連線。".to_string(),
                Language::SimplifiedChinese => "AirPods 尚未就绪或被其他设备占用，正在连接 HomePod。如欲在电脑使用，请至 Windows 蓝牙设置点击连接。".to_string(),
                Language::English => "AirPods not ready or in use by another device; connecting to HomePod. To use on PC, connect via Windows Bluetooth Settings.".to_string(),
                Language::French => "AirPods non prêts ou utilisés par un autre appareil ; connexion au HomePod. Pour utiliser sur PC, connectez via les paramètres Bluetooth Windows.".to_string(),
                Language::German => "AirPods noch nicht bereit oder anderweitig verwendet; verbinde mit HomePod. Zum Verbinden Windows-Bluetooth-Einstellungen öffnen.".to_string(),
                Language::Italian => "AirPods non pronti o in uso su altro dispositivo; connessione a HomePod. Per usarli su PC, connetterli nelle impostazioni Bluetooth di Windows.".to_string(),
                Language::Japanese => "AirPods の準備が未完了か他の機器で使用中のため、HomePod に接続中。PCで使用する場合は Windows の Bluetooth 設定から接続してください。".to_string(),
                Language::Korean => "AirPods가 준비되지 않았거나 다른 기기에서 사용 중이므로 HomePod에 연결합니다. PC에서 사용하려면 Windows Bluetooth 설정에서 연결하세요.".to_string(),
                Language::PortugueseBrazil => "AirPods não prontos ou em uso por outro dispositivo; conectando ao HomePod. Para usá-los no PC, conecte nas configurações de Bluetooth do Windows.".to_string(),
                Language::PortuguesePortugal => "AirPods não preparados ou em utilização por outro dispositivo; a ligar ao HomePod. Para os utilizar no PC, ligue nas Definições de Bluetooth do Windows.".to_string(),
                Language::Indonesian => "AirPods belum siap atau sedang digunakan perangkat lain; menghubungkan ke HomePod. Untuk menggunakan di PC, hubungkan melalui Pengaturan Bluetooth Windows.".to_string(),
                Language::Vietnamese => "AirPods chưa sẵn sàng hoặc đang được thiết bị khác sử dụng; đang kết nối với HomePod. Để sử dụng trên PC, hãy kết nối trong Cài đặt Bluetooth của Windows.".to_string(),
                Language::Russian => "AirPods не готовы или заняты другим устройством; подключение к HomePod. Для использования на ПК подключите их в настройках Bluetooth Windows.".to_string(),
                Language::Spanish => "AirPods no listos o en uso por otro dispositivo; conectando a HomePod. Para usarlos en PC, conéctelos en la configuración de Bluetooth de Windows.".to_string(),
            },
        }
    }

    pub fn title(&self, lang: Language) -> &'static str {
        match self {
            Self::ConnectionRestored(_) | Self::ResumeRestored(_) => match lang {
                Language::TraditionalChinese => "連線已成功恢復",
                Language::SimplifiedChinese => "连接已成功恢复",
                Language::English => "Connection Restored",
                Language::French => "Connexion rétablie",
                Language::German => "Verbindung wiederhergestellt",
                Language::Italian => "Connessione ripristinata",
                Language::Japanese => "接続が正常に復旧しました",
                Language::Korean => "연결이 성공적으로 복구되었습니다",
                Language::PortugueseBrazil => "Conexão restabelecida",
                Language::PortuguesePortugal => "Ligação restabelecida",
                Language::Indonesian => "Koneksi Berhasil Dipulihkan",
                Language::Vietnamese => "Đã khôi phục kết nối thành công",
                Language::Russian => "Связь успешно восстановлена",
                Language::Spanish => "Conexión restablecida con éxito",
            },
            Self::Preempted(_) | Self::ResumePreempted(_) => lang.preempted_banner_title(),
            Self::ConnectionLostReconnecting(_) | Self::ResumeReconnecting(_) => match lang {
                Language::TraditionalChinese => "正在重新連線",
                Language::SimplifiedChinese => "正在重新连接",
                Language::English => "Reconnecting...",
                Language::French => "Reconnexion en cours...",
                Language::German => "Wiederverbindung...",
                Language::Italian => "Riconnessione in corso...",
                Language::Japanese => "再接続中...",
                Language::Korean => "다시 연결 중...",
                Language::PortugueseBrazil => "Reconectando...",
                Language::PortuguesePortugal => "A religar...",
                Language::Indonesian => "Menghubungkan kembali...",
                Language::Vietnamese => "Đang kết nối lại...",
                Language::Russian => "Переподключение...",
                Language::Spanish => "Reconectando...",
            },
            Self::Disconnected(_) | Self::ResumeFailed(_) => match lang {
                Language::TraditionalChinese => "音響連線中斷",
                Language::SimplifiedChinese => "音响连接中断",
                Language::English => "Connection Lost",
                Language::French => "Connexion perdue",
                Language::German => "Verbindung verloren",
                Language::Italian => "Connessione persa",
                Language::Japanese => "接続が切断されました",
                Language::Korean => "연결이 끊어졌습니다",
                Language::PortugueseBrazil => "Conexão perdida",
                Language::PortuguesePortugal => "Ligação perdida",
                Language::Indonesian => "Koneksi Terputus",
                Language::Vietnamese => "Mất kết nối",
                Language::Russian => "Связь потеряна",
                Language::Spanish => "Conexión perdida",
            },
            Self::AirPodsOccupied | Self::AirPodsOccupiedFallbackHomePod => match lang {
                Language::TraditionalChinese => "AirPods 狀態提示",
                Language::SimplifiedChinese => "AirPods 状态提示",
                Language::English => "AirPods Notice",
                Language::French => "Notification AirPods",
                Language::German => "AirPods Hinweis",
                Language::Italian => "Avviso AirPods",
                Language::Japanese => "AirPods のお知らせ",
                Language::Korean => "AirPods 알림",
                Language::PortugueseBrazil => "Aviso do AirPods",
                Language::PortuguesePortugal => "Aviso dos AirPods",
                Language::Indonesian => "Pemberitahuan AirPods",
                Language::Vietnamese => "Thông báo AirPods",
                Language::Russian => "Уведомление AirPods",
                Language::Spanish => "Aviso de AirPods",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_tooltips_plan_a() {
        let lang = Language::TraditionalChinese;
        let s = lang.tray_streaming_tooltip(false, "書房", 9);
        assert_eq!(s, "🔊 串流中：書房 (9%)");

        let s_muted = lang.tray_streaming_tooltip(true, "書房", 0);
        assert_eq!(s_muted, "🔇 串流中：書房 (靜音)");

        let ap = lang.tray_airpods_with_battery("AirPods 3", "L 100%");
        assert_eq!(ap, "🎧 AirPods 3：L 100%");

        let ap_status = lang.tray_airpods_status_only("AirPods 3", lang.airpods_connected_status_tag());
        assert_eq!(ap_status, "🎧 AirPods 3 (已連線)");
    }

    #[test]
    fn test_phase1_languages() {
        // Portuguese (Portugal)
        assert_eq!(Language::from_code("pt-pt"), Some(Language::PortuguesePortugal));
        assert_eq!(Language::PortuguesePortugal.code(), "pt-PT");
        assert_eq!(Language::PortuguesePortugal.btn_refresh(), " ⟳ Atualizar ");
        assert_eq!(Language::PortuguesePortugal.settings_section_audio(), "Fonte de áudio e saída");
        assert_eq!(Language::PortuguesePortugal.settings_tab_receiver(), "Recetor de áudio");
        assert_eq!(Language::PortuguesePortugal.vol_speaker_muted("HomePod"), "🔇 HomePod sem som");
        assert_eq!(Language::PortuguesePortugal.vol_speaker_playing("HomePod"), "🔊 HomePod em reprodução");
        assert_eq!(Language::PortuguesePortugal.balance_left(25), "◄ E 25%");
        assert_eq!(Language::PortuguesePortugal.balance_right(25), "25% D ►");

        // Indonesian
        assert_eq!(Language::from_code("id"), Some(Language::Indonesian));
        assert_eq!(Language::from_code("id-id"), Some(Language::Indonesian));
        assert_eq!(Language::from_code("in"), Some(Language::Indonesian));
        assert_eq!(Language::Indonesian.code(), "id-ID");
        assert_eq!(Language::Indonesian.btn_refresh(), " ⟳ Segarkan ");
        assert_eq!(Language::Indonesian.btn_back(), "⬅ Kembali");
        assert_eq!(Language::Indonesian.settings_section_audio(), "Sumber & Output Audio");
        assert_eq!(Language::Indonesian.vol_speaker_muted("HomePod"), "🔇 HomePod dibisukan");
        assert_eq!(Language::Indonesian.vol_speaker_playing("HomePod"), "🔊 HomePod sedang memutar");
        assert_eq!(Language::Indonesian.balance_center(), "◄ Tengah ►");

        // Vietnamese
        assert_eq!(Language::from_code("vi"), Some(Language::Vietnamese));
        assert_eq!(Language::from_code("vi-vn"), Some(Language::Vietnamese));
        assert_eq!(Language::Vietnamese.code(), "vi-VN");
        assert_eq!(Language::Vietnamese.btn_refresh(), " ⟳ Làm mới ");
        assert_eq!(Language::Vietnamese.btn_back(), "⬅ Quay lại");
        assert_eq!(Language::Vietnamese.settings_section_audio(), "Nguồn & Đầu ra âm thanh");
        assert_eq!(Language::Vietnamese.vol_speaker_muted("HomePod"), "🔇 HomePod đã tắt tiếng");
        assert_eq!(Language::Vietnamese.vol_speaker_playing("HomePod"), "🔊 HomePod đang phát");
        assert_eq!(Language::Vietnamese.balance_center(), "◄ Giữa ►");
    }

    #[test]
    fn test_receiver_forwarding_and_dual_eq_i18n() {
        for lang in ALL_LANGUAGES {
            assert!(!lang.airplay_receiver_output_device_label().is_empty());
            assert!(!lang.airplay_receiver_output_device_hint().is_empty());
            assert!(!lang.airplay_receiver_apply_effects_label().is_empty());
            assert!(!lang.airplay_receiver_btn_adjust_eq().is_empty());
            assert!(!lang.eq_profile_label().is_empty());
            assert!(!lang.eq_profile_normal().is_empty());
            assert!(!lang.eq_profile_receiver().is_empty());
            assert!(!lang.eq_profile_receiver_hint().is_empty());
        }
        assert_eq!(Language::TraditionalChinese.eq_profile_normal(), "🖥️ 一般播放 / HomePod");
        assert_eq!(Language::TraditionalChinese.eq_profile_receiver(), "📲 音訊接收轉發");
        assert_eq!(Language::English.eq_profile_normal(), "🖥️ Playback / HomePod");
        assert_eq!(Language::English.eq_profile_receiver(), "📲 Receiver Forwarding");
    }

    #[test]
    fn test_equalizer_window_i18n() {
        for lang in ALL_LANGUAGES {
            assert!(!lang.btn_popout_equalizer().is_empty());
            assert!(!lang.btn_dock_equalizer().is_empty());
            assert!(!lang.eq_window_title().is_empty());
            assert!(!lang.mini_equalizer_window_title().is_empty());
            assert!(!lang.setting_detach_equalizer_window().is_empty());
            assert!(!lang.btn_open_mini_player().is_empty());
            assert!(!lang.btn_open_equalizer().is_empty());
            assert!(!lang.mini_player_window_title().is_empty());
            assert!(!lang.tooltip_open_mini_player().is_empty());
            assert!(!lang.tooltip_open_equalizer().is_empty());
            assert!(!lang.mini_player_title().is_empty());
            assert!(!lang.mini_equalizer_title().is_empty());
            assert!(!lang.tooltip_show_main_window().is_empty());
            assert!(!lang.tooltip_popout_mini_player().is_empty());
            assert!(!lang.tooltip_popout_open().is_empty());
            assert!(!lang.tooltip_popout_close().is_empty());
            assert!(!lang.tooltip_modular_workstation().is_empty());
            assert!(!lang.onboarding_workstation_title().is_empty());
            assert!(!lang.onboarding_workstation_desc().is_empty());
            assert!(!lang.onboarding_workstation_btn_try().is_empty());
            assert!(!lang.onboarding_workstation_btn_dismiss().is_empty());
            assert!(!lang.tooltip_main_window_docked_focus().is_empty());
            assert!(!lang.eq_profile_normal_hint().is_empty());
            assert!(!lang.pkg_muelle_desc().is_empty());
        }
        assert_eq!(Language::TraditionalChinese.btn_popout_equalizer(), "⧉ 獨立視窗");
        assert_eq!(Language::TraditionalChinese.btn_dock_equalizer(), "⤓ 嵌入主視窗");
        assert_eq!(Language::TraditionalChinese.btn_open_mini_player(), "♫ 迷你播放器");
        assert_eq!(Language::TraditionalChinese.btn_open_equalizer(), "⦀ 等化器");
        assert_eq!(Language::TraditionalChinese.mini_player_title(), "迷你播放器");
        assert_eq!(Language::TraditionalChinese.mini_equalizer_title(), "迷你等化器");
        assert_eq!(Language::English.btn_popout_equalizer(), "⧉ Pop out window");
        assert_eq!(Language::English.btn_dock_equalizer(), "⤓ Embed in main window");
        assert_eq!(Language::English.btn_open_mini_player(), "♫ Mini Player");
        assert_eq!(Language::English.btn_open_equalizer(), "⦀ Equalizer");
        assert_eq!(Language::English.mini_player_title(), "Mini Player");
        assert_eq!(Language::English.mini_equalizer_title(), "Mini Equalizer");
    }
}
