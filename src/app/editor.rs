use aceditor::EditorOptionsBuilder;
use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen_futures::spawn_local;
use web_sys::UrlSearchParams;

use crate::{
    SQLightError,
    app::{GlobalState, GlobalStateStoreFields, header::execute},
};

istyles!(styles, "assets/module.postcss/editor.module.css.map");

#[component]
pub fn Editor() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let editor_ref = NodeRef::new();

    editor_ref.on_load(move |_| {
        let shared_code = || {
            let search = window().location().search().ok()?;
            let params = UrlSearchParams::new_with_str(&search).ok()?;
            params.get("code")
        };
        let opt = EditorOptionsBuilder::default()
            .mode("ace/mode/sql")
            .theme(&format!(
                "ace/theme/{}",
                state.editor_config().read_untracked().theme
            ))
            .keyboard(&format!(
                "ace/keyboard/{}",
                state.editor_config().read_untracked().keyboard
            ))
            .value(&shared_code().unwrap_or_else(|| state.code().get_untracked()))
            .build();

        match aceditor::Editor::open("ace_editor", Some(&opt)) {
            Ok(editor) => state.editor().set(Some(editor)),
            Err(err) => state
                .last_error()
                .set(Some(SQLightError::new_ace_editor(err))),
        }

        spawn_local(async move {
            if let Err(err) = aceditor::Editor::define_vim_w(execute(state)).await {
                state
                    .last_error()
                    .set(Some(SQLightError::new_ace_editor(err)));
            }
        });
    });
    view! {
        <div class=styles::container>
            <div node_ref=editor_ref id="ace_editor" class=styles::ace></div>
        </div>
    }
}
