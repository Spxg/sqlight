use istyles::istyles;
use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/button_menu_item.module.css.map"
);

#[component]
pub fn ButtonMenuItem<C>(name: String, on_click: C, children: Children) -> impl IntoView
where
    C: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuItem>
            <button class=styles::container on:click=on_click>
                <div class=styles::name>{name}</div>
                <div class=styles::description>{children()}</div>
            </button>
        </MenuItem>
    }
}
