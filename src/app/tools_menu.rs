use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{button_menu_item::ButtonMenuItem, menu_aside::MenuAside, menu_group::MenuGroup};

#[component]
pub fn ToolsMenu<F, E, I>(on_format: F, on_embed: E, on_internal: I) -> impl IntoView
where
    F: Fn(MouseEvent) + Send + 'static,
    E: Fn(MouseEvent) + Send + 'static,
    I: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuGroup title="Tools".into()>
            <ButtonMenuItem name="SQL Format".into() on_click=on_format>
                <MenuAside>"https://crates.io/crates/sqlformat"</MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem name="Embed Query Result".into() on_click=on_embed>
                <MenuAside>"Embed results into query statements for easy sharing."</MenuAside>
            </ButtonMenuItem>
            <a href="https://sqlite-internal.pages.dev" target="_blank">
                <ButtonMenuItem name="SQLite internal".into() on_click=on_internal>
                    <MenuAside>
                        "This tool helps you explore the SQLite file format internals."
                    </MenuAside>
                </ButtonMenuItem>
            </a>
        </MenuGroup>
    }
}
