use aceditor::{BindKey, EditorOptionsBuilder};
use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen::{JsCast, prelude::Closure};
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
            .value(&shared_code().unwrap_or_else(|| state.sql().get_untracked()))
            .build();

        match aceditor::Editor::open("ace_editor", Some(&opt)) {
            Ok(editor) => {
                let exec = Closure::<dyn Fn() + 'static>::new(execute(state));
                let command = aceditor::Command {
                    name: "executeCode".into(),
                    bind_key: BindKey {
                        win: "Ctrl-Enter".into(),
                        mac: "Ctrl-Enter|Command-Enter".into(),
                    },
                    exec: exec.as_ref().unchecked_ref::<js_sys::Function>().clone(),
                    read_only: true,
                };
                exec.forget();
                editor.add_command(command);
                state.editor().set(Some(editor));
            }
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
