use istyles::istyles;
use leptos::{html::Input, prelude::*, tachys::html};
use reactive_stores::Store;
use sqlformat::{FormatOptions, QueryParams};
use std::{collections::HashSet, time::Duration};

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, Event, HtmlInputElement, Url};

use crate::{
    DatabaseFile, LoadDbsOptions, RunOptions, SQLightError, SendWrapper, WorkerRequest,
    app::{
        ImportProgress,
        button_set::{Button, ButtonSet, LinkButton},
        config_menu::ConfigMenu,
        files::DatabaseMenu,
        icon::{build_icon, config_icon, expandable_icon, github_icon},
        output::change_focus,
        pop_button::PopButton,
        state::{Focus, GlobalState, GlobalStateStoreFields},
        tools_menu::ToolsMenu,
        workspace_menu::WorkspaceMenu,
    },
    archive::unpack_zip,
    send_request,
};

istyles!(styles, "assets/module.postcss/header.module.css.map");

#[component]
pub fn Header() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let menu_container = NodeRef::new();

    let input_ref = NodeRef::new();

    view! {
        <>
            <div id="header" class=styles::container>
                <div class=styles::left>
                    <ButtonSet>
                        <ExecuteButton />
                    </ButtonSet>

                    <ButtonSet>
                        <WorkspaceMenuButton menu_container=menu_container />
                    </ButtonSet>
                    <ButtonSet>
                        <DatabaseButton input_ref=input_ref menu_container=menu_container />
                    </ButtonSet>
                </div>
                <div class=styles::right>
                    <input
                        type="file"
                        multiple
                        node_ref=input_ref
                        style="display: none"
                        on:change=move |event| import_database(state, event)
                    />

                    <ButtonSet>
                        <ShareButton />
                    </ButtonSet>
                    <ButtonSet>
                        <ToolsButton menu_container=menu_container />
                    </ButtonSet>
                    <ButtonSet>
                        <ConfigMenuButton menu_container=menu_container />
                    </ButtonSet>
                    <ButtonSet>
                        <GithubButton />
                    </ButtonSet>

                </div>
            </div>
            <div node_ref=menu_container></div>
        </>
    }
}

pub fn execute(state: Store<GlobalState>) -> Box<dyn Fn() + Send + 'static> {
    Box::new(move || {
        if state.is_busy().get_untracked() {
            return;
        }
        let editor_guard = state.editor().read_untracked();
        let Some(editor) = editor_guard.as_ref() else {
            return;
        };

        let (code, selected_code) = (editor.get_value(), editor.get_selected_value());

        drop(editor_guard);

        state.sql().set(code.clone());
        change_focus(state, Some(Focus::Execute));
        state.output().set(Vec::new());

        let run_selected_code =
            !selected_code.trim().is_empty() && state.run_selected_sql().get_untracked();

        send_request(
            state,
            WorkerRequest::Run(RunOptions {
                embed: false,
                sql: if run_selected_code {
                    selected_code
                } else {
                    code
                },
                clear_on_prepare: !state.keep_ctx().get_untracked(),
            }),
        );
    })
}

#[component]
fn ExecuteButton() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let on_click = execute(state);

    view! {
        <Button
            is_primary=true
            icon_right=build_icon()
            disabled=Signal::derive(move || state.is_busy().get())
            on_click=move |_| on_click()
        >
            "Run"
        </Button>
    }
}

#[component]
fn WorkspaceMenuButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let button = move |toggle, node_ref| {
        view! {
            <Button
                icon_right=expandable_icon()
                on_click=toggle
                node_ref=node_ref
                aria_label=Signal::derive(move || Some(format!(
                    "Workspace: {} ({})",
                    state.workspace_name().get(),
                    if state.read_only().get() { "Read only" } else { "Read write" },
                )))
            >
                <span class=styles::workspaceName>{move || state.workspace_name().get()}</span>
                " "
                <span class=styles::workspaceMode>
                    <span
                        style:visibility=move || {
                            if state.read_only().get() { "visible" } else { "hidden" }
                        }
                        aria-hidden=move || (!state.read_only().get()).to_string()
                    >
                        "(Read only)"
                    </span>
                    <span
                        style:visibility=move || {
                            if state.read_only().get() { "hidden" } else { "visible" }
                        }
                        aria-hidden=move || state.read_only().get().to_string()
                    >
                        "(Read write)"
                    </span>
                </span>
            </Button>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(|_close| view! { <WorkspaceMenu /> }.into_any())
            menu_container=menu_container
        />
    }
}

