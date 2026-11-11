use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, config_element::Toggle, menu_aside::MenuAside,
    menu_group::MenuGroup,
};

#[component]
pub fn AdvancedOptionsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let ciphers_hint = view! {
        <MenuAside>
            "Open encrypted databases with "
            <a
                href="https://utelle.github.io/SQLite3MultipleCiphers"
                target="_blank"
                rel="noopener noreferrer"
            >
                "SQLite3 Multiple Ciphers"
            </a>
            "."
        </MenuAside>
    }
    .into_any();

    view! {
        <MenuGroup title="Advanced options".into()>
            <Toggle
                id="run_selected_sql"
                name="Run Selected SQL".into()
                value=move || state.run_selected_sql().get()
                on_change=move |value| state.run_selected_sql().set(value)
            />
            <Toggle
                id="multiple_ciphers"
                name="Multiple Ciphers".into()
                value=move || state.multiple_ciphers().get()
                on_change=move |value| state.multiple_ciphers().set(value)
                aside=Some(ciphers_hint)
            />
        </MenuGroup>
    }
}
