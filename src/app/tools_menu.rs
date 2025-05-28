use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{button_menu_item::ButtonMenuItem, menu_aside::MenuAside, menu_group::MenuGroup};

#[component]
pub fn ToolsMenu<F, E>(on_format: F, on_embed: E) -> impl IntoView
where
    F: Fn(MouseEvent) + Send + 'static,
    E: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuGroup title="Tools".into()>
            <ButtonMenuItem name="SQL Format".into() on_click=on_format>
                <MenuAside>"https://crates.io/crates/sqlformat"</MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem name="Embed Query Result".into() on_click=on_embed>
                <MenuAside>"https://crates.io/crates/prettytable-rs"</MenuAside>
            </ButtonMenuItem>
        </MenuGroup>
    }
}
