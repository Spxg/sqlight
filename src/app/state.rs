use std::collections::HashSet;

use aceditor::Editor;
use js_sys::Uint8Array;
use leptos::tachys::dom::window;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use web_sys::MediaQueryList;

use crate::{FragileComfirmed, SQLightError, SQLiteStatementResult, WorkerHandle};

const DEFAULT_CODE: &str = "PRAGMA page_size=4096;

CREATE TABLE IF NOT EXISTS blobs (
    id INTEGER PRIMARY KEY,
    data BLOB
);

INSERT INTO blobs(data) VALUES (randomblob(12));

SELECT 'Hello World!',
        datetime('now','localtime') AS TM,
        x'73716c69676874' AS BLOB_VAL,
        NULL as NULL_VAL;

SELECT * FROM blobs;";

#[derive(Store, Serialize, Deserialize)]
pub struct GlobalState {
    vfs: Vfs,
    editor_config: EditorConfig,
    orientation: Orientation,
    theme: Theme,
    keep_ctx: bool,
    sql: String,
    run_selected_sql: bool,
    // runtime state below
    #[serde(skip)]
    worker: Option<WorkerHandle>,
    #[serde(skip)]
    editor: Option<Editor>,
    #[serde(skip)]
    focus: Option<Focus>,
    #[serde(skip)]
    is_focused: bool,
    #[serde(skip)]
    opened_focus: HashSet<Focus>,
    #[serde(skip)]
    share_href: Option<String>,
    #[serde(skip)]
    share_sql_with_result: Option<String>,
    #[serde(skip)]
    show_something: bool,
    #[serde(skip)]
    output: Vec<SQLiteStatementResult>,
    #[serde(skip)]
    last_error: Option<FragileComfirmed<SQLightError>>,
    #[serde(skip)]
    import_progress: Option<ImportProgress>,
    #[serde(skip)]
    exported: Option<Exported>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            vfs: Vfs::Memory,
            editor_config: EditorConfig::default(),
            orientation: Orientation::Automatic,
            theme: Theme::System,
            keep_ctx: false,
            sql: DEFAULT_CODE.into(),
            run_selected_sql: false,
            worker: None,
            editor: None,
            focus: None,
            is_focused: false,
            opened_focus: HashSet::new(),
            share_href: None,
            share_sql_with_result: None,
            show_something: false,
            output: Vec::new(),
            last_error: None,
            import_progress: None,
            exported: None,
        }
    }
}

impl GlobalState {
    pub fn load() -> Option<Self> {
        let storage = window().local_storage().ok()??;
        let value = storage.get("config").ok()??;
        serde_json::from_str(&value).ok()
    }

    pub fn save(&self) {
        if let Some(Err(e)) = window()
            .local_storage()
            .ok()
            .flatten()
            .map(|s| s.set_item("config", &serde_json::to_string(self).unwrap()))
        {
            log::error!("Faild to save config to localstorage: {e:?}");
        }
    }
}

pub struct ImportProgress {
    pub filename: String,
    pub loaded: f64,
    pub total: f64,
    pub opened: Option<bool>,
}

pub struct Exported {
    pub filename: String,
    pub data: FragileComfirmed<Uint8Array>,
}

#[derive(Serialize, Deserialize)]
pub struct EditorConfig {
    pub keyboard: String,
    pub light_theme: String,
    pub dark_theme: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig {
            keyboard: "ace".into(),
            light_theme: "github".into(),
            dark_theme: "github_dark".into(),
        }
    }
}

impl GlobalState {
    pub fn is_focus(&self) -> bool {
        self.focus.is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vfs {
    Memory,
    OPFS,
}

impl Vfs {
    pub fn value(&self) -> String {
        match self {
            Vfs::Memory => "Memory".into(),
            Vfs::OPFS => "OPFS".into(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Focus {
    Execute,
    Share,
    Status,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Theme {
    System,
    SystemLight,
    SystemDark,
    Light,
    Dark,
}

impl Theme {
    pub fn is_system(&self) -> bool {
        matches!(self, Theme::System | Theme::SystemLight | Theme::SystemDark)
    }

    pub fn from_select(s: &str) -> Self {
        match s {
            "System" => Self::System,
            "Light" => Self::Light,
            "Dark" => Self::Dark,
            _ => unreachable!(),
        }
    }

    pub fn match_media() -> Option<MediaQueryList> {
        window()
            .match_media("(prefers-color-scheme: dark)")
            .ok()
            .flatten()
    }

    pub fn value(&self) -> Self {
        if *self == Theme::System {
            Self::match_media()
                .map(|query| {
                    if query.matches() {
                        Theme::SystemDark
                    } else {
                        Theme::SystemLight
                    }
                })
                .unwrap_or_else(|| Theme::SystemLight)
        } else {
            *self
        }
    }

    pub fn select(&self) -> String {
        match self {
            Theme::System | Theme::SystemLight | Theme::SystemDark => "System",
            Theme::Light => "Light",
            Theme::Dark => "Dark",
        }
        .into()
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Orientation {
    Automatic,
    AutoHorizontal,
    AutoVertical,
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn is_auto(&self) -> bool {
        matches!(
            self,
            Orientation::Automatic | Orientation::AutoVertical | Orientation::AutoHorizontal
        )
    }

    pub fn from_select(s: &str) -> Self {
        match s {
            "Automatic" => Self::Automatic,
            "Horizontal" => Self::Horizontal,
            "Vertical" => Self::Vertical,
            _ => unreachable!(),
        }
    }

    pub fn match_media() -> Option<MediaQueryList> {
        window().match_media("(max-width: 1600px)").ok().flatten()
    }

    pub fn value(&self) -> Self {
        if *self == Orientation::Automatic {
            Self::match_media()
                .map(|query| {
                    if query.matches() {
                        Orientation::AutoHorizontal
                    } else {
                        Orientation::AutoVertical
                    }
                })
                .unwrap_or_else(|| Orientation::AutoVertical)
        } else {
            *self
        }
    }

    pub fn select(&self) -> String {
        match self {
            Orientation::Automatic | Orientation::AutoVertical | Orientation::AutoHorizontal => {
                "Automatic"
            }
            Orientation::Horizontal => "Horizontal",
            Orientation::Vertical => "Vertical",
        }
        .into()
    }
}
