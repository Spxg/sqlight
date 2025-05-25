use istyles::istyles;
use leptos::{html::Input, prelude::*, tachys::html};
use prettytable::{Cell, Row, Table};
use reactive_stores::Store;
use wasm_bindgen::{JsCast, prelude::Closure};
use web_sys::{Blob, Event, FileReader, HtmlInputElement, Url, UrlSearchParams};

use crate::{
    FragileComfirmed, LoadDbOptions, PrepareOptions, SQLightError, SQLiteStatementResult,
    WorkerRequest,
    app::{
        ImportProgress,
        advanced_options_menu::AdvancedOptionsMenu,
        button_set::{Button, ButtonSet, IconButton, LinkButton, Rule},
        config_menu::ConfigMenu,
        context_menu::ContextMenu,
        icon::{build_icon, config_icon, expandable_icon, github_icon, more_options_icon},
        output::change_focus,
        pop_button::PopButton,
        state::{Focus, GlobalState, GlobalStateStoreFields},
        vfs_menu::VfsMenu,
    },
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
                        <LoadButton input_ref=input_ref />
                        <Rule />
                        <DownloadButton />
                    </ButtonSet>
                    <ButtonSet>
                        <ShareButton />
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
        let Some((code, selected_code)) = state
            .editor()
            .read_untracked()
            .as_ref()
            .map(|editor| (editor.get_value(), editor.get_selected_value()))
        else {
            return;
        };

        let run_selected_code = state.run_selected_sql().get();

        state.sql().set(code.clone());
        change_focus(state, Some(Focus::Execute));
        std::mem::take(&mut *state.output().write());

        if let Some(worker) = &*state.worker().read_untracked() {
            worker.send_task(WorkerRequest::Prepare(PrepareOptions {
                sql: if !selected_code.is_empty() && run_selected_code {
                    selected_code
                } else {
                    code
                },
                clear_on_prepare: !*state.keep_ctx().read_untracked(),
            }));
            worker.send_task(WorkerRequest::Continue);
        }
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
fn DownloadButton() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

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

    let on_click = move |_| {
        if let Some(worker) = &*state.worker().read() {
            worker.send_task(WorkerRequest::DownloadDb);
        }
    };

    view! { <Button on_click=on_click>"Download DB"</Button> }
}

#[component]
fn LoadButton(input_ref: NodeRef<Input>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

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
                        if let Some(worker) = &*state.worker().read() {
                            worker.send_task(WorkerRequest::LoadDb(LoadDbOptions { data }));
                        }
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

    let on_click = move |_| {
        if let Some(input) = &*input_ref.read() {
            if input.onchange().is_none() {
                let callback = Closure::wrap(Box::new(on_change) as Box<dyn Fn(Event)>);
                input.set_onchange(Some(callback.as_ref().unchecked_ref::<js_sys::Function>()));
                callback.forget();
            }
            input.set_value("");
            input.click();
        }
    };

    view! { <Button on_click=on_click>"Load DB"</Button> }
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
                {move || if *state.keep_ctx().read() { "Keep Context" } else { "Discard Context" }}
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

        let mut sqls = vec![];

        for result in &*state.output().read() {
            let mut table_s = Table::new();

            match result {
                SQLiteStatementResult::Finish => continue,
                SQLiteStatementResult::Step(table) => {
                    let mut sql = table.sql.trim().to_string();

                    if let Some(values) = &table.values {
                        table_s.add_row(Row::new(
                            values.columns.iter().map(|s| Cell::new(s)).collect(),
                        ));
                        for row in &values.rows {
                            table_s.add_row(Row::new(row.iter().map(|s| Cell::new(s)).collect()));
                        }

                        let result = table_s
                            .to_string()
                            .lines()
                            .map(|x| format!("-- {x}"))
                            .collect::<Vec<String>>()
                            .join("\n");

                        sql.push('\n');
                        sql.push_str(&result);
                    }

                    sqls.push(sql);
                }
            }
        }

        state.share_sql_with_result().set(Some(sqls.join("\n\n")));

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
