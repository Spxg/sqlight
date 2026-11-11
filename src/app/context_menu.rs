use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, menu_group::MenuGroup, select_one::SelectOne,
};

#[component]
pub fn ContextMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <MenuGroup title="Database context".into()>
            <SelectOne
                name="Drop Context".into()
                current_value=move || { *state.keep_ctx().read() }
                this_value=false
                change_value=move || {
                    state.keep_ctx().set(false);
                }
            >
                "Start each run with an empty database."
            </SelectOne>
            <SelectOne
                name="Keep Context".into()
                current_value=move || { *state.keep_ctx().read() }
                this_value=true
                change_value=move || {
                    state.keep_ctx().set(true);
                }
            >
                "Keep tables and data between runs."
            </SelectOne>
        </MenuGroup>
    }
}
