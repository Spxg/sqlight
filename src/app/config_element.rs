use istyles::istyles;
use leptos::prelude::*;
use web_sys::Event;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/config_element.module.css.map"
);

#[component]
pub fn Select<E, V>(
    on_change: E,
    value: V,
    name: String,
    #[prop(default = None)] aside: Option<AnyView>,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
    children: Children,
) -> impl IntoView
where
    E: FnMut(Event) + Send + 'static,
    V: Fn() -> String + Send + 'static,
{
    let label = name.clone();
    view! {
        <ConfigElement name=name is_default=is_default aside=aside>
            <select
                class=styles::select
                aria-label=label
                prop:value=value
                disabled=move || disabled.get()
                on:change=on_change
            >
                {children()}
            </select>
        </ConfigElement>
    }
}

#[component]
pub fn ConfigElement(
    name: String,
    #[prop(default = None)] aside: Option<AnyView>,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
    children: Children,
) -> impl IntoView {
    let label = name.clone();
    let style = move || {
        if is_default() {
            styles::name
        } else {
            styles::notDefault
        }
    };
    view! {
        <MenuItem>
            <div class=styles::container role="group" aria-label=label>
                <span class=style>{name}</span>
                <div class=styles::value>{children()}</div>
            </div>
            {aside}
        </MenuItem>
    }
}
