use std::ops::Deref;

use istyles::istyles;
use leptos::prelude::*;
use leptos::tachys::html;
use prettytable::{Cell, Row, Table};
use reactive_stores::Store;
use split_grid::{Gutter, SplitOptions};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::wasm_bindgen::JsValue;

use crate::{
    SQLightError, SQLiteStatementResult,
    app::{
        Focus,
        editor::Editor,
        header::Header,
        output::{Output, change_focus},
        state::{GlobalState, GlobalStateStoreFields, Orientation, Theme},
    },
};

istyles!(styles, "assets/module.postcss/playground.module.css.map");

#[component]
pub fn Playground() -> impl IntoView {
    let state = GlobalState::load().unwrap_or_default();
    provide_context(Store::new(state));

    let state = expect_context::<Store<GlobalState>>();

    handle_last_error(state);
    handle_system_theme(state);
    handle_automic_orientation(state);
    handle_save_state(state);
    handle_import_progress(state);
    handle_ace_config(state);
    handle_embed_query_result(state);

    view! {
        <div id="playground" class=styles::container>
            <Header />
            <ResizableArea />
        </div>
    }
    .into_any()
}

fn handle_embed_query_result(state: Store<GlobalState>) {
    const MARK: &str = "-- R:";

    Effect::new(move || {
        let mut sqls = vec![];

        for result in &*state.embed().read() {
            let mut table_s = Table::new();

            match result {
                SQLiteStatementResult::Finish => continue,
                SQLiteStatementResult::Step(table) => {
                    let sql = table.sql.trim().to_string();

                    let sql = if let Some(values) = &table.values {
                        table_s.add_row(Row::new(
                            values.columns.iter().map(|s| Cell::new(s)).collect(),
                        ));
                        for row in &values.rows {
                            table_s.add_row(Row::new(row.iter().map(|s| Cell::new(s)).collect()));
                        }

                        let mut result = table_s
                            .to_string()
                            .lines()
                            .map(|x| format!("{MARK} {x}"))
                            .collect::<Vec<String>>()
                            .join("\n");

                        result.push('\n');

                        let sql = sql
                            .lines()
                            .filter(|s| !s.starts_with(MARK))
                            .collect::<Vec<_>>()
                            .join("\n");
                        result.push_str(&sql);

                        result
                    } else {
                        sql
                    };

                    sqls.push(sql);
                }
            }
        }

        if !sqls.is_empty() {
            if let Some(editor) = &*state.editor().read_untracked() {
                editor.set_value(sqls.join("\n\n"));
            }
        }
    });
}

fn handle_ace_config(state: Store<GlobalState>) {
    Effect::new(move || {
        let config = state.editor_config().read();
        let keyboard = &config.keyboard;
        let theme = match state.theme().read().value() {
            Theme::SystemLight | Theme::Light => &config.light_theme,
            Theme::SystemDark | Theme::Dark => &config.dark_theme,
            Theme::System => unreachable!(),
        };

        if let Some(Err(err)) = state.editor().read().as_ref().map(|editor| {
            let handler = (keyboard != "ace").then_some(format!("ace/keyboard/{keyboard}"));
            editor.set_keyboard_handler(handler.as_deref())
        }) {
            state
                .last_error()
                .set(Some(SQLightError::new_ace_editor(err)));
        }

        if let Some(Err(err)) = state
            .editor()
            .read()
            .as_ref()
            .map(|editor| editor.set_theme(&format!("ace/theme/{theme}")))
        {
            state
                .last_error()
                .set(Some(SQLightError::new_ace_editor(err)));
        }
    });
}

fn handle_save_state(state: Store<GlobalState>) {
    Effect::new(move || {
        state.vfs().track();
        state.editor_config().track();
        state.orientation().track();
        state.theme().track();
        state.keep_ctx().track();
        state.sql().track();
        state.run_selected_sql().track();
        state.multiple_ciphers().track();

        state.read_untracked().save();
    });
}

fn handle_last_error(state: Store<GlobalState>) {
    Effect::new(move || {
        if state.last_error().read().is_some() {
            change_focus(state, Some(Focus::Status));
        }
    });
}

fn handle_import_progress(state: Store<GlobalState>) {
    Effect::new(move || {
        if state.import_progress().read().is_some() {
            change_focus(state, Some(Focus::Status));
        }
    });
}

