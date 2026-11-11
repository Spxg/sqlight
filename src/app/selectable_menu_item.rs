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
    S: Fn() -> bool + Send + Sync + 'static,
    C: FnMut(MouseEvent) + Send + 'static,
{
    let selected = Memo::new(move |_| selected());

    view! {
        <MenuItem>
            <button
                type="button"
                class=move || { if selected.get() { styles::selected } else { styles::container } }
                aria-pressed=move || selected.get().to_string()
                on:click=on_click
            >
                <span class=styles::header>
                    <span class=styles::checkmark aria-hidden="true">{checkmark_icon()}</span>
                    <span class=styles::name>{name}</span>
                </span>
                <span class=styles::description>{children()}</span>
            </button>
        </MenuItem>
    }
}
