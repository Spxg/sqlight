mod execute;
mod header;
mod loader;
mod section;
mod share;
mod status;

use execute::Execute;
use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use share::Share;
use status::Status;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, KeyboardEvent};

use crate::app::state::{Focus, GlobalState, GlobalStateStoreFields};

istyles!(styles, "assets/module.postcss/output.module.css.map");

const TABS: [(Focus, &str); 3] = [
    (Focus::Execute, "Execution"),
    (Focus::Share, "Share"),
    (Focus::Status, "Status"),
];

fn tab_id(kind: Focus) -> &'static str {
    match kind {
        Focus::Execute => "execution-tab",
        Focus::Share => "share-tab",
        Focus::Status => "status-tab",
    }
}

#[component]
fn Tab(kind: Focus, label: &'static str) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let selected = move || state.focus().get() == Some(kind);
    let is_tab_stop = move || {
        state.focus().get().or_else(|| {
            let opened = state.opened_focus().read();
            TABS.iter()
                .map(|&(focus, _)| focus)
                .find(|focus| opened.contains(focus))
        }) == Some(kind)
    };

    let on_keydown = move |event: KeyboardEvent| {
        let opened = state.opened_focus().read_untracked();
        let tabs = TABS
            .iter()
            .map(|&(focus, _)| focus)
            .filter(|focus| opened.contains(focus))
            .collect::<Vec<_>>();
        let index = tabs.iter().position(|&focus| focus == kind).unwrap();
        let next = match event.key().as_str() {
            "ArrowLeft" => tabs[(index + tabs.len() - 1) % tabs.len()],
            "ArrowRight" => tabs[(index + 1) % tabs.len()],
            "Home" => tabs[0],
            "End" => tabs[tabs.len() - 1],
            _ => return,
        };
        drop(opened);

        event.prevent_default();
        change_focus(state, Some(next));
        if let Some(tab) = document().get_element_by_id(tab_id(next)) {
            let _ = tab.unchecked_into::<HtmlElement>().focus();
        }
    };

    view! {
        <Show when=move || state.opened_focus().read().contains(&kind)>
            <button
                type="button"
                id=tab_id(kind)
                role="tab"
                aria-controls="output-panel"
                aria-selected=move || selected().to_string()
                tabindex=move || if is_tab_stop() { 0 } else { -1 }
                class=move || if selected() { styles::tabSelected } else { styles::tab }
                on:click=move |_| change_focus(state, Some(kind))
                on:keydown=on_keydown
            >
                {label}
            </button>
        </Show>
    }
}

#[component]
pub fn Output() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <Show when=move || state.show_something().get()>
            <div class=styles::container>
                <div class=styles::tabs>
                    <div class=styles::tabList role="tablist" aria-label="Output">
                        {TABS
                            .into_iter()
                            .map(|(kind, label)| view! { <Tab kind=kind label=label /> })
                            .collect_view()}
                    </div>
                    <Show when=move || state.focus().read().is_some()>
                        <button
                            type="button"
                            class=styles::tabClose
                            aria-label="Collapse output"
                            on:click=move |_| change_focus(state, None)
                        >
                            "Close"
                        </button>
                    </Show>
                </div>
                <Show when=move || state.focus().read().is_some()>
                    <div
                        id="output-panel"
                        class=styles::body
                        role="tabpanel"
                        tabindex="0"
                        aria-labelledby=move || state.focus().get().map(tab_id)
                    >
                        {move || match state.focus().get() {
                            Some(Focus::Execute) => view! { <Execute /> }.into_any(),
                            Some(Focus::Share) => view! { <Share /> }.into_any(),
                            Some(Focus::Status) => view! { <Status /> }.into_any(),
                            None => ().into_any(),
                        }}
                    </div>
                </Show>
            </div>
        </Show>
    }
}

pub fn change_focus(state: Store<GlobalState>, focus: Option<Focus>) {
    batch(move || {
        if let Some(focus) = focus {
            state
                .opened_focus()
                .maybe_update(|opened| opened.insert(focus));
        }
        if state.focus().get_untracked() != focus {
            state.focus().set(focus);
        }
        if !state.show_something().get_untracked() {
            state.show_something().set(true);
        }
    });
}
