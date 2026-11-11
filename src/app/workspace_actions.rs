use super::{GlobalState, GlobalStateStoreFields};
use leptos::prelude::*;
use reactive_stores::Store;

fn save_editor(state: Store<GlobalState>) {
    let sql = state
        .editor()
        .with_untracked(|editor| editor.as_ref().map(|editor| editor.get_value()));
    if let Some(sql) = sql {
        state.sql().set(sql);
    }
    state.read_untracked().save();
    if let Some(saved) = GlobalState::load() {
        state.write().refresh_workspaces(saved);
    }
}

fn load_editor(state: Store<GlobalState>) {
    let sql = state.sql().get_untracked();
    state.editor().with_untracked(|editor| {
        if let Some(editor) = editor {
            editor.set_workspace_value(sql);
        }
    });
    batch(move || {
        state.output().set(Vec::new());
        state.focus().set(None);
        state.opened_focus().update(|opened| opened.clear());
        state.show_something().set(false);
        state.share_href().set(None);
        state.last_error().set(None);
        state.import_progress().set(None);
        state.exported().set(None);
    });
    state.read_untracked().save();
}

pub(crate) fn switch_workspace(state: Store<GlobalState>, id: String) {
    if state.is_busy().get_untracked() || state.workspace_id().get_untracked() == id {
        return;
    }
    save_editor(state);
    let changed = state.write().switch_workspace(&id);
    if changed {
        load_editor(state);
    }
}

pub(crate) fn new_workspace(state: Store<GlobalState>) {
    if state.is_busy().get_untracked() {
        return;
    }
    save_editor(state);
    let id = format!(
        "{:x}-{:013x}",
        js_sys::Date::now() as u64,
        (js_sys::Math::random() * (1u64 << 52) as f64) as u64,
    );
    let name = format!(
        "Workspace {}",
        state.workspaces().read_untracked().len() + 2
    );
    state.write().create_workspace(id, name);
    load_editor(state);
}

pub(crate) fn rename_workspace(state: Store<GlobalState>, name: String) {
    let name = name.trim();
    if state.is_busy().get_untracked() || name.is_empty() {
        return;
    }
    state.workspace_name().set(name.to_owned());
    state.read_untracked().save();
}

pub(crate) fn use_database(state: Store<GlobalState>, filename: String) {
    batch(move || {
        state.database().set(filename);
        state.keep_ctx().set(true);
    });
    state.read_untracked().save();
}

pub(crate) fn delete_workspace(state: Store<GlobalState>) {
    if state.is_busy().get_untracked() {
        return;
    }
    save_editor(state);
    if state.workspaces().read_untracked().is_empty() {
        return;
    }
    let name = state.workspace_name().get_untracked();
    let confirmed = window()
        .confirm_with_message(&format!(
            "Delete workspace “{name}” and its SQL draft? Database files will be kept."
        ))
        .unwrap_or(false);
    if !confirmed {
        return;
    }

    state.write().remove_current_workspace();
    load_editor(state);
}
