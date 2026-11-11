use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{
    button_menu_item::{ButtonMenuItem, LinkMenuItem},
    menu_aside::MenuAside,
    menu_group::MenuGroup,
};

#[component]
pub fn ToolsMenu<F, E, I>(
    on_format: F,
    on_embed: E,
    on_internal: I,
    busy: Signal<bool>,
) -> impl IntoView
where
    F: Fn(MouseEvent) + Send + 'static,
    E: Fn(MouseEvent) + Send + 'static,
    I: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuGroup title="Tools".into()>
            <ButtonMenuItem name="Format SQL".into() on_click=on_format>
                <MenuAside>"Format SQL with uppercase keywords."</MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem name="Embed Query Result".into() on_click=on_embed disabled=busy>
                <MenuAside>"Run SQL and insert the results as comments for easy sharing."</MenuAside>
            </ButtonMenuItem>
            <LinkMenuItem
                name="SQLite internals".into()
                href="https://sqlite-internal.pages.dev"
                on_click=on_internal
            >
                <MenuAside>
                    "Explore the SQLite file format. Encrypted databases are not supported."
                </MenuAside>
            </LinkMenuItem>
        </MenuGroup>
    }
}
