use istyles::istyles;
use leptos::{prelude::*, tachys::html};
use reactive_stores::Store;
use web_sys::{Url, UrlSearchParams};

use crate::{
    PrepareOptions, WorkerRequest,
    app::{
        advanced_options_menu::AdvancedOptionsMenu,
        button_set::{Button, ButtonSet, IconButton, Rule},
        config_menu::ConfigMenu,
        context_menu::ContextMenu,
        icon::{build_icon, config_icon, expandable_icon, more_options_icon},
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
                    <ButtonSet>
                        <ShareButton />
                    </ButtonSet>
                    <ButtonSet>
                        <ConfigMenuButton menu_container=menu_container />
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

        let run_selected_code = state.run_selected_code().get();

        state.code().set(code.clone());
        change_focus(state, Some(Focus::Execute));
        std::mem::take(&mut *state.output().write());

        if let Some(worker) = &*state.worker().read_untracked() {
            worker.send_task(WorkerRequest::Prepare(PrepareOptions {
                id: String::new(),
                sql: if !selected_code.is_empty() && run_selected_code {
                    selected_code
                } else {
                    code
                },
                clear_on_prepare: !*state.keep_ctx().read_untracked(),
            }));
            worker.send_task(WorkerRequest::Continue(String::new()));
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

        if let Ok(href) = window().location().href().and_then(|href| {
            let url = Url::new(&href)?;
            let params = UrlSearchParams::new()?;
            params.set("code", &code);
            url.set_search(&params.to_string().as_string().unwrap());
            Ok(url.href())
        }) {
            state.share_href().set(Some(href));
            change_focus(state, Some(Focus::Share));
        }
    };

    view! { <Button on_click=click>"Share"</Button> }
}