fn handle_system_theme(state: Store<GlobalState>) {
    Effect::new(move || {
        let theme = match state.theme().read().value() {
            Theme::SystemLight | Theme::Light => "light",
            Theme::SystemDark | Theme::Dark => "dark",
            Theme::System => unreachable!(),
        };
        if let Some(element) = document().document_element() {
            element.set_attribute("data-theme", theme).unwrap()
        }
    });

    if let Some(query) = Theme::match_media() {
        let f = move |query: web_sys::MediaQueryList| {
            if state.theme().get_untracked().is_system() {
                state.theme().set(if query.matches() {
                    Theme::SystemDark
                } else {
                    Theme::SystemLight
                });
            }
        };
        let callback = Closure::<dyn Fn(web_sys::MediaQueryList)>::new(f);
        query
            .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
            .unwrap();
        callback.forget();
    }
}

fn handle_automic_orientation(state: Store<GlobalState>) {
    let auto_change = move |query: web_sys::MediaQueryList| {
        if state.orientation().read().is_auto() {
            let value = if query.matches() {
                Orientation::AutoHorizontal
            } else {
                Orientation::AutoVertical
            };
            state
                .orientation()
                .maybe_update(|orientation| std::mem::replace(orientation, value) != value);
        }
    };

    Effect::new(move || {
        if let Some(query) = Orientation::match_media() {
            auto_change(query);
        }
    });

    if let Some(query) = Orientation::match_media() {
        let callback = Closure::<dyn Fn(web_sys::MediaQueryList)>::new(auto_change);
        query
            .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
            .unwrap();
        callback.forget();
    }
}

fn gird_style() -> String {
    let state = expect_context::<Store<GlobalState>>();

    let (focused_grid_style, unfocused_grid_style) = match state.orientation().read().value() {
        Orientation::Horizontal | Orientation::AutoHorizontal => (
            styles::resizeableAreaRowOutputFocused.to_string(),
            styles::resizeableAreaRowOutputUnfocused.to_string(),
        ),
        Orientation::Vertical | Orientation::AutoVertical => (
            styles::resizeableAreaColumnOutputFocused.to_string(),
            styles::resizeableAreaColumnOutputUnfocused.to_string(),
        ),
        Orientation::Automatic => unreachable!(),
    };

    if state.read().is_focus() {
        focused_grid_style
    } else {
        unfocused_grid_style
    }
}

fn handle_outer_style() -> String {
    let state = expect_context::<Store<GlobalState>>();

    match state.orientation().read().value() {
        Orientation::Horizontal | Orientation::AutoHorizontal => {
            styles::splitRowsGutter.to_string()
        }
        Orientation::Vertical | Orientation::AutoVertical => styles::splitColumnsGutter.to_string(),
        Orientation::Automatic => unreachable!(),
    }
}

fn handle_inner_style() -> String {
    let state = expect_context::<Store<GlobalState>>();

    match state.orientation().read().value() {
        Orientation::Horizontal | Orientation::AutoHorizontal => {
            styles::splitRowsGutterHandle.to_string()
        }
        Orientation::Vertical | Orientation::AutoVertical => String::new(),
        Orientation::Automatic => unreachable!(),
    }
}

#[component]
fn ResizableArea() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let node_ref = NodeRef::<html::element::Div>::new();
    let drag_handle = NodeRef::<html::element::Div>::new();

    Effect::new(move || {
        state.orientation().track();
        state.is_focused().track();

        if let Some(div) = &*node_ref.read() {
            let style = div.deref().style();
            let _ = style.remove_property("grid-template-columns");
            let _ = style.remove_property("grid-template-rows");
        }
    });

    Effect::new(move || {
        state.show_something().track();

        let element = if let Some(element) = &*drag_handle.read() {
            JsValue::from(element)
        } else {
            JsValue::null()
        };

        let options = match state.orientation().read().value() {
            Orientation::Horizontal | Orientation::AutoHorizontal => SplitOptions {
                min_size: 100,
                row_gutters: Some(vec![Gutter { track: 1, element }]),
                column_gutters: None,
            },
            Orientation::Vertical | Orientation::AutoVertical => SplitOptions {
                min_size: 100,
                row_gutters: None,
                column_gutters: Some(vec![Gutter { track: 1, element }]),
            },

            Orientation::Automatic => unreachable!(),
        };
        let grid = split_grid::split(&options.into());
        on_cleanup(move || grid.destroy());
    });

    view! {
        <div node_ref=node_ref class=gird_style>
            <div class=styles::editor>
                <Editor />
            </div>
            <Show when=move || state.read().is_focus() fallback=|| ()>
                <div node_ref=drag_handle class=handle_outer_style>
                    <span class=handle_inner_style>"⣿"</span>
                </div>
            </Show>
            <Show when=move || *state.show_something().read() fallback=|| ()>
                <div class=styles::output>
                    <Output />
                </div>
            </Show>

        </div>
    }
}
