use istyles::istyles;
use leptos::prelude::*;
use web_sys::Event;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/config_element.module.css.map"
);

#[component]
pub fn Select<E>(
    on_change: E,
    name: String,
    #[prop(default = true)] is_default: bool,
    children: Children,
) -> impl IntoView
where
    E: FnMut(Event) + Send + 'static,
{
    view! {
        <ConfigElement name=name is_default=is_default>
            <select class=styles::select on:change=on_change>
                {children()}
            </select>
        </ConfigElement>
    }
}

#[component]
pub fn ConfigElement(
    name: String,
    #[prop(default = true)] is_default: bool,
    children: Children,
) -> impl IntoView {
    let style = if is_default {
        styles::name
    } else {
        styles::notDefault
    };
    view! {
        <MenuItem>
            <div class=styles::container>
                <span class=style>{name}</span>
                <div class=styles::value>{children()}</div>
            </div>
        </MenuItem>
    }
}
