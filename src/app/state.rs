use std::collections::HashSet;

use aceditor::Editor;
use leptos::tachys::dom::window;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};
use web_sys::MediaQueryList;

use crate::{FragileComfirmed, SQLightError, SQLiteStatementResult, WorkerHandle};

const DEFAULT_CODE: &str = "SELECT 'Hello World!', datetime('now','localtime') AS current_time;";

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
    pub editor: Option<Editor>,
    #[serde(skip)]
    focus: Option<Focus>,
    #[serde(skip)]
    is_focused: bool,
    #[serde(skip)]
    opened_focus: HashSet<Focus>,
    #[serde(skip)]
    share_href: Option<String>,
    #[serde(skip)]
    show_something: bool,
    #[serde(skip)]
    output: Vec<SQLiteStatementResult>,
    #[serde(skip)]
    last_error: Option<FragileComfirmed<SQLightError>>,
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            editor_config: EditorConfig::default(),
            sql: DEFAULT_CODE.into(),
            focus: None,
            show_something: false,
            orientation: Orientation::Automatic,
            theme: Theme::System,
            output: Vec::new(),
            vfs: Vfs::Memory,
            keep_ctx: false,
            share_href: None,
            is_focused: false,
            opened_focus: HashSet::new(),
            worker: None,
            editor: None,
            last_error: None,
            run_selected_sql: false,
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

#[derive(Serialize, Deserialize)]
pub struct EditorConfig {
    pub keyboard: String,
    pub theme: String,
}

impl Default for EditorConfig {
    fn default() -> Self {
        EditorConfig {
            keyboard: "ace".into(),
            theme: "github".into(),
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
                .unwrap_or_else(|| Theme::SystemDark)
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