#[component]
fn ConfigMenuButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let button = |toggle, node_ref| {
        view! {
            <Button
                icon_left=config_icon()
                icon_right=expandable_icon()
                on_click=toggle
                node_ref=node_ref
            >
                "Config"
            </Button>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(|_close| { view! { <ConfigMenu /> }.into_any() })
            menu_container=menu_container
        ></PopButton>
    }
}

#[component]
fn ToolsButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let button = |toggle, node_ref| {
        view! {
            <Button icon_right=expandable_icon() on_click=toggle node_ref=node_ref>
                "Tools"
            </Button>
        }
        .into_any()
    };

    let on_format = move |_event, signal: WriteSignal<bool>| {
        let Some(editor) = &*state.editor().read_untracked() else {
            return;
        };

        let format_options = FormatOptions {
            uppercase: Some(true),
            lines_between_queries: 2,
            ..Default::default()
        };

        let sql = sqlformat::format(
            &editor.get_value(),
            &QueryParams::default(),
            &format_options,
        );

        editor.set_value(sql);
        signal.set(false);
    };

    let on_embed = move |_event, signal: WriteSignal<bool>| {
        if state.is_busy().get_untracked() {
            return;
        }
        let editor_guard = state.editor().read_untracked();
        let Some(editor) = editor_guard.as_ref() else {
            return;
        };
        let sql = editor.get_value();
        drop(editor_guard);

        send_request(
            state,
            WorkerRequest::Run(RunOptions {
                embed: true,
                sql,
                clear_on_prepare: !state.keep_ctx().get_untracked(),
            }),
        );

        signal.set(false);
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(move |signal: WriteSignal<bool>| {
                view! {
                    <ToolsMenu
                        busy=Signal::derive(move || state.is_busy().get())
                        on_format=move |e| {
                            on_format(e, signal);
                        }
                        on_embed=move |e| {
                            on_embed(e, signal);
                        }
                        on_internal=move |_| {
                            signal.set(false);
                        }
                    />
                }
                    .into_any()
            })
            menu_container=menu_container
        ></PopButton>
    }
}

#[component]
fn DatabaseButton(
    input_ref: NodeRef<Input>,
    menu_container: NodeRef<html::element::Div>,
) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let upload = Callback::new(move |_| {
        if !state.is_busy().get_untracked()
            && let Some(input) = input_ref.get_untracked()
        {
            input.set_value("");
            input.click();
        }
    });
    Effect::new(move || {
        state.exported().track();

        let Some(downloaded) = state.exported().write_untracked().take() else {
            return;
        };
        let filename = downloaded.filename;
        let buffer = downloaded.data;
        let array = js_sys::Array::new();
        array.push(&buffer);

        let options = BlobPropertyBag::new();
        options.set_type(downloaded.mime_type);
        let blob = Blob::new_with_u8_array_sequence_and_options(&array, &options).unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();

        let document = document();
        let a = document
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();

        a.set_href(&url);
        a.set_download(&filename);
        document.body().unwrap().append_child(&a).unwrap();
        a.click();
        a.remove();
        set_timeout(
            move || Url::revoke_object_url(&url).unwrap(),
            Duration::from_secs(1),
        );
    });

    let button = |toggle, node_ref| {
        view! {
            <Button icon_right=expandable_icon() on_click=toggle node_ref=node_ref>
                "Database"
            </Button>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(move |_close| view! { <DatabaseMenu upload=upload /> }.into_any())
            menu_container=menu_container
            label="Database"
        />
    }
}

