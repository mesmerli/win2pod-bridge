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
            "pt-br" | "pt" => Some(Self::PortugueseBrazil),
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
            0x16 => Self::PortugueseBrazil,
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
            Self::Russian => "⬅ Назад",
            Self::Spanish => "⬅ Volver",
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
            Self::Russian => "✕ Закрытие сворачивает в системный трей",
            Self::Spanish => "✕ Al cerrar se minimiza a la bandeja del sistema",
        }
    }

    pub fn btn_quit_app(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🚪 結束程式",
            Self::SimplifiedChinese => "🚪 退出程序",
            Self::English => "🚪 Exit App",
            Self::French => "🚪 Quitter",
            Self::German => "🚪 Beenden",
            Self::Italian => "🚪 Esci",
            Self::Japanese => "🚪 終了",
            Self::Korean => "🚪 앱 종료",
            Self::PortugueseBrazil => "🚪 Sair do app",
            Self::Russian => "🚪 Выйти",
            Self::Spanish => "🚪 Salir de la app",
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
            Self::Russian => "Поиск устройств AirPlay в локальной сети",
            Self::Spanish => "Buscar dispositivos AirPlay en la red local",
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
            Self::Russian => "Подключение колонки перехвачено",
            Self::Spanish => "Conexión del altavoz transferida",
        }
    }

    pub fn available_devices_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "可用的音響裝置",
            Self::SimplifiedChinese => "可用的音响设备",
            Self::English => "Available Audio Devices",
            Self::French => "Appareils audio disponibles",
            Self::German => "Verfügbare Audiogeräte",
            Self::Italian => "Dispositivi audio disponibili",
            Self::Japanese => "利用可能なオーディオ機器",
            Self::Korean => "사용 가능한 오디오 기기",
            Self::PortugueseBrazil => "Dispositivos de áudio disponíveis",
            Self::Russian => "Доступные аудиоустройства",
            Self::Spanish => "Dispositivos de audio disponibles",
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
            Self::Russian => "🔊 Ожидание (Ошибка соединения)",
            Self::Spanish => "🔊 En espera (Error de conexión)",
        }
    }

    pub fn btn_unmute(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔊 取消靜音",
            Self::SimplifiedChinese => "🔊 取消静音",
            Self::English => "🔊 Unmute",
            Self::French => "🔊 Rétablir le son",
            Self::German => "🔊 Laut stellen",
            Self::Italian => "🔊 Riattiva audio",
            Self::Japanese => "🔊 ミュート解除",
            Self::Korean => "🔊 음소거 해제",
            Self::PortugueseBrazil => "🔊 Desativar mudo",
            Self::Russian => "🔊 Включить звук",
            Self::Spanish => "🔊 Reactivar sonido",
        }
    }

    pub fn btn_mute(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🔇 一鍵靜音",
            Self::SimplifiedChinese => "🔇 一键静音",
            Self::English => "🔇 Mute",
            Self::French => "🔇 Couper le son",
            Self::German => "🔇 Stummschalten",
            Self::Italian => "🔇 Disattiva audio",
            Self::Japanese => "🔇 ミュート",
            Self::Korean => "🔇 음소거",
            Self::PortugueseBrazil => "🔇 Silenciar",
            Self::Russian => "🔇 Заглушить",
            Self::Spanish => "🔇 Silenciar",
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
            Self::Russian => "Расширенные настройки",
            Self::Spanish => "Configuración avanzada",
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
            Self::Russian => "Источник звука:",
            Self::Spanish => "Fuente de audio:",
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
            Self::Russian => "Все системные звуки (По умолчанию)",
            Self::Spanish => "Audio del sistema (Dispositivo predeterminado)",
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
            Self::Russian => "⚙ Открыть микшер громкости Windows",
            Self::Spanish => "⚙ Abrir Mezclador de volumen de Windows",
        }
    }

    pub fn auto_mute_speakers_checkbox(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線時自動關閉電腦喇叭 (避免重音/雙重發聲)",
            Self::SimplifiedChinese => "连接时自动关闭电脑喇叭 (避免重音/双重发声)",
            Self::English => "Mute PC speakers during playback (prevents duplicate audio / echo)",
            Self::French => "Couper les haut-parleurs du PC lors de la lecture (évite l'écho)",
            Self::German => "PC-Lautsprecher bei Wiedergabe stummschalten (verhindert Echo)",
            Self::Italian => "Disattiva altoparlanti PC durante la riproduzione (evita eco)",
            Self::Japanese => "接続時にPCスピーカーを自動ミュート (二重再生・エコー防止)",
            Self::Korean => "재생 시 PC 스피커 자동 음소거 (이중 소리 / 에코 방지)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC durante reprodução (evita eco)",
            Self::Russian => "Отключать динамики ПК при воспроизведении (без эха)",
            Self::Spanish => "Silenciar altavoces del PC al reproducir (evita eco)",
        }
    }

    pub fn auto_mute_disabled_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線時自動關閉電腦喇叭 (分流模式下主喇叭保持開啟)",
            Self::SimplifiedChinese => "连接时自动关闭电脑喇叭 (分流模式下主喇叭保持开启)",
            Self::English => "Mute PC speakers during playback (Disabled in split mode)",
            Self::French => "Couper les haut-parleurs du PC (Désactivé en mode séparé)",
            Self::German => "PC-Lautsprecher stummschalten (Inaktiv im Split-Modus)",
            Self::Italian => "Disattiva altoparlanti PC (Disabilitato in modalità separata)",
            Self::Japanese => "PCスピーカー自動ミュート (個別ルーティング時は無効)",
            Self::Korean => "재생 시 PC 스피커 음소거 (개별 라우팅 모드에서는 비활성화)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC (Desativado no modo separado)",
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
            Self::Russian => "Раздельный вывод активен; динамики ПК продолжают воспроизводить системные звуки.",
            Self::Spanish => "Enrutamiento específico activo; los altavoces principales siguen activos.",
        }
    }

    pub fn auto_mute_locked_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "連線時自動關閉電腦喇叭 🔒 (試用期已結束)",
            Self::SimplifiedChinese => "连接时自动关闭电脑喇叭 🔒 (试用期已结束)",
            Self::English => "Auto-mute PC speakers on connect 🔒 (Trial Expired)",
            Self::French => "Couper les haut-parleurs PC à la connexion 🔒 (Essai expiré)",
            Self::German => "PC-Lautsprecher bei Verbindung stummschalten 🔒 (Testversion abgelaufen)",
            Self::Italian => "Disattiva altoparlanti PC alla connessione 🔒 (Prova scaduta)",
            Self::Japanese => "接続時にPCスピーカーを自動ミュート 🔒 (試用期間終了)",
            Self::Korean => "연결 시 PC 스피커 자동 음소거 🔒 (체험 기간 만료)",
            Self::PortugueseBrazil => "Silenciar alto-falantes do PC ao conectar 🔒 (Avaliação expirada)",
            Self::Russian => "Автозаглушение динамиков ПК при подключении 🔒 (Пробный период истек)",
            Self::Spanish => "Silenciar altavoces de PC al conectar 🔒 (Prueba expirada)",
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
            Self::Russian => "Пробный период истек. Функция заблокирована. Приобретите продукт для разблокировки.",
            Self::Spanish => "Período de prueba expirado. Función bloqueada. Compre el producto para desbloquear.",
        }
    }

    pub fn btn_buy_product(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🛍️ 前往微軟商店購買產品",
            Self::SimplifiedChinese => "🛍️ 前往微软商店购买产品",
            Self::English => "🛍️ Buy Product in Microsoft Store",
            Self::French => "🛍️ Acheter sur le Microsoft Store",
            Self::German => "🛍️ Im Microsoft Store kaufen",
            Self::Italian => "🛍️ Acquista su Microsoft Store",
            Self::Japanese => "🛍️ Microsoft Storeで製品を購入",
            Self::Korean => "🛍️ Microsoft Store에서 제품 구매",
            Self::PortugueseBrazil => "🛍️ Comprar na Microsoft Store",
            Self::Russian => "🛍️ Купить в Microsoft Store",
            Self::Spanish => "🛍️ Comprar en Microsoft Store",
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
            Self::Russian => "Автономный исполняемый файл (Unpackaged)",
            Self::Spanish => "Ejecutable independiente (Desempaquetado)",
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
            Self::Russian => "Вкл: Запрещает сон при трансляции; музыка играет при погасшем экране.\nВыкл: Обычный спящий режим по настройкам Windows.",
            Self::Spanish => "Activado: Evita la suspensión al reproducir; la música continúa con la pantalla apagada.\nDesactivado: Suspende según el plan de energía.",
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
            Self::Russian => "⚪ Нет звука более 30 сек; блокировка сна снята.",
            Self::Spanish => "⚪ Sin señal de audio durante 30s; bloqueo de suspensión liberado.",
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
            Self::Russian => "💡 Блокирует сон при воспроизведении; разрешает при отсутствии звука.",
            Self::Spanish => "💡 Bloquea la suspensión al reproducir; la permite cuando no hay audio.",
        }
    }

    pub fn hint_sleep_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⚠️ 電腦閒置達到系統設定時間後將自動休眠並暫停串流。",
            Self::SimplifiedChinese => "⚠️ 电脑闲置达到系统设定时间后将自动休眠并暂停串流。",
            Self::English => "⚠️ PC will sleep after idle timeout, pausing audio stream.",
            Self::French => "⚠️ Le PC se mettra en veille après expiration du délai, suspendant la lecture.",
            Self::German => "⚠️ PC wechselt nach Inaktivität in den Ruhezustand und pausiert das Streaming.",
            Self::Italian => "⚠️ Il PC andrà in sospensione per inattività, interrompendo lo streaming.",
            Self::Japanese => "⚠️ アイドル時間が設定に達するとPCがスリープし、配信が中断されます。",
            Self::Korean => "⚠️ 유휴 시간 초과 시 PC가 절전 모드로 전환되어 스트리밍이 일시 정지됩니다.",
            Self::PortugueseBrazil => "⚠️ O PC suspenderá após tempo ocioso, pausando a transmissão.",
            Self::Russian => "⚠️ ПК перейдет в спящий режим по таймеру, прервав трансляцию.",
            Self::Spanish => "⚠️ El PC se suspenderá tras el tiempo de inactividad, pausando el streaming.",
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
            Self::Russian => "💡 Буфер минимален; при щелчках звука переключитесь в обычный режим.",
            Self::Spanish => "💡 Búfer minimizado; cambia al modo estándar si se producen chasquidos de audio.",
        }
    }

    pub fn hint_low_latency_off(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "🛡️ 高抗干擾模式；適合純音樂聆聽或 WiFi 距離路由器較遠時使用。",
            Self::SimplifiedChinese => "🛡️ 高抗干扰模式；适合纯音乐聆听或 WiFi 距离路由器较远时使用。",
            Self::English => "🛡️ High stability mode; recommended for music listening or distant Wi-Fi.",
            Self::French => "🛡️ Mode haute stabilité ; idéal pour la musique ou Wi-Fi éloigné.",
            Self::German => "🛡️ Hohe Stabilität; empfohlen für reinen Musikgenuss oder schwaches Wi-Fi.",
            Self::Italian => "🛡️ Massima stabilità; consigliato per l'ascolto musicale o Wi-Fi distante.",
            Self::Japanese => "🛡️ 耐干渉モード：音楽鑑賞やルーターから離れた場所での使用に最適。",
            Self::Korean => "🛡️ 높은 안정성 모드: 음악 감상 또는 신호가 약한 Wi-Fi 환경에 적합.",
            Self::PortugueseBrazil => "🛡️ Modo de alta estabilidade; recomendado para música ou Wi-Fi distante.",
            Self::Russian => "🛡️ Режим повышенной стабильности; подходит для музыки и дальнего Wi-Fi.",
            Self::Spanish => "🛡️ Modo de alta estabilidad; recomendado para música o Wi-Fi distante.",
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
            Self::Russian => "🚀 Автоподключение к последней колонке при запуске",
            Self::Spanish => "🚀 Conectar automáticamente al iniciar",
        }
    }

    pub fn auto_connect_tooltip(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟：程式啟動時自動搜尋並連線至上次播放的 HomePod 或立體聲組。\n關閉：啟動後維持閒置狀態，等待手動點選音響。",
            Self::SimplifiedChinese => "开启：程序启动时自动搜索并连接至上次播放的 HomePod 或立体声组。\n关闭：启动后维持闲置状态，等待手动点选音响。",
            Self::English => "On: Automatically finds and connects to last speaker on startup.\nOff: Starts in idle state.",
            Self::French => "Activé : Se connecte automatiquement au dernier HomePod au lancement.\nDésactivé : Reste inactif au démarrage.",
            Self::German => "Ein: Verbindet sich beim Start automatisch mit dem letzten HomePod.\nAus: Startet im Leerlauf.",
            Self::Italian => "Attivo: Cerca e connette automaticamente l'ultimo HomePod all'avvio.\nDisattivo: Rimane inattivo all'avvio.",
            Self::Japanese => "ON：起動時に前回のHomePodまたはステレオペアに自動接続。\nOFF：待機状態で起動し手動選択を待ちます。",
            Self::Korean => "켜짐: 프로그램 시작 시 마지막 HomePod 또는 스테레오 페어에 자동 연결.\n꺼짐: 대기 상태로 시작하여 수동 연결 대기.",
            Self::PortugueseBrazil => "Ligado: Conecta automaticamente ao último HomePod na inicialização.\nDesligado: Inicia em espera.",
            Self::Russian => "Вкл: Автоматический поиск и подключение к последнему HomePod при старте.\nВыкл: Запуск в режиме ожидания.",
            Self::Spanish => "Activado: Se conecta automáticamente al último HomePod al iniciar.\nDesactivado: Inicia en espera.",
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
            Self::Russian => "⚪ Автозапуск отключен; требуется ручной запуск.",
            Self::Spanish => "⚪ No se inicia al arrancar; inicio manual requerido.",
        }
    }

    pub fn global_hotkey_label(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "⌨️ 全域面板快捷鍵：",
            Self::SimplifiedChinese => "⌨️ 全局面板热键：",
            Self::English => "⌨️ Global Panel Hotkey:",
            Self::French => "⌨️ Raccourci global du panneau :",
            Self::German => "⌨️ Globaler Panel-Hotkey:",
            Self::Italian => "⌨️ Scorciatoia globale pannello:",
            Self::Japanese => "⌨️ パネル表示グローバルショートカット：",
            Self::Korean => "⌨️ 전역 패널 단축키:",
            Self::PortugueseBrazil => "⌨️ Atalho global do painel:",
            Self::Russian => "⌨️ Глобальная горячая клавиша:",
            Self::Spanish => "⌨️ Atajo global del panel:",
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
            Self::Russian => "🌐 Язык интерфейса:",
            Self::Spanish => "🌐 Idioma de la interfaz:",
        }
    }

    pub fn tooltip_about(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "軟體資訊與開源宣告",
            Self::SimplifiedChinese => "软件信息与开源协议",
            Self::English => "About & Open Source Info",
            Self::French => "À propos et licences open-source",
            Self::German => "Über Win2Pod & Open-Source-Info",
            Self::Italian => "Informazioni e licenze open source",
            Self::Japanese => "バージョン情報とオープンソース情報",
            Self::Korean => "소프트웨어 정보 및 오픈 소스 라이선스",
            Self::PortugueseBrazil => "Sobre e informações de código aberto",
            Self::Russian => "О программе и открытых компонентах",
            Self::Spanish => "Acerca de e información de código abierto",
        }
    }

    pub fn btn_about(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "關於軟體與開源套件",
            Self::SimplifiedChinese => "关于软件与开源套件",
            Self::English => "About & Open Source",
            Self::French => "À propos et composants open-source",
            Self::German => "Über & Open-Source-Pakete",
            Self::Italian => "Informazioni e pacchetti open source",
            Self::Japanese => "バージョン情報とオープンソース",
            Self::Korean => "소프트웨어 정보 및 오픈 소스",
            Self::PortugueseBrazil => "Sobre e pacotes de código aberto",
            Self::Russian => "О программе и компонентах",
            Self::Spanish => "Acerca de y paquetes de código abierto",
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
            Self::Russian => "Автор:",
            Self::Spanish => "Autor:",
        }
    }

    pub fn about_author_value(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "mesmerli & 開源社群貢獻者",
            Self::SimplifiedChinese => "mesmerli & 开源社区贡献者",
            Self::English => "mesmerli & Open Source Contributors",
            Self::French => "mesmerli & Contributeurs open-source",
            Self::German => "mesmerli & Open-Source-Mitwirkende",
            Self::Italian => "mesmerli & Collaboratori open source",
            Self::Japanese => "mesmerli & オープンソース コントリビューター",
            Self::Korean => "mesmerli & 오픈 소스 기여자",
            Self::PortugueseBrazil => "mesmerli & Contribuidores de código aberto",
            Self::Russian => "mesmerli & разработчики открытого ПО",
            Self::Spanish => "mesmerli & Colaboradores de código abierto",
        }
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
            Self::Russian => "Лицензия:",
            Self::Spanish => "Licencia:",
        }
    }

    pub fn about_copyright_notice(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "版權宣告：版權所有 © 2024-2026 Win2Pod 貢獻者。本軟體為自由軟體，依據 GNU 通用公眾授權條款第二版 (GPL-2.0) 發布。",
            Self::SimplifiedChinese => "版权宣告：版权所有 © 2024-2026 Win2Pod 贡献者。本软件为自由软件，依据 GNU 通用公共许可证第二版 (GPL-2.0) 发布。",
            Self::English => "Copyright Notice: Copyright © 2024-2026 Win2Pod Contributors. Free software released under the GNU General Public License v2.0 (GPL-2.0).",
            Self::French => "Mentions de droits d'auteur : Copyright © 2024-2026 Contributeurs Win2Pod. Logiciel libre publié sous licence GNU General Public License v2.0 (GPL-2.0).",
            Self::German => "Urheberrechtshinweis: Copyright © 2024-2026 Win2Pod-Mitwirkende. Freie Software, veröffentlicht unter der GNU General Public License v2.0 (GPL-2.0).",
            Self::Italian => "Avviso di copyright: Copyright © 2024-2026 Collaboratori Win2Pod. Software libero distribuito sotto GNU General Public License v2.0 (GPL-2.0).",
            Self::Japanese => "著作権表記：Copyright © 2024-2026 Win2Pod 開発者一同。本ソフトウェアは GNU General Public License v2.0 (GPL-2.0) の下で配布されるフリーソフトウェアです。",
            Self::Korean => "저작권 고지: Copyright © 2024-2026 Win2Pod 기여자. 본 소프트웨어는 GNU General Public License v2.0 (GPL-2.0)에 따라 배포되는 자유 소프트웨어입니다.",
            Self::PortugueseBrazil => "Aviso de direitos autorais: Copyright © 2024-2026 Contribuidores do Win2Pod. Software livre distribuído sob a GNU General Public License v2.0 (GPL-2.0).",
            Self::Russian => "Уведомление об авторских правах: Copyright © 2024-2026 Разработчики Win2Pod. Свободное ПО, распространяемое под лицензией GNU General Public License v2.0 (GPL-2.0).",
            Self::Spanish => "Aviso de copyright: Copyright © 2024-2026 Colaboradores de Win2Pod. Software libre distribuido bajo la GNU General Public License v2.0 (GPL-2.0).",
        }
    }

    pub fn about_packages_header(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "內建開源組件",
            Self::SimplifiedChinese => "内置开源组件",
            Self::English => "Built-in Open Source Components",
            Self::French => "Composants open source intégrés",
            Self::German => "Integrierte Open-Source-Komponenten",
            Self::Italian => "Componenti open source integrati",
            Self::Japanese => "内蔵オープンソースコンポーネント",
            Self::Korean => "내장 오픈 소스 컴포넌트",
            Self::PortugueseBrazil => "Componentes de código aberto integrados",
            Self::Russian => "Встроенные компоненты с открытым исходным кодом",
            Self::Spanish => "Componentes de código abierto integrados",
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
            Self::Russian => "Библиотека декодирования и обработки изображений",
            Self::Spanish => "Biblioteca de decodificación y procesamiento de imágenes",
        }
    }

    // -------------------------------------------------------------
    // Tray & Notification Strings
    // -------------------------------------------------------------

    pub fn tray_open_panel(&self) -> &'static str {
        match self {
            Self::TraditionalChinese => "開啟控制面板",
            Self::SimplifiedChinese => "打开控制面板",
            Self::English => "Open Control Panel",
            Self::French => "Ouvrir le panneau de contrôle",
            Self::German => "Bedienfeld öffnen",
            Self::Italian => "Apri pannello di controllo",
            Self::Japanese => "コントロールパネルを開く",
            Self::Korean => "제어판 열기",
            Self::PortugueseBrazil => "Abrir painel de controle",
            Self::Russian => "Открыть панель управления",
            Self::Spanish => "Abrir panel de control",
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
            Self::Russian => "✓ Режим звука (Звук отключен)",
            Self::Spanish => "✓ Modo silencio (Silenciado)",
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
            Self::Russian => "Win2Pod Bridge - Ожидание",
            Self::Spanish => "Win2Pod Bridge - En espera",
        }
    }

    pub fn tray_connecting_tooltip(&self, name: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("Win2Pod Bridge - 連線中 ({name})"),
            Self::SimplifiedChinese => format!("Win2Pod Bridge - 连接中 ({name})"),
            Self::English => format!("Win2Pod Bridge - Connecting ({name})"),
            Self::French => format!("Win2Pod Bridge - Connexion ({name})"),
            Self::German => format!("Win2Pod Bridge - Verbindung läuft ({name})"),
            Self::Italian => format!("Win2Pod Bridge - Connessione ({name})"),
            Self::Japanese => format!("Win2Pod Bridge - 接続中 ({name})"),
            Self::Korean => format!("Win2Pod Bridge - 연결 중 ({name})"),
            Self::PortugueseBrazil => format!("Win2Pod Bridge - Conectando ({name})"),
            Self::Russian => format!("Win2Pod Bridge - Подключение ({name})"),
            Self::Spanish => format!("Win2Pod Bridge - Conectando ({name})"),
        }
    }

    pub fn tray_streaming_tooltip(&self, name: &str, vol_suffix: &str) -> String {
        match self {
            Self::TraditionalChinese => format!("Win2Pod Bridge - 現正串流 ({name}){vol_suffix}"),
            Self::SimplifiedChinese => format!("Win2Pod Bridge - 正在串流 ({name}){vol_suffix}"),
            Self::English => format!("Win2Pod Bridge - Streaming ({name}){vol_suffix}"),
            Self::French => format!("Win2Pod Bridge - Lecture en cours ({name}){vol_suffix}"),
            Self::German => format!("Win2Pod Bridge - Streaming ({name}){vol_suffix}"),
            Self::Italian => format!("Win2Pod Bridge - In riproduzione ({name}){vol_suffix}"),
            Self::Japanese => format!("Win2Pod Bridge - 再生中 ({name}){vol_suffix}"),
            Self::Korean => format!("Win2Pod Bridge - 스트리밍 중 ({name}){vol_suffix}"),
            Self::PortugueseBrazil => format!("Win2Pod Bridge - Transmitindo ({name}){vol_suffix}"),
            Self::Russian => format!("Win2Pod Bridge - Трансляция ({name}){vol_suffix}"),
            Self::Spanish => format!("Win2Pod Bridge - Reproduciendo ({name}){vol_suffix}"),
        }
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
            Self::Russian => format!("Win2Pod Bridge - Ошибка: {err}"),
            Self::Spanish => format!("Win2Pod Bridge - Error: {err}"),
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
            Self::Russian => "Win2Pod Bridge - Изменение состояния звука",
            Self::Spanish => "Win2Pod Bridge - Cambio de estado de audio",
        }
    }
}

/// Notice message kind for structured, localizable notifications and banners.
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
                Language::Russian => format!("Не удалось переподключиться к \"{name}\" после сна, звук отключен."),
                Language::Spanish => format!("No se pudo reconectar a \"{name}\" tras reactivar, silenciado."),
            },
        }
    }
}
