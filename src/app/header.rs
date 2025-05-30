use istyles::istyles;
use leptos::{html::Input, prelude::*, tachys::html};
use reactive_stores::Store;
use sqlformat::{FormatOptions, QueryParams};
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{Blob, Event, FileReader, HtmlInputElement, MouseEvent, Url, UrlSearchParams};

use crate::{
    FragileComfirmed, LoadDbOptions, RunOptions, SQLightError, WorkerRequest,
    app::{
        ImportProgress,
        advanced_options_menu::AdvancedOptionsMenu,
        button_set::{Button, ButtonSet, IconButton, LinkButton, Rule},
        config_menu::ConfigMenu,
        context_menu::ContextMenu,
        database_menu::DatabaseMenu,
        icon::{build_icon, config_icon, expandable_icon, github_icon, more_options_icon},
        output::change_focus,
        pop_button::PopButton,
        state::{Focus, GlobalState, GlobalStateStoreFields},
        tools_menu::ToolsMenu,
        vfs_menu::VfsMenu,
    },
    send_request,
};

istyles!(styles, "assets/module.postcss/header.module.css.map");

#[component]
pub fn Header() -> impl IntoView {
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
                        <VfsMenuButton menu_container=menu_container />
                        <Rule />
                        <ContextMenuButton menu_container=menu_container />
                        <Rule />
                        <AdvancedOptionsMenuButton menu_container=menu_container />
                    </ButtonSet>
                </div>
                <div class=styles::right>
                    <input type="file" node_ref=input_ref style="display: none" />

                    <ButtonSet>
                        <ShareButton />
                    </ButtonSet>
                    <ButtonSet>
                        <DatabaseButton input_ref=input_ref menu_container=menu_container />
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
        let editor_guard = state.editor().read_untracked();
        let Some(editor) = editor_guard.as_ref() else {
            return;
        };

        let (code, selected_code) = (editor.get_value(), editor.get_selected_value());

        drop(editor_guard);

        state.sql().set(code.clone());
        change_focus(state, Some(Focus::Execute));
        std::mem::take(&mut *state.output().write());

        let run_selected_code =
            !selected_code.is_empty() && state.run_selected_sql().get_untracked();

        send_request(
            state,
            WorkerRequest::Run(RunOptions {
                embed: false,
                sql: if run_selected_code {
                    selected_code
                } else {
                    code
                },
                clear_on_prepare: !*state.keep_ctx().read_untracked(),
            }),
        );
    })
}

#[component]
fn ExecuteButton() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let on_click = execute(state);

    view! {
        <Button is_primary=true icon_right=build_icon() on_click=move |_| on_click()>
            "Run"
        </Button>
    }
}

#[component]
fn VfsMenuButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let button = move |toggle, node_ref| {
        view! {
            <Button icon_right=expandable_icon() on_click=toggle node_ref=node_ref>
                {move || state.vfs().read().value()}
            </Button>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(|_close| { view! { <VfsMenu /> }.into_any() })
            menu_container=menu_container
        ></PopButton>
    }
}

