use std::sync::Arc;

use istyles::istyles;
use leptos::prelude::*;
use web_sys::Event;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/config_element.module.css.map"
);

#[component]
pub fn Either<E, V, T>(
    #[prop(default =None)] aside: Option<AnyView>,
    on_change: E,
    id: String,
    a: T,
    b: T,
    a_label: Option<String>,
    b_label: Option<String>,
    value: V,
    name: String,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
) -> impl IntoView
where
    E: Fn(&T) + Send + Sync + 'static,
    V: Fn() -> T + Send + Sync + 'static,
    T: PartialEq + Eq + Send + Sync + ToString + 'static,
{
    let a_label = if let Some(label) = a_label {
        label
    } else {
        a.to_string()
    };

    let b_label = if let Some(label) = b_label {
        label
    } else {
        b.to_string()
    };

    let a_id = format!("{id}-a");
    let b_id = format!("{id}-b");

    let on_change1 = Arc::new(on_change);
    let on_change2 = Arc::clone(&on_change1);

    let value1 = Arc::new(value);
    let value2 = Arc::clone(&value1);

    let a_value = a.to_string();
    let b_value = b.to_string();

    let a1 = Arc::new(a);
    let a2 = Arc::clone(&a1);
    let b1 = Arc::new(b);
    let b2 = Arc::clone(&b1);

    view! {
        <ConfigElement name=name is_default=is_default aside>
            <div class=styles::toggle>
                <input
                    id=a_id.clone()
                    name=id.clone()
                    value=a_value
                    type="radio"
                    checked=move || value1().eq(&a1)
                    on:change=move |_ev| on_change1(&a2)
                />
                <label for=a_id>{a_label}</label>
                <input
                    id=b_id.clone()
                    name=id.clone()
                    value=b_value
                    type="radio"
                    checked=move || value2().eq(&b1)
                    on:change=move |_ev| on_change2(&b2)
                />
                <label for=b_id>{b_label}</label>
            </div>
        </ConfigElement>
    }
}

#[component]
pub fn Select<E>(
    on_change: E,
    name: String,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
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
    #[prop(default =None)] aside: Option<AnyView>,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
    children: Children,
) -> impl IntoView {
    let style = move || {
        if is_default() {
            styles::name
        } else {
            styles::notDefault
        }
    };
    view! {
        <MenuItem>
            <div class=styles::container>
                <span class=style>{name}</span>
                <div class=styles::value>{children()}</div>
            </div>
            {aside}
        </MenuItem>
    }
}
