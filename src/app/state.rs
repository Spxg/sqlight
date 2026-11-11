use std::collections::HashSet;

use aceditor::Editor;
use js_sys::Uint8Array;
use leptos::tachys::dom::window;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

use crate::{SQLightError, SQLiteStatementResult, SendWrapper};

const DEFAULT_CODE: &str = "PRAGMA page_size=4096;

CREATE TABLE IF NOT EXISTS blobs (
    id INTEGER PRIMARY KEY,
    data BLOB
);

INSERT INTO blobs(data) VALUES (randomblob(12));

SELECT 'Hello World!',
        datetime('now','localtime') AS TM,
        unhex('73716c69676874') AS BLOB_VAL,
        NULL as NULL_VAL;

SELECT * FROM blobs;";

#[derive(Store, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalState {
    vfs: Vfs,
    editor_config: EditorConfig,
    orientation: Orientation,
    theme: Theme,
    keep_ctx: bool,
    sql: String,
    run_selected_sql: bool,
    multiple_ciphers: bool,
    // runtime state below
    #[serde(skip)]
    editor: Option<SendWrapper<Editor>>,
    #[serde(skip)]
    focus: Option<Focus>,
    #[serde(skip)]
    is_running: bool,
    #[serde(skip)]
    opened_focus: HashSet<Focus>,
    #[serde(skip)]
    share_href: Option<String>,
    #[serde(skip)]
    show_something: bool,
    #[serde(skip)]
    output: Vec<SQLiteStatementResult>,
    #[serde(skip)]
    embed: Vec<SQLiteStatementResult>,
    #[serde(skip)]
    last_error: Option<SendWrapper<SQLightError>>,
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
            multiple_ciphers: false,
            editor: None,
            focus: None,
            is_running: false,
            opened_focus: HashSet::new(),
            share_href: None,
            show_something: false,
            output: vec![],
            embed: vec![],
            last_error: None,
            import_progress: None,
            exported: None,
        }
    }
}

impl GlobalState {
    pub fn load() -> Option<Self> {
        let storage = window().local_storage().ok()??;
        let value = storage.get_item("config").ok()??;
        serde_json::from_str(&value).ok()
    }

    pub fn save(&self) {
        let Ok(Some(storage)) = window().local_storage() else {
            return;
        };
        let value = serde_json::to_string(self).unwrap();
        if let Err(error) = storage.set_item("config", &value) {
            log::error!("Failed to save configuration to local storage: {error:?}");
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
    pub data: SendWrapper<Uint8Array>,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
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

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Vfs {
    Memory,
    OPFS,
}

impl Vfs {
    pub fn value(&self) -> &'static str {
        match self {
            Vfs::Memory => "Memory",
            Vfs::OPFS => "OPFS",
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
    #[serde(alias = "SystemLight", alias = "SystemDark")]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn from_select(value: &str) -> Self {
        match value {
            "System" => Self::System,
            "Light" => Self::Light,
            "Dark" => Self::Dark,
            _ => unreachable!(),
        }
    }

    pub fn resolve(self, prefers_dark: bool) -> Self {
        match self {
            Self::System if prefers_dark => Self::Dark,
            Self::System => Self::Light,
            theme => theme,
        }
    }

    pub fn select(self) -> &'static str {
        match self {
            Self::System => "System",
            Self::Light => "Light",
            Self::Dark => "Dark",
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Orientation {
    #[serde(alias = "AutoHorizontal", alias = "AutoVertical")]
    Automatic,
    Horizontal,
    Vertical,
}

impl Orientation {
    pub fn from_select(value: &str) -> Self {
        match value {
            "Automatic" => Self::Automatic,
            "Horizontal" => Self::Horizontal,
            "Vertical" => Self::Vertical,
            _ => unreachable!(),
        }
    }

    pub fn resolve(self, narrow_screen: bool) -> Self {
        match self {
            Self::Automatic if narrow_screen => Self::Horizontal,
            Self::Automatic => Self::Vertical,
            orientation => orientation,
        }
    }

    pub fn select(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::Horizontal => "Horizontal",
            Self::Vertical => "Vertical",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{GlobalState, Orientation, Theme};

    #[test]
    fn missing_preferences_use_defaults() {
        let state: GlobalState =
            serde_json::from_str(r#"{"editor_config":{"keyboard":"vim"}}"#).unwrap();

        assert_eq!(state.editor_config.keyboard, "vim");
        assert_eq!(state.editor_config.light_theme, "github");
        assert!(matches!(state.theme, Theme::System));
        assert!(!state.sql.is_empty());
    }

    #[test]
    fn previous_automatic_values_preserve_user_preferences() {
        let state: GlobalState =
            serde_json::from_str(r#"{"theme":"SystemDark","orientation":"AutoHorizontal"}"#)
                .unwrap();

        assert!(matches!(state.theme, Theme::System));
        assert!(matches!(state.orientation, Orientation::Automatic));
        assert!(matches!(state.theme.resolve(false), Theme::Light));
        assert!(matches!(
            state.orientation.resolve(false),
            Orientation::Vertical
        ));
    }

    #[test]
    fn runtime_state_is_not_persisted() {
        let state = GlobalState {
            is_running: true,
            ..GlobalState::default()
        };
        let value = serde_json::to_value(state).unwrap();

        assert_eq!(value["theme"], "System");
        assert!(value.get("is_running").is_none());
        assert!(value.get("editor").is_none());
        assert!(value.get("output").is_none());
    }
}
