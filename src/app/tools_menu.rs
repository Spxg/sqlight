use leptos::prelude::*;
use reactive_stores::Store;
use sqlformat::{FormatOptions, QueryParams};

use crate::app::{
    GlobalState, GlobalStateStoreFields, button_menu_item::ButtonMenuItem, menu_aside::MenuAside,
    menu_group::MenuGroup,
};

#[component]
pub fn ToolsMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let format = move |_event| {
        let Some(editor) = &*state.editor().read() else {
            return;
        };

        let format_options = FormatOptions {
            uppercase: Some(true),
            lines_between_queries: 2,
            ..Default::default()
        };

        let sql = sqlformat::format(
            &editor.get_value(),
            &QueryParams::default(),
            &format_options,
        );

        editor.set_value(sql);
    };

    view! {
        <MenuGroup title="Tools".into()>
            <ButtonMenuItem name="SQL Format".into() on_click=format>
                <div>Format this code with sqlformat.</div>
                <MenuAside>"https://crates.io/crates/sqlformat"</MenuAside>
            </ButtonMenuItem>
        </MenuGroup>
    }
}