#[component]
fn ShareButton() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let click = move |_| {
        let Some(code) = state
            .editor()
            .read_untracked()
            .as_ref()
            .map(|editor| editor.get_value())
        else {
            return;
        };

        if let Ok(href) = window().location().href().and_then(|href| {
            let url = Url::new(&href)?;
            let params = url.search_params();
            params.set("code", &code);
            params.set("engine", state.engine().get_untracked().url_value());
            params.set(
                "keep_ctx",
                if state.keep_ctx().get_untracked() {
                    "1"
                } else {
                    "0"
                },
            );
            params.set(
                "read_only",
                if state.read_only().get_untracked() {
                    "1"
                } else {
                    "0"
                },
            );
            Ok(url.href())
        }) {
            state.share_href().set(Some(href));
        }

        change_focus(state, Some(Focus::Share));
    };

    view! { <Button on_click=click>"Share"</Button> }
}

#[component]
fn GithubButton() -> impl IntoView {
    view! { <LinkButton href="https://github.com/Spxg/sqlight".into() label="GitHub repository">{github_icon()}</LinkButton> }
}

fn import_database(state: Store<GlobalState>, event: Event) {
    if state.is_busy().get_untracked() {
        return;
    }
    let input = event_target::<HtmlInputElement>(&event);
    let Some(files) = input.files() else {
        return;
    };
    let files = (0..files.length())
        .map(|index| files.get(index).unwrap())
        .collect::<Vec<_>>();
    if files.is_empty() {
        return;
    }

    state.is_busy().set(true);
    let total = files.iter().map(|file| file.size()).sum();
    let filenames = files.iter().map(|file| file.name()).collect::<Vec<_>>();
    state.last_error().set(None);
    state.import_progress().set(Some(ImportProgress {
        filename: filenames.join(", "),
        loaded: 0.0,
        total,
        opened: None,
    }));
    spawn_local(async move {
        let imported = async {
            let mut imported = Vec::with_capacity(files.len());
            let mut names = HashSet::new();
            let mut loaded = 0.0;
            for file in files {
                let buffer = JsFuture::from(file.array_buffer()).await?;
                loaded += file.size();
                state.import_progress().update(|progress| {
                    if let Some(progress) = progress {
                        progress.loaded = loaded;
                    }
                });
                let filename = file.name();
                let data = js_sys::Uint8Array::new(&buffer);
                let is_zip = filename.to_ascii_lowercase().ends_with(".zip")
                    || (data.length() >= 4
                        && data.get_index(0) == 0x50
                        && data.get_index(1) == 0x4b
                        && matches!(
                            (data.get_index(2), data.get_index(3)),
                            (3, 4) | (5, 6) | (7, 8)
                        ));
                let databases = if is_zip {
                    unpack_zip(data).await?
                } else {
                    vec![DatabaseFile { filename, data }]
                };
                for database in databases {
                    if !names.insert(database.filename.clone()) {
                        return Err(wasm_bindgen::JsValue::from_str(&format!(
                            "Duplicate database filename: {}",
                            database.filename
                        )));
                    }
                    imported.push(database);
                }
            }

            state.import_progress().update(|progress| {
                if let Some(progress) = progress {
                    progress.filename = imported
                        .iter()
                        .map(|file| file.filename.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                }
            });
            let existing = state.databases().get_untracked();
            let collisions = imported
                .iter()
                .map(|file| file.filename.as_str())
                .filter(|name| existing.iter().any(|existing| existing == name))
                .collect::<Vec<_>>();
            let replace = !collisions.is_empty();
            if replace
                && !window()
                    .confirm_with_message(&format!(
                        "Replace existing databases: {}?",
                        collisions.join(", ")
                    ))
                    .unwrap_or(false)
            {
                return Ok(None);
            }
            Ok::<_, wasm_bindgen::JsValue>(Some(LoadDbsOptions {
                files: imported,
                replace,
            }))
        }
        .await;

        match imported {
            Ok(Some(options)) => send_request(state, WorkerRequest::LoadDbs(options)),
            Ok(None) => {
                state.is_busy().set(false);
                state.import_progress().set(None);
            }
            Err(error) => {
                state.is_busy().set(false);
                state.import_progress().update(|progress| {
                    if let Some(progress) = progress {
                        progress.opened = Some(false);
                    }
                });
                let message = error
                    .as_string()
                    .or_else(|| {
                        error
                            .dyn_ref::<js_sys::Error>()
                            .map(|error| error.message().into())
                    })
                    .unwrap_or_else(|| format!("{error:?}"));
                state
                    .last_error()
                    .set(Some(SendWrapper::new(SQLightError::ImportDb(message))));
            }
        }
    });
}
