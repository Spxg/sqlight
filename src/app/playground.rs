use std::time::Duration;

use istyles::istyles;
use leptos::prelude::*;
use prettytable::{Cell, Row, Table};
use reactive_stores::Store;
use split_grid::{Gutter, SplitOptions};
use wasm_bindgen::{JsCast, prelude::Closure};

use crate::{
    SQLightError, SQLiteStatementResult, SQLiteStatementValues, SendWrapper,
    app::{
        Focus,
        editor::Editor,
        header::Header,
        output::{Output, change_focus},
        state::{GlobalState, GlobalStateStoreFields, Orientation, Theme},
    },
};

istyles!(styles, "assets/module.postcss/playground.module.css.map");

#[derive(Clone, Copy)]
pub(super) struct ResolvedTheme(pub Memo<Theme>);

#[component]
pub fn Playground() -> impl IntoView {
    let state = Store::new(GlobalState::load().unwrap_or_default());
    provide_context(state);

    let prefers_dark = media_query("(prefers-color-scheme: dark)");
    let narrow_screen = media_query("(max-width: 1600px)");
    let theme = Memo::new(move |_| state.theme().get().resolve(prefers_dark.get()));
    let orientation = Memo::new(move |_| state.orientation().get().resolve(narrow_screen.get()));
    provide_context(ResolvedTheme(theme));

    handle_system_theme(theme);
    handle_status(state);
    handle_save_state(state);
    handle_ace_config(state, theme);

    view! {
        <div id="playground" class=styles::container>
            <Header />
            <ResizableArea orientation=orientation />
        </div>
    }
}

fn media_query(query: &str) -> RwSignal<bool> {
    let query = window().match_media(query).unwrap();
    let matches = RwSignal::new(query.as_ref().is_some_and(|query| query.matches()));

    if let Some(query) = query {
        let observed_query = query.clone();
        let callback = Closure::<dyn Fn()>::new(move || matches.set(observed_query.matches()));
        query
            .add_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
            .unwrap();

        let query = SendWrapper::new(query);
        let callback = SendWrapper::new(callback);
        on_cleanup(move || {
            query
                .remove_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
                .unwrap();
        });
    }

    matches
}

pub(crate) fn embed_query_result(source: &str, results: &[SQLiteStatementResult]) -> String {
    let mut output = String::with_capacity(source.len());
    let mut consumed = 0;

    for result in results {
        let SQLiteStatementResult::Step(table) = result else {
            continue;
        };
        let [start, end] = table.position;
        output.push_str(&source[consumed..start]);
        let sql = without_leading_results(&source[start..end]);

        if let Some(values) = &table.values {
            // Keep blank lines before the result and indentation with the SQL.
            let whitespace_end = sql.len() - sql.trim_start().len();
            let prefix_end = sql[..whitespace_end]
                .rfind('\n')
                .map_or(0, |index| index + 1);
            output.push_str(&sql[..prefix_end]);
            if !output.is_empty() && !output.ends_with('\n') {
                output.push('\n');
            }
            output.push_str(&result_comment(values));
            output.push_str(&sql[prefix_end..]);
        } else {
            output.push_str(&sql);
        }
        consumed = end;
    }

    output.push_str(&source[consumed..]);
    output
}

fn without_leading_results(mut sql: &str) -> String {
    let mut output = String::with_capacity(sql.len());
    loop {
        let whitespace = sql.bytes().take_while(u8::is_ascii_whitespace).count();
        output.push_str(&sql[..whitespace]);
        sql = &sql[whitespace..];

        if sql.starts_with("--") {
            let end = sql.find('\n').map_or(sql.len(), |index| index + 1);
            let generated = sql.starts_with("-- R: +") || sql.starts_with("-- R: |");
            if !generated {
                output.push_str(&sql[..end]);
            }
            sql = &sql[end..];
        } else if sql.starts_with("/*") {
            let end = sql.find("*/").map_or(sql.len(), |index| index + 2);
            output.push_str(&sql[..end]);
            sql = &sql[end..];
        } else if let Some(rest) = sql.strip_prefix(';') {
            output.push(';');
            sql = rest;
        } else {
            // SQL strings and comments inside the statement remain untouched.
            output.push_str(sql);
            break;
        }
    }
    output
}

fn result_comment(values: &SQLiteStatementValues) -> String {
    let mut rendered = Table::new();
    rendered.add_row(Row::new(
        values
            .columns
            .iter()
            .map(|column| Cell::new(column))
            .collect(),
    ));
    for row in &values.rows {
        rendered.add_row(Row::new(row.iter().map(|value| Cell::new(value)).collect()));
    }

    let rendered = rendered.to_string().replace('\0', "\\0");
    let mut result = String::new();
    for line in rendered.lines() {
        result.push_str("-- R: ");
        result.push_str(line);
        result.push('\n');
    }
    result
}

