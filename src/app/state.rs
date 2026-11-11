use std::collections::HashSet;

use aceditor::Editor;
use js_sys::Uint8Array;
use leptos::tachys::dom::window;
use reactive_stores::Store;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{FileSnapshot, SQLightError, SQLiteStatementResult, SendWrapper};

const DEFAULT_SQL: &str = include_str!("default.sql");
const NEW_WORKSPACE_SQL: &str =
    "SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name;";

#[derive(Store, Serialize, Deserialize)]
#[serde(default)]
pub struct GlobalState {
    workspace_id: String,
    workspace_name: String,
    workspaces: Vec<Workspace>,
    deleted_workspace_ids: Vec<String>,
    #[serde(default)]
    database: String,
    #[serde(default)]
    pending_database: bool,
    editor_config: EditorConfig,
    orientation: Orientation,
    theme: Theme,
    keep_ctx: bool,
    sql: String,
    run_selected_sql: bool,
    #[serde(alias = "multiple_ciphers", deserialize_with = "deserialize_engine")]
    engine: Engine,
    read_only: bool,
    // runtime state below
    #[serde(skip)]
    editor: Option<SendWrapper<Editor>>,
    #[serde(skip)]
    focus: Option<Focus>,
    #[serde(skip)]
    is_running: bool,
    #[serde(skip)]
    is_busy: bool,
    #[serde(skip)]
    databases: Vec<String>,
    #[serde(skip)]
    files: Option<FileSnapshot>,
    #[serde(skip)]
    opened_focus: HashSet<Focus>,
    #[serde(skip)]
    share_href: Option<String>,
    #[serde(skip)]
    show_something: bool,
    #[serde(skip)]
    output: Vec<SQLiteStatementResult>,
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
            workspace_id: "default".into(),
            workspace_name: "Default".into(),
            workspaces: vec![],
            deleted_workspace_ids: vec![],
            database: "default.db".into(),
            pending_database: false,
            editor_config: EditorConfig::default(),
            orientation: Orientation::Automatic,
            theme: Theme::System,
            keep_ctx: false,
            sql: DEFAULT_SQL.into(),
            run_selected_sql: false,
            engine: Engine::SQLite,
            read_only: false,
            editor: None,
            focus: None,
            is_running: false,
            is_busy: false,
            databases: vec![],
            files: None,
            opened_focus: HashSet::new(),
            share_href: None,
            show_something: false,
            output: vec![],
            last_error: None,
            import_progress: None,
            exported: None,
        }
    }
}

impl GlobalState {
    fn swap_workspace(&mut self, workspace: &mut Workspace) {
        std::mem::swap(&mut self.workspace_id, &mut workspace.id);
        std::mem::swap(&mut self.workspace_name, &mut workspace.name);
        std::mem::swap(&mut self.database, &mut workspace.database);
        std::mem::swap(&mut self.pending_database, &mut workspace.pending_database);
        std::mem::swap(&mut self.sql, &mut workspace.sql);
        std::mem::swap(&mut self.engine, &mut workspace.engine);
        std::mem::swap(&mut self.keep_ctx, &mut workspace.keep_ctx);
        std::mem::swap(&mut self.read_only, &mut workspace.read_only);
    }

    pub fn switch_workspace(&mut self, id: &str) -> bool {
        let Some(index) = self
            .workspaces
            .iter()
            .position(|workspace| workspace.id == id)
        else {
            return false;
        };
        let mut workspace = self.workspaces.remove(index);
        self.swap_workspace(&mut workspace);
        if !self.deleted_workspace_ids.contains(&workspace.id) {
            self.workspaces.insert(index, workspace);
        }
        true
    }

    pub fn create_workspace(&mut self, id: String, name: String) {
        let mut workspace = Workspace {
            database: self.database.clone(),
            pending_database: self.pending_database,
            id,
            name,
            sql: NEW_WORKSPACE_SQL.into(),
            engine: self.engine,
            keep_ctx: true,
            read_only: self.read_only,
        };
        self.swap_workspace(&mut workspace);
        if !self.deleted_workspace_ids.contains(&workspace.id) {
            self.workspaces.push(workspace);
        }
    }

