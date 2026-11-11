use aceditor::{BindKey, EditorOptionsBuilder};
use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{ResizeObserver, UrlSearchParams};

use crate::{
    SQLightError, SendWrapper,
    app::{GlobalState, GlobalStateStoreFields, Theme, header::execute, playground::ResolvedTheme},
};

istyles!(styles, "assets/module.postcss/editor.module.css.map");

#[component]
pub fn Editor() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let theme = expect_context::<ResolvedTheme>().0;
    let editor_ref = NodeRef::<leptos::html::Div>::new();

    if let Some(code) = shared_code() {
        state.sql().set(code);
    }

    editor_ref.on_load(move |element| {
        let options = state.editor_config().with_untracked(|config| {
            let theme = match theme.get_untracked() {
                Theme::Light => &config.light_theme,
                Theme::Dark => &config.dark_theme,
                Theme::System => unreachable!(),
            };
            let keyboard =
                (config.keyboard != "ace").then(|| format!("ace/keyboard/{}", config.keyboard));
            EditorOptionsBuilder::default()
                .mode("ace/mode/sql")
                .theme(&format!("ace/theme/{theme}"))
                .keyboard(keyboard.as_deref())
                .value(&state.sql().get_untracked())
                .build()
        });
        let mut editor = match aceditor::Editor::open("ace_editor", Some(&options)) {
            Ok(editor) => editor,
            Err(error) => {
                state
                    .last_error()
                    .set(Some(SQLightError::new_ace_editor(error)));
                return;
            }
        };

        editor.add_command(
            "executeCode",
            BindKey {
                win: "Ctrl-Enter".into(),
                mac: "Ctrl-Enter|Command-Enter".into(),
            },
            execute(state),
        );
        editor.on_change(move |sql| {
            // Ace emits changes synchronously, including while the store's editor is borrowed.
            spawn_local(async move {
                state.sql().set(sql);
            });
        });
        state.editor().set(Some(SendWrapper::new(editor)));

        if let Err(error) = aceditor::Editor::define_vim_w() {
            state
                .last_error()
                .set(Some(SQLightError::new_ace_editor(error)));
        }

        let callback = Closure::<dyn Fn()>::new(move || {
            state.editor().with_untracked(|editor| {
                if let Some(editor) = editor {
                    editor.resize();
                }
            });
        });
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).unwrap();
        observer.observe(&element);
        let observer = SendWrapper::new(observer);
        let callback = SendWrapper::new(callback);
        on_cleanup(move || {
            observer.disconnect();
            drop(callback);
            state.editor().set(None);
        });
    });

    view! {
        <div class=styles::container>
            <div node_ref=editor_ref id="ace_editor" class=styles::ace></div>
        </div>
    }
}

fn shared_code() -> Option<String> {
    let search = window().location().search().ok()?;
    UrlSearchParams::new_with_str(&search).ok()?.get("code")
}
