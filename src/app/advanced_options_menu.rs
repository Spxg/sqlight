use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    Engine, GlobalState, GlobalStateStoreFields,
    config_element::{Select, Toggle},
    menu_group::MenuGroup,
};

#[component]
pub fn AdvancedOptionsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    view! {
        <MenuGroup title="Advanced options".into()>
            <Toggle
                id="run_selected_sql"
                name="Run Selected SQL".into()
                value=move || state.run_selected_sql().get()
                on_change=move |value| state.run_selected_sql().set(value)
            />
            <Select
                name="SQLite Engine".into()
                value=move || state.engine().get().value().to_owned()
                is_default=Box::new(move || state.engine().get() == Engine::SQLite)
                on_change=move |event| {
                    state.engine().set(Engine::from_select(&event_target_value(&event)));
                }
            >
                {[Engine::SQLite, Engine::MultipleCiphers, Engine::SQLCipher]
                    .into_iter()
                    .map(|engine| {
                        let value = engine.value();
                        view! { <option value=value>{value}</option> }
                    })
                    .collect_view()}
            </Select>
        </MenuGroup>
    }
}