fn handle_ace_config(state: Store<GlobalState>, theme: Memo<Theme>) {
    Effect::new(move || {
        let theme = theme.get();
        let (keyboard, theme) = state.editor_config().with(|config| {
            let keyboard =
                (config.keyboard != "ace").then(|| format!("ace/keyboard/{}", config.keyboard));
            let theme = match theme {
                Theme::Light => &config.light_theme,
                Theme::Dark => &config.dark_theme,
                Theme::System => unreachable!(),
            };
            (keyboard, format!("ace/theme/{theme}"))
        });
        let result = state.editor().with(|editor| {
            editor.as_ref().map(|editor| {
                editor.set_keyboard_handler(keyboard.as_deref())?;
                editor.set_theme(&theme)
            })
        });

        if let Some(Err(error)) = result {
            state
                .last_error()
                .set(Some(SQLightError::new_ace_editor(error)));
        }
    });
}

fn handle_save_state(state: Store<GlobalState>) {
    let pending = StoredValue::new(None::<TimeoutHandle>);
    let save = move || {
        if let Some(timeout) = pending.get_value() {
            timeout.clear();
            pending.set_value(None);
            state.read_untracked().save();
        }
    };

    Effect::watch(
        move || {
            state.editor_config().track();
            state.orientation().track();
            state.theme().track();
            state.keep_ctx().track();
            state.sql().track();
            state.run_selected_sql().track();
            state.engine().track();
            state.workspace_id().track();
            state.workspace_name().track();
            state.workspaces().track();
            state.read_only().track();
            state.database().track();
        },
        move |_, _, _| {
            if let Some(timeout) = pending.get_value() {
                timeout.clear();
            }
            let timeout = set_timeout_with_handle(save, Duration::from_millis(300)).unwrap();
            pending.set_value(Some(timeout));
        },
        false,
    );

    let callback = Closure::<dyn Fn()>::new(save);
    window()
        .add_event_listener_with_callback("pagehide", callback.as_ref().unchecked_ref())
        .unwrap();
    let callback = SendWrapper::new(callback);
    on_cleanup(move || {
        if let Some(timeout) = pending.get_value() {
            timeout.clear();
        }
        window()
            .remove_event_listener_with_callback("pagehide", callback.as_ref().unchecked_ref())
            .unwrap();
    });
}

fn handle_status(state: Store<GlobalState>) {
    Effect::new(move || {
        if state.last_error().read().is_some() {
            change_focus(state, Some(Focus::Status));
        }
    });
    Effect::new(move || {
        if state.import_progress().read().is_some() {
            change_focus(state, Some(Focus::Status));
        }
    });
}

fn handle_system_theme(theme: Memo<Theme>) {
    Effect::new(move || {
        let theme = match theme.get() {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::System => unreachable!(),
        };
        if let Some(element) = document().document_element() {
            element.set_attribute("data-theme", theme).unwrap();
        }
    });
}

#[component]
fn ResizableArea(orientation: Memo<Orientation>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let focused = Memo::new(move |_| state.focus().get().is_some());
    let node_ref = NodeRef::<leptos::html::Div>::new();
    let drag_handle = NodeRef::<leptos::html::Div>::new();

    Effect::new(move || {
        let orientation = orientation.get();
        let focused = focused.get();
        let Some(container) = node_ref.get() else {
            return;
        };
        let style = web_sys::HtmlElement::style(&container);
        style.remove_property("grid-template-columns").unwrap();
        style.remove_property("grid-template-rows").unwrap();

        if !focused {
            return;
        }
        let Some(handle) = drag_handle.get() else {
            return;
        };
        let gutter = Gutter {
            track: 1,
            element: handle.into(),
        };
        let options = match orientation {
            Orientation::Horizontal => SplitOptions {
                min_size: 100,
                row_gutters: Some(vec![gutter]),
                column_gutters: None,
            },
            Orientation::Vertical => SplitOptions {
                min_size: 100,
                row_gutters: None,
                column_gutters: Some(vec![gutter]),
            },
            Orientation::Automatic => unreachable!(),
        };
        let grid = SendWrapper::new(split_grid::split(&options.into()));
        on_cleanup(move || grid.destroy());
    });

    let grid_class = move || match (orientation.get(), focused.get()) {
        (Orientation::Horizontal, true) => styles::resizeableAreaRowOutputFocused,
        (Orientation::Horizontal, false) => styles::resizeableAreaRowOutputUnfocused,
        (Orientation::Vertical, true) => styles::resizeableAreaColumnOutputFocused,
        (Orientation::Vertical, false) => styles::resizeableAreaColumnOutputUnfocused,
        (Orientation::Automatic, _) => unreachable!(),
    };
    let gutter_class = move || match orientation.get() {
        Orientation::Horizontal => styles::splitRowsGutter,
        Orientation::Vertical => styles::splitColumnsGutter,
        Orientation::Automatic => unreachable!(),
    };
    let handle_class = move || match orientation.get() {
        Orientation::Horizontal => styles::splitRowsGutterHandle,
        Orientation::Vertical => "",
        Orientation::Automatic => unreachable!(),
    };

    view! {
        <div node_ref=node_ref class=grid_class>
            <div class=styles::editor>
                <Editor />
            </div>
            <Show when=move || focused.get()>
                <div node_ref=drag_handle class=gutter_class>
                    <span class=handle_class>"⣿"</span>
                </div>
            </Show>
            <Show when=move || state.show_something().get()>
                <div class=styles::output>
                    <Output />
                </div>
            </Show>
        </div>
    }
}
