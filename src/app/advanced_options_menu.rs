use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, config_element::Either, menu_group::MenuGroup,
};

#[component]
pub fn AdvancedOptionsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <>
            <MenuGroup title="Advanced options".into()>
                <Either
                    id="run_selected_sql".into()
                    name="Run Selected SQL".into()
                    a=true
                    b=false
                    a_label=Some("On".to_string())
                    b_label=Some("Off".to_string())
                    value=move || *state.run_selected_sql().read()
                    is_default=Box::new(move || !*state.run_selected_sql().read())
                    on_change=move |value: &bool| {
                        state.run_selected_sql().set(*value);
                    }
                />
            </MenuGroup>
        </>
    }
}
