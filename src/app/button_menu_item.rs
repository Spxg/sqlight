use istyles::istyles;
use leptos::prelude::*;
use web_sys::MouseEvent;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/button_menu_item.module.css.map"
);

#[component]
pub fn ButtonMenuItem<C>(
    name: String,
    #[prop(optional, into)] disabled: Signal<bool>,
    on_click: C,
    children: Children,
) -> impl IntoView
where
    C: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuItem>
            <button
                type="button"
                class=styles::container
                disabled=move || disabled.get()
                on:click=on_click
            >
                <span class=styles::name>{name}</span>
                <span class=styles::description>{children()}</span>
            </button>
        </MenuItem>
    }
}

#[component]
pub fn LinkMenuItem<C>(
    name: String,
    href: &'static str,
    on_click: C,
    children: Children,
) -> impl IntoView
where
    C: Fn(MouseEvent) + Send + 'static,
{
    view! {
        <MenuItem>
            <a
                class=styles::container
                href=href
                target="_blank"
                rel="noopener noreferrer"
                on:click=on_click
            >
                <span class=styles::name>{name}</span>
                <span class=styles::description>{children()}</span>
            </a>
        </MenuItem>
    }
}