#[component]
fn ContextMenuButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let button = move |toggle, node_ref| {
        view! {
            <Button icon_right=expandable_icon() on_click=toggle node_ref=node_ref>
                {move || if *state.keep_ctx().read() { "Keep Context" } else { "Drop Context" }}
            </Button>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(|_close| { view! { <ContextMenu /> }.into_any() })
            menu_container=menu_container
        ></PopButton>
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
fn AdvancedOptionsMenuButton(menu_container: NodeRef<html::element::Div>) -> impl IntoView {
    let button = |toggle, node_ref| {
        view! {
            <IconButton on_click=toggle node_ref=node_ref>
                {more_options_icon()}
            </IconButton>
        }
        .into_any()
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(|_close| { view! { <AdvancedOptionsMenu /> }.into_any() })
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
        let Some(editor) = &*state.editor().read() else {
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
                clear_on_prepare: !*state.keep_ctx().read_untracked(),
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

    let button = |toggle, node_ref| {
        view! {
            <Button icon_right=expandable_icon() on_click=toggle node_ref=node_ref>
                "Database"
            </Button>
        }
        .into_any()
    };

    let (file, set_file) = signal::<Option<FragileComfirmed<web_sys::File>>>(None);

    Effect::new(move || {
        if let Some(file) = &*file.read() {
            let filename = file.name();

            if let Ok(reader) = FileReader::new() {
                let on_progress = FragileComfirmed::new(Closure::wrap(Box::new(
                    move |ev: web_sys::ProgressEvent| {
                        if ev.length_computable() {
                            state.import_progress().set(Some(ImportProgress {
                                filename: filename.clone(),
                                loaded: ev.loaded(),
                                total: ev.total(),
                                opened: None,
                            }));
                        }
                    },
                )
                    as Box<dyn FnMut(_)>));

                let on_error = FragileComfirmed::new(Closure::wrap(Box::new(
                    move |ev: web_sys::ProgressEvent| {
                        let target = ev.target().unwrap();
                        let reader = target.unchecked_into::<FileReader>();
                        let dom_error = reader.error().unwrap();
                        state.last_error().set(Some(FragileComfirmed::new(
                            SQLightError::ImportDb(dom_error.message().to_string()),
                        )));
                    },
                )
                    as Box<dyn FnMut(_)>));

                let on_load =
                    FragileComfirmed::new(Closure::wrap(Box::new(move |ev: web_sys::Event| {
                        let target = ev.target().unwrap();
                        let reader = target.unchecked_into::<FileReader>();
                        let result = reader.result().unwrap();
                        let array_buffer = result.unchecked_into::<js_sys::ArrayBuffer>();
                        let data = js_sys::Uint8Array::new(&array_buffer);
                        send_request(state, WorkerRequest::LoadDb(LoadDbOptions { data }));
                    })
                        as Box<dyn FnMut(_)>));

                reader.set_onprogress(Some(on_progress.as_ref().unchecked_ref()));
                reader.set_onerror(Some(on_error.as_ref().unchecked_ref()));
                reader.set_onload(Some(on_load.as_ref().unchecked_ref()));
                reader.read_as_array_buffer(file).unwrap();

                on_cleanup(move || {
                    drop(on_progress);
                    drop(on_error);
                    drop(on_load);
                });
            }
        }
    });

    let on_change = move |ev: Event| {
        if let Some(target) = ev.target() {
            let input = target.unchecked_into::<HtmlInputElement>();
            if let Some(files) = input.files() {
                if files.length() > 0 {
                    let file = FragileComfirmed::new(files.get(0).unwrap());
                    set_file.set(Some(file));
                } else {
                    set_file.set(None);
                }
            }
        }
    };

    let on_load = move |_: MouseEvent, signal: WriteSignal<bool>| {
        if let Some(input) = &*input_ref.read() {
            if input.onchange().is_none() {
                let callback = Closure::wrap(Box::new(on_change) as Box<dyn Fn(Event)>);
                input.set_onchange(Some(callback.as_ref().unchecked_ref::<js_sys::Function>()));
                callback.forget();
            }
            input.set_value("");
            input.click();
        }
        signal.set(false);
    };

    Effect::new(move || {
        state.exported().track();

        let Some(downloaded) = state.exported().write_untracked().take() else {
            return;
        };
        let filename = downloaded.filename;
        let buffer = downloaded.data;
        let array = js_sys::Array::new();
        array.push(&buffer);

        let blob = Blob::new_with_u8_array_sequence(&array).unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();

        let document = document();
        let a = document
            .create_element("a")
            .unwrap()
            .dyn_into::<web_sys::HtmlAnchorElement>()
            .unwrap();

        a.set_href(&url);
        a.set_download(&filename);
        a.click();

        Url::revoke_object_url(&url).unwrap();
    });

    let on_download = move |_: MouseEvent, signal: WriteSignal<bool>| {
        send_request(state, WorkerRequest::DownloadDb);
        signal.set(false);
    };

    view! {
        <PopButton
            button=button
            menu=Box::new(move |signal| {
                view! {
                    <DatabaseMenu
                        load=move |e| on_load(e, signal)
                        download=move |e| on_download(e, signal)
                    />
                }
                    .into_any()
            })
            menu_container=menu_container
        ></PopButton>
    }
}

#[component]
fn ShareButton() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let click = move |_| {
        let Some(code) = state
            .editor()
            .read()
            .as_ref()
            .map(|editor| editor.get_value())
        else {
            return;
        };

        if let Ok(href) = window().location().href().and_then(|href| {
            let url = Url::new(&href)?;
            let params = UrlSearchParams::new()?;
            params.set("code", &code);
            url.set_search(&params.to_string().as_string().unwrap());
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
    view! { <LinkButton href="https://github.com/Spxg/sqlight".into()>{github_icon()}</LinkButton> }
}
