use leptos::prelude::*;
use reactive_stores::Store;

use crate::app::{
    GlobalState, GlobalStateStoreFields, Vfs, menu_group::MenuGroup, select_one::SelectOne,
};

#[component]
pub fn VfsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <MenuGroup title="Choose SQLite VFS".into()>
            <SelectOne
                name="Memory".into()
                current_value=move || { *state.vfs().read() }
                this_value=Vfs::Memory
                change_value=move || {
                    *state.vfs().write() = Vfs::Memory;
                }
            >
                "Data will be lost after refreshing."
            </SelectOne>
            <SelectOne
                name="OPFS".into()
                current_value=move || { *state.vfs().read() }
                this_value=Vfs::OPFS
                change_value=move || {
                    *state.vfs().write() = Vfs::OPFS;
                }
            >
                "Persistent Storage."
            </SelectOne>
        </MenuGroup>
    }
}
