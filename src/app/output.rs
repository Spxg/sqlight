mod execute;
mod header;
mod loader;
mod section;
mod share;
mod simple_pane;
mod status;

use execute::Execute;
use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use share::Share;
use status::Status;

use crate::app::state::{Focus, GlobalState, GlobalStateStoreFields};

istyles!(styles, "assets/module.postcss/output.module.css.map");

fn close() -> AnyView {
    let state = expect_context::<Store<GlobalState>>();

    if state.read().is_focus() {
        view! {
            <button class=styles::tabClose on:click=move |_| change_focus(state, None)>
                "Close"
            </button>
        }
        .into_any()
    } else {
        ().into_any()
    }
}

fn body() -> AnyView {
    let state = expect_context::<Store<GlobalState>>();

    if state.read().is_focus() {
        view! {
            <>
                <div class=styles::body>
                    <Show
                        when=move || matches!(*state.focus().read(), Some(Focus::Execute))
                        fallback=|| ()
                    >
                        <Execute />
                    </Show>

                    <Show
                        when=move || matches!(*state.focus().read(), Some(Focus::Share))
                        fallback=|| ()
                    >
                        <Share />
                    </Show>

                    <Show
                        when=move || matches!(*state.focus().read(), Some(Focus::Status))
                        fallback=|| ()
                    >
                        <Status />
                    </Show>
                </div>
            </>
        }
        .into_any()
    } else {
        ().into_any()
    }
}

#[component]
pub fn Tab(kind: Focus, label: String) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let class = move || {
        if matches!(*state.focus().read(), Some(focus) if focus == kind) {
            styles::tabSelected
        } else {
            styles::tab
        }
    };

    view! {
        <Show when=move || state.opened_focus().read().contains(&kind) fallback=|| ()>
            <button class=class>{label.clone()}</button>
        </Show>
    }
}

#[component]
pub fn Output() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <Show when=move || *state.show_something().read() fallback=|| ()>
            <div class=styles::container>
                <div class=styles::tabs>
                    <Tab
                        kind=Focus::Execute
                        label="Execution".into()
                        on:click=move |_| change_focus(state, Some(Focus::Execute))
                    />

                    <Tab
                        kind=Focus::Share
                        label="Share".into()
                        on:click=move |_| change_focus(state, Some(Focus::Share))
                    />

                    <Tab
                        kind=Focus::Status
                        label="Status".into()
                        on:click=move |_| change_focus(state, Some(Focus::Status))
                    />
                    {close}
                </div>
                {body}
            </div>
        </Show>
    }
}

pub fn change_focus(state: Store<GlobalState>, focus: Option<Focus>) {
    if let Some(focus) = focus {
        state.opened_focus().write().insert(focus);
        state
            .is_focused()
            .maybe_update(|before| !std::mem::replace(before, true));
    } else {
        state
            .is_focused()
            .maybe_update(|before| std::mem::replace(before, false));
    }

    state
        .focus()
        .maybe_update(|before| std::mem::replace(before, focus) != focus);
    state
        .show_something()
        .maybe_update(|before| !std::mem::replace(before, true));
}