    pub fn refresh_workspaces(&mut self, saved: Self) {
        for id in saved.deleted_workspace_ids {
            if !self.deleted_workspace_ids.contains(&id) {
                self.deleted_workspace_ids.push(id);
            }
        }
        let mut workspaces = saved.workspaces;
        workspaces.push(Workspace {
            id: saved.workspace_id,
            name: saved.workspace_name,
            database: saved.database,
            pending_database: saved.pending_database,
            sql: saved.sql,
            engine: saved.engine,
            keep_ctx: saved.keep_ctx,
            read_only: saved.read_only,
        });
        self.workspaces = workspaces
            .into_iter()
            .filter(|workspace| {
                workspace.id != self.workspace_id
                    && !self.deleted_workspace_ids.contains(&workspace.id)
            })
            .collect();
    }

    pub fn remove_current_workspace(&mut self) -> bool {
        if self.workspaces.is_empty() {
            return false;
        }
        if !self.deleted_workspace_ids.contains(&self.workspace_id) {
            self.deleted_workspace_ids.push(self.workspace_id.clone());
        }
        let mut workspace = self.workspaces.remove(0);
        self.swap_workspace(&mut workspace);
        true
    }

    pub fn workspace_options(&self) -> Vec<(String, String)> {
        std::iter::once((self.workspace_id.clone(), self.workspace_name.clone()))
            .chain(
                self.workspaces
                    .iter()
                    .map(|workspace| (workspace.id.clone(), workspace.name.clone())),
            )
            .collect()
    }

    pub fn database_filename(&self) -> String {
        self.database.clone()
    }

    fn current_workspace(&self) -> Workspace {
        Workspace {
            id: self.workspace_id.clone(),
            name: self.workspace_name.clone(),
            database: self.database.clone(),
            pending_database: self.pending_database,
            sql: self.sql.clone(),
            engine: self.engine,
            keep_ctx: self.keep_ctx,
            read_only: self.read_only,
        }
    }

    fn merge_saved(&self, mut saved: Self) -> Self {
        for id in &self.deleted_workspace_ids {
            if !saved.deleted_workspace_ids.contains(id) {
                saved.deleted_workspace_ids.push(id.clone());
            }
        }

        if !saved.deleted_workspace_ids.contains(&self.workspace_id) {
            let exists = saved.workspace_id == self.workspace_id
                || saved.switch_workspace(&self.workspace_id);
            let mut current = self.current_workspace();
            saved.swap_workspace(&mut current);
            if !exists {
                saved.workspaces.push(current);
            }
        }

        // Other tabs own their latest inactive drafts; only this active draft is replaced.
        let deleted = &saved.deleted_workspace_ids;
        saved
            .workspaces
            .retain(|workspace| !deleted.contains(&workspace.id));
        if saved.deleted_workspace_ids.contains(&saved.workspace_id)
            && !saved.remove_current_workspace()
        {
            // Two tabs cannot delete the last surviving workspace together.
            saved
                .deleted_workspace_ids
                .retain(|id| id != &saved.workspace_id);
        }
        saved.editor_config = self.editor_config.clone();
        saved.orientation = self.orientation;
        saved.theme = self.theme;
        saved.run_selected_sql = self.run_selected_sql;
        saved
    }

    pub fn load() -> Option<Self> {
        let storage = window().local_storage().ok()??;
        let value = storage.get_item("config").ok()??;
        Self::decode_config(&value).ok()
    }

    fn decode_config(value: &str) -> serde_json::Result<Self> {
        let mut state: Self = serde_json::from_str(value)?;
        if state.database.is_empty() {
            state.database = initial_database(&state.workspace_id);
        }
        for workspace in &mut state.workspaces {
            if workspace.database.is_empty() {
                workspace.database = initial_database(&workspace.id);
            }
        }
        Ok(state)
    }

