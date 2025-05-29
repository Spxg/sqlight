use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{button_menu_item::ButtonMenuItem, menu_aside::MenuAside, menu_group::MenuGroup};

#[component]
pub fn DatabaseMenu<L, D>(load: L, download: D) -> impl IntoView
where
    L: Fn(MouseEvent) + Send + 'static,
    D: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuGroup title="Database".into()>
            <ButtonMenuItem name="Load".into() on_click=load>
                <MenuAside>"The KEEP CONTEXT will be automatically enabled."</MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem name="Download".into() on_click=download>
                <MenuAside>"Will be downloaded as test.db."</MenuAside>
            </ButtonMenuItem>
        </MenuGroup>
    }
}
