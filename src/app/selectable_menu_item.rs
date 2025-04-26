use istyles::istyles;
use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::{icon::checkmark_icon, menu_item::MenuItem};

istyles!(
    styles,
    "assets/module.postcss/selectable_menu_item.module.css.map"
);

#[component]
pub fn SelectableMenuItem<S, C>(
    name: String,
    selected: S,
    on_click: C,
    children: Children,
) -> impl IntoView
where
    S: Fn() -> bool + Send + 'static,
    C: FnMut(MouseEvent) + Send + 'static,
{
    view! {
        <MenuItem>
            <button
                class=move || { if selected() { styles::selected } else { styles::container } }
                on:click=on_click
            >
                <div class=styles::header>
                    <span class=styles::checkmark>{checkmark_icon()}</span>
                    <span class=styles::name>{name}</span>
                </div>
                <div class=styles::description>{children()}</div>
            </button>
        </MenuItem>
    }
}