    pub fn save(&self) {
        let Ok(Some(storage)) = window().local_storage() else {
            return;
        };
        let latest = storage
            .get_item("config")
            .ok()
            .flatten()
            .and_then(|value| Self::decode_config(&value).ok());
        let value = if let Some(latest) = latest {
            if latest.deleted_workspace_ids.contains(&self.workspace_id) {
                log::warn!("Workspace {} was deleted in another tab", self.workspace_id);
            }
            serde_json::to_string(&self.merge_saved(latest)).unwrap()
        } else {
            serde_json::to_string(self).unwrap()
        };
        if let Err(error) = storage.set_item("config", &value) {
            log::error!("Failed to save configuration to local storage: {error:?}");
        }
    }
}

fn initial_database(workspace_id: &str) -> String {
    if workspace_id == "default" {
        "test.db".into()
    } else {
        format!("workspace-{workspace_id}.db")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub database: String,
    #[serde(default)]
    pub pending_database: bool,
    pub sql: String,
    pub engine: Engine,
    pub keep_ctx: bool,
    #[serde(default)]
    pub read_only: bool,
}

pub struct ImportProgress {
    pub filename: String,
    pub loaded: f64,
    pub total: f64,
    pub opened: Option<bool>,
}

pub struct Exported {
    pub filename: String,
    pub mime_type: &'static str,
    pub data: SendWrapper<Uint8Array>,
}

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Engine {
    #[default]
    SQLite,
    MultipleCiphers,
    SQLCipher,
}

impl Engine {
    pub fn url_value(self) -> &'static str {
        match self {
            Self::SQLite => "sqlite3",
            Self::MultipleCiphers => "sqlite3mc",
            Self::SQLCipher => "sqlcipher",
        }
    }

    pub fn from_url_value(value: &str) -> Option<Self> {
        match value {
            "sqlite3" => Some(Self::SQLite),
            "sqlite3mc" => Some(Self::MultipleCiphers),
            "sqlcipher" => Some(Self::SQLCipher),
            _ => None,
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::SQLite => "SQLite",
            Self::MultipleCiphers => "SQLite3 Multiple Ciphers",
            Self::SQLCipher => "SQLCipher",
        }
    }

    pub fn from_select(value: &str) -> Self {
        match value {
            "SQLite" => Self::SQLite,
            "SQLite3 Multiple Ciphers" => Self::MultipleCiphers,
            "SQLCipher" => Self::SQLCipher,
            _ => unreachable!(),
        }
    }
}

