use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{button_menu_item::ButtonMenuItem, menu_aside::MenuAside, menu_group::MenuGroup};

#[component]
pub fn DatabaseMenu<L, D>(load: L, download: D, importing: Signal<bool>) -> impl IntoView
where
    L: Fn(MouseEvent) + Send + 'static,
    D: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuGroup title="Database".into()>
            <ButtonMenuItem name="Load".into() on_click=load disabled=importing>
                <MenuAside>"Import a SQLite database and enable Keep Context."</MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem name="Download".into() on_click=download>
                <MenuAside>"Save the current database as test.db."</MenuAside>
            </ButtonMenuItem>
        </MenuGroup>
    }
}
