use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, menu_group::MenuGroup, select_one::SelectOne,
};

#[component]
pub fn ContextMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <MenuGroup title="Choose whether to keep the context".into()>
            <SelectOne
                name="Drop Context".into()
                current_value=move || { *state.keep_ctx().read() }
                this_value=false
                change_value=move || {
                    state.keep_ctx().set(false);
                }
            >
                "Each execution is in a new DB."
            </SelectOne>
            <SelectOne
                name="Keep Context".into()
                current_value=move || { *state.keep_ctx().read() }
                this_value=true
                change_value=move || {
                    state.keep_ctx().set(true);
                }
            >
                "Keep the results of each execution."
            </SelectOne>
        </MenuGroup>
    }
}