fn deserialize_engine<'de, D>(deserializer: D) -> Result<Engine, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StoredEngine {
        Engine(Engine),
        MultipleCiphers(bool),
    }

    Ok(match StoredEngine::deserialize(deserializer)? {
        StoredEngine::Engine(engine) => engine,
        StoredEngine::MultipleCiphers(true) => Engine::MultipleCiphers,
        StoredEngine::MultipleCiphers(false) => Engine::SQLite,
    })
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
    use super::{Engine, GlobalState, NEW_WORKSPACE_SQL, Orientation, Theme};

    #[test]
    fn new_installations_use_default_db_and_preserve_it_on_reload() {
        let state = GlobalState::default();
        assert_eq!(state.database_filename(), "default.db");

        let saved = serde_json::to_string(&state).unwrap();
        let mut restored = GlobalState::decode_config(&saved).unwrap();
        assert_eq!(restored.database_filename(), "default.db");
        restored.create_workspace("second".into(), "Second".into());
        assert_eq!(restored.database_filename(), "default.db");
    }

    #[test]
    fn saved_database_names_are_kept_including_legacy_test_db() {
        for database in ["test.db", "default.db", "existing-project.db"] {
            let config = serde_json::json!({
                "database": database,
                "sql": "SELECT 'existing query';",
                "vfs": "OPFS",
                "keep_ctx": false,
                "read_only": true,
            });
            let state = GlobalState::decode_config(&config.to_string()).unwrap();

            assert_eq!(state.database_filename(), database);
            assert_eq!(state.sql, "SELECT 'existing query';");
            assert!(!state.keep_ctx);
            assert!(state.read_only);
            assert_eq!(serde_json::to_value(state).unwrap()["database"], database);
        }
    }

    #[test]
    fn legacy_configuration_keeps_its_database_and_query() {
        let state = GlobalState::decode_config(
            &serde_json::json!({
                "sql": "SELECT 'legacy database';",
                "engine": "SQLCipher",
                "vfs": "OPFS",
                "keep_ctx": true,
                "run_selected_sql": true,
                "theme": "Dark",
                "editor_config": { "keyboard": "vim" },
            })
            .to_string(),
        )
        .unwrap();

        assert_eq!(state.workspace_id, "default");
        assert_eq!(state.workspace_name, "Default");
        assert_eq!(state.database_filename(), "test.db");
        assert!(state.workspaces.is_empty());
        assert_eq!(state.sql, "SELECT 'legacy database';");
        assert_eq!(state.engine, Engine::SQLCipher);
        assert!(state.keep_ctx);
        assert!(state.run_selected_sql);
        assert!(!state.read_only);
        assert!(matches!(state.theme, Theme::Dark));
        assert_eq!(state.editor_config.keyboard, "vim");
    }

    #[test]
    fn legacy_storage_choices_preserve_workspace_settings_without_being_saved() {
        for (active_storage, inactive_storage) in [("Memory", "OPFS"), ("OPFS", "Memory")] {
            let mut state = GlobalState::decode_config(
                &serde_json::json!({
                    "workspace_id": "current",
                    "workspace_name": "Encrypted",
                    "database": "cipher.db",
                    "sql": "PRAGMA cipher_version;",
                    "engine": "SQLCipher",
                    "vfs": active_storage,
                    "keep_ctx": false,
                    "read_only": true,
                    "run_selected_sql": true,
                    "workspaces": [{
                        "id": "archive",
                        "name": "Archive",
                        "database": "archive.db",
                        "sql": "SELECT * FROM archived;",
                        "engine": "MultipleCiphers",
                        "vfs": inactive_storage,
                        "keep_ctx": true,
                        "read_only": false,
                    }],
                })
                .to_string(),
            )
            .unwrap();

            assert_eq!(state.workspace_id, "current");
            assert_eq!(state.workspace_name, "Encrypted");
            assert_eq!(state.database_filename(), "cipher.db");
            assert_eq!(state.sql, "PRAGMA cipher_version;");
            assert_eq!(state.engine, Engine::SQLCipher);
            assert!(state.read_only);
            assert!(!state.keep_ctx);
            assert!(state.run_selected_sql);

            let saved = serde_json::to_value(&state).unwrap();
            assert!(saved.get("vfs").is_none());
            assert!(saved["workspaces"][0].get("vfs").is_none());

            assert!(state.switch_workspace("archive"));
            assert_eq!(state.workspace_name, "Archive");
            assert_eq!(state.database_filename(), "archive.db");
            assert_eq!(state.sql, "SELECT * FROM archived;");
            assert_eq!(state.engine, Engine::MultipleCiphers);
            assert!(!state.read_only);
            assert!(state.keep_ctx);
            assert!(state.run_selected_sql);

            let saved = serde_json::to_string(&state).unwrap();
            let mut restored = GlobalState::decode_config(&saved).unwrap();
            assert!(restored.switch_workspace("current"));
            assert_eq!(restored.database_filename(), "cipher.db");
            assert_eq!(restored.sql, "PRAGMA cipher_version;");
            assert!(restored.read_only);
            assert!(!restored.keep_ctx);
        }
    }

    #[test]
    fn switching_workspaces_preserves_each_query_and_database_configuration() {
        let mut state = GlobalState {
            database: "encrypted.db".into(),
            sql: "SELECT 'first database';".into(),
            engine: Engine::SQLCipher,
            keep_ctx: true,
            run_selected_sql: true,
            read_only: true,
            ..GlobalState::default()
        };
        state.create_workspace("second".into(), "Second database".into());

        assert_eq!(state.workspace_id, "second");
        assert_eq!(state.database, "encrypted.db");
        assert_eq!(state.sql, NEW_WORKSPACE_SQL);
        assert_eq!(state.engine, Engine::SQLCipher);
        assert!(state.keep_ctx);
        assert!(state.run_selected_sql);
        assert!(state.read_only);
        state.sql = "SELECT 'second database';".into();
        state.database = "reports.db".into();
        state.engine = Engine::MultipleCiphers;
        state.keep_ctx = false;
        state.run_selected_sql = false;
        state.read_only = false;

        assert!(state.switch_workspace("default"));
        assert_eq!(state.database, "encrypted.db");
        assert_eq!(state.sql, "SELECT 'first database';");
        assert_eq!(state.engine, Engine::SQLCipher);
        assert!(state.keep_ctx);
        assert!(!state.run_selected_sql);
        assert!(state.read_only);
        assert!(!state.switch_workspace("default"));
        assert!(!state.switch_workspace("missing"));

        let saved = serde_json::to_string(&state).unwrap();
        let mut restored: GlobalState = serde_json::from_str(&saved).unwrap();
        assert!(restored.switch_workspace("second"));
        assert_eq!(restored.database, "reports.db");
        assert_eq!(restored.sql, "SELECT 'second database';");
        assert_eq!(restored.engine, Engine::MultipleCiphers);
        assert!(!restored.keep_ctx);
        assert!(!restored.run_selected_sql);
        assert!(!restored.read_only);
        assert_eq!(restored.workspaces.len(), 1);
        assert_eq!(restored.workspaces[0].id, "default");
    }

    #[test]
    fn renaming_a_configuration_keeps_its_selected_database() {
        let mut state = GlobalState {
            database: "reports.db".into(),
            ..GlobalState::default()
        };
        state.create_workspace("first".into(), "First".into());
        let filename = state.database_filename();
        state.workspace_name = "Renamed database".into();
        assert_eq!(state.database_filename(), filename);

        state.create_workspace("second".into(), "Second".into());
        assert_eq!(state.database_filename(), filename);
        state.database = "orders.db".into();
        assert_eq!(state.database_filename(), "orders.db");
        assert_eq!(
            state.workspace_options(),
            vec![
                ("second".into(), "Second".into()),
                ("default".into(), "Default".into()),
                ("first".into(), "Renamed database".into()),
            ]
        );
    }

    #[test]
    fn configurations_can_select_the_same_database_with_different_sql() {
        let mut state = GlobalState {
            database: "shared.db".into(),
            sql: "SELECT 'first configuration';".into(),
            ..GlobalState::default()
        };
        state.create_workspace("second".into(), "Second".into());
        state.database = "shared.db".into();
        state.sql = "SELECT 'second configuration';".into();

        assert!(state.switch_workspace("default"));
        assert_eq!(state.database_filename(), "shared.db");
        assert_eq!(state.sql, "SELECT 'first configuration';");
        assert!(state.switch_workspace("second"));
        assert_eq!(state.database_filename(), "shared.db");
        assert_eq!(state.sql, "SELECT 'second configuration';");
    }

    #[test]
    fn selected_sql_preference_is_global_across_workspace_changes() {
        let mut state = GlobalState {
            run_selected_sql: true,
            ..GlobalState::default()
        };
        state.create_workspace("second".into(), "Second".into());
        assert!(state.run_selected_sql);
        state.switch_workspace("default");
        assert!(state.run_selected_sql);
        state.remove_current_workspace();
        assert!(state.run_selected_sql);

        let saved = serde_json::to_string(&state).unwrap();
        let restored = GlobalState::decode_config(&saved).unwrap();
        assert!(restored.run_selected_sql);
        state.run_selected_sql = false;
        assert!(!state.merge_saved(restored).run_selected_sql);
        let serialized = serde_json::to_value(state).unwrap();
        assert_eq!(serialized["run_selected_sql"], false);
    }

    #[test]
    fn missing_database_names_keep_legacy_default_and_scratch_files() {
        let mut state = GlobalState::default();
        state.create_workspace("second".into(), "Second".into());
        let mut saved = serde_json::to_value(state).unwrap();
        saved.as_object_mut().unwrap().remove("database");
        saved["workspaces"][0]
            .as_object_mut()
            .unwrap()
            .remove("database");

        let mut restored = GlobalState::decode_config(&saved.to_string()).unwrap();
        assert_eq!(restored.database_filename(), "workspace-second.db");
        restored.switch_workspace("default");
        assert_eq!(restored.database_filename(), "test.db");
    }

    #[test]
    fn fresh_database_and_read_only_settings_round_trip_independently() {
        for read_only in [false, true] {
            for keep_ctx in [false, true] {
                let mut state = GlobalState::default();
                state.create_workspace("second".into(), "Second".into());
                state.read_only = read_only;
                state.keep_ctx = keep_ctx;
                state.workspaces[0].read_only = read_only;
                state.workspaces[0].keep_ctx = keep_ctx;

                let saved = serde_json::to_string(&state).unwrap();
                let mut restored = GlobalState::decode_config(&saved).unwrap();
                assert_eq!(restored.read_only, read_only);
                assert_eq!(restored.keep_ctx, keep_ctx);
                assert!(restored.switch_workspace("default"));
                assert_eq!(restored.read_only, read_only);
                assert_eq!(restored.keep_ctx, keep_ctx);
            }
        }
    }

    #[test]
    fn removing_a_workspace_keeps_at_least_one_database() {
        let mut state = GlobalState::default();
        assert!(!state.remove_current_workspace());
        state.create_workspace("first".into(), "First".into());
        state.create_workspace("second".into(), "Second".into());

        assert!(state.remove_current_workspace());
        assert_eq!(state.workspace_id, "default");
        assert_eq!(state.workspaces.len(), 1);
        assert_eq!(state.workspaces[0].id, "first");
        assert!(state.remove_current_workspace());
        assert_eq!(state.workspace_id, "first");
        assert!(!state.remove_current_workspace());
        assert!(state.workspaces.is_empty());
    }

    #[test]
    fn saving_a_draft_keeps_workspaces_created_in_another_tab() {
        let mut original = GlobalState {
            sql: "SELECT 'original';".into(),
            ..GlobalState::default()
        };
        let mut shared: GlobalState =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        shared.create_workspace("shared".into(), "Shared query".into());
        shared.sql = "SELECT 'shared';".into();
        let saved = shared.merge_saved(original.merge_saved(GlobalState::default()));

        original.sql = "SELECT 'new original draft';".into();
        let mut merged = original.merge_saved(saved);
        assert_eq!(merged.sql, "SELECT 'new original draft';");
        assert!(merged.switch_workspace("shared"));
        assert_eq!(merged.sql, "SELECT 'shared';");
        assert_eq!(merged.workspace_name, "Shared query");
        assert_eq!(merged.workspaces.len(), 1);
    }

    #[test]
    fn stale_inactive_drafts_do_not_replace_newer_saved_content() {
        let mut original = GlobalState::default();
        original.create_workspace("second".into(), "Second".into());
        original.sql = "SELECT 'old second';".into();
        original.switch_workspace("default");
        let mut newer: GlobalState =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        newer.switch_workspace("second");
        newer.sql = "SELECT 'new second';".into();

        original.sql = "SELECT 'new first';".into();
        let mut merged = original.merge_saved(newer);
        assert_eq!(merged.sql, "SELECT 'new first';");
        merged.switch_workspace("second");
        assert_eq!(merged.sql, "SELECT 'new second';");
    }

    #[test]
    fn deleted_workspaces_are_not_restored_by_stale_tabs() {
        let mut saved = GlobalState::default();
        saved.create_workspace("second".into(), "Second".into());
        let mut stale: GlobalState =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert!(saved.remove_current_workspace());
        stale.sql = "SELECT 'stale deleted draft';".into();

        let merged = stale.merge_saved(saved);
        assert_eq!(merged.workspace_id, "default");
        assert!(merged.workspaces.is_empty());
        assert_eq!(merged.deleted_workspace_ids, vec!["second"]);
    }

    #[test]
    fn concurrent_deletions_keep_the_last_saved_workspace() {
        let mut first = GlobalState::default();
        first.create_workspace("second".into(), "Second".into());
        let mut second: GlobalState =
            serde_json::from_str(&serde_json::to_string(&first).unwrap()).unwrap();
        assert!(first.remove_current_workspace());
        second.switch_workspace("default");
        assert!(second.remove_current_workspace());

        let merged = second.merge_saved(first);
        assert_eq!(merged.workspace_id, "default");
        assert!(merged.workspaces.is_empty());
        assert!(!merged.deleted_workspace_ids.contains(&merged.workspace_id));
    }

    #[test]
    fn refreshing_before_a_switch_loads_the_latest_inactive_draft() {
        let mut original = GlobalState {
            sql: "SELECT 'first draft';".into(),
            ..GlobalState::default()
        };
        original.create_workspace("second".into(), "Second".into());
        original.sql = "SELECT 'old second draft';".into();
        original.switch_workspace("default");
        let mut newer: GlobalState =
            serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
        newer.switch_workspace("second");
        newer.sql = "SELECT 'latest second draft';".into();
        newer.read_only = true;
        newer.keep_ctx = true;

        original.refresh_workspaces(newer);
        assert_eq!(original.workspace_id, "default");
        assert_eq!(original.sql, "SELECT 'first draft';");
        assert!(!original.read_only);
        assert!(original.switch_workspace("second"));
        assert_eq!(original.sql, "SELECT 'latest second draft';");
        assert!(original.read_only);
        assert!(original.keep_ctx);
    }

    #[test]
    fn leaving_a_deleted_active_workspace_does_not_reinsert_it() {
        let mut saved = GlobalState::default();
        saved.create_workspace("second".into(), "Second".into());
        let snapshot = serde_json::to_string(&saved).unwrap();
        let mut stale: GlobalState = serde_json::from_str(&snapshot).unwrap();
        saved.remove_current_workspace();
        let deleted = serde_json::to_string(&saved).unwrap();

        stale.refresh_workspaces(serde_json::from_str(&deleted).unwrap());
        assert_eq!(stale.workspace_id, "second");
        assert!(stale.switch_workspace("default"));
        assert!(stale.workspaces.is_empty());

        let mut stale: GlobalState = serde_json::from_str(&snapshot).unwrap();
        stale.refresh_workspaces(saved);
        stale.create_workspace("third".into(), "Third".into());
        assert_eq!(stale.workspace_id, "third");
        assert_eq!(stale.workspaces.len(), 1);
        assert_eq!(stale.workspaces[0].id, "default");
    }

    #[test]
    fn missing_preferences_use_defaults() {
        let state: GlobalState =
            serde_json::from_str(r#"{"editor_config":{"keyboard":"vim"}}"#).unwrap();

        assert_eq!(state.editor_config.keyboard, "vim");
        assert_eq!(state.editor_config.light_theme, "github");
        assert_eq!(state.engine, Engine::SQLite);
        assert!(matches!(state.theme, Theme::System));
        assert!(!state.sql.is_empty());
    }

    #[test]
    fn previous_cipher_toggle_migrates_to_engine() {
        for (enabled, engine) in [(false, Engine::SQLite), (true, Engine::MultipleCiphers)] {
            let config = serde_json::json!({ "multiple_ciphers": enabled });
            let state: GlobalState = serde_json::from_value(config).unwrap();

            assert_eq!(state.engine, engine);

            let saved = serde_json::to_value(state).unwrap();
            assert_eq!(saved["engine"], serde_json::to_value(engine).unwrap());
            assert!(saved.get("multiple_ciphers").is_none());
        }
    }

    #[test]
    fn every_engine_survives_configuration_round_trip() {
        for engine in [Engine::SQLite, Engine::MultipleCiphers, Engine::SQLCipher] {
            let state = GlobalState {
                engine,
                ..GlobalState::default()
            };
            let saved = serde_json::to_string(&state).unwrap();
            let restored: GlobalState = serde_json::from_str(&saved).unwrap();

            assert_eq!(restored.engine, engine);
        }
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
            is_busy: true,
            databases: vec!["test.db".into(), "shared.db".into()],
            ..GlobalState::default()
        };
        let value = serde_json::to_value(state).unwrap();

        assert_eq!(value["theme"], "System");
        assert!(value.get("is_running").is_none());
        assert!(value.get("is_busy").is_none());
        assert!(value.get("databases").is_none());
        assert!(value.get("editor").is_none());
        assert!(value.get("output").is_none());
    }
}
