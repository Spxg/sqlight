use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, config_element::Either, menu_group::MenuGroup,
};

#[component]
pub fn AdvancedOptionsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let value = move || *state.run_selected_code().read();
    let is_default = move || !*state.run_selected_code().read();
    let on_change = move |value: &bool| {
        state.run_selected_code().set(*value);
    };

    view! {
        <>
            <MenuGroup title="Advanced options".into()>
                <Either
                    id="run_selected_code".into()
                    name="Run Selected Code".into()
                    a=true
                    b=false
                    a_label=Some("On".to_string())
                    b_label=Some("Off".to_string())
                    value=value
                    is_default=Box::new(is_default)
                    on_change=on_change
                />
            </MenuGroup>
        </>
    }
}
