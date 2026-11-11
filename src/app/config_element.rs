use istyles::istyles;
use leptos::prelude::*;
use web_sys::Event;

use crate::app::menu_item::MenuItem;

istyles!(
    styles,
    "assets/module.postcss/config_element.module.css.map"
);

#[component]
pub fn Toggle<E, V>(
    #[prop(default = None)] aside: Option<AnyView>,
    on_change: E,
    id: &'static str,
    value: V,
    name: String,
) -> impl IntoView
where
    E: Fn(bool) + Send + Sync + 'static,
    V: Fn() -> bool + Send + Sync + 'static,
{
    let value = Signal::derive(value);
    let on_change = Callback::new(on_change);
    let on_id = format!("{id}-on");
    let off_id = format!("{id}-off");

    view! {
        <ConfigElement
            name=name
            is_default=Box::new(move || !value.get())
            aside=aside
        >
            <div class=styles::toggle>
                <input
                    id=on_id.clone()
                    name=id
                    type="radio"
                    prop:checked=move || value.get()
                    on:change=move |_| on_change.run(true)
                />
                <label for=on_id>"On"</label>
                <input
                    id=off_id.clone()
                    name=id
                    type="radio"
                    prop:checked=move || !value.get()
                    on:change=move |_| on_change.run(false)
                />
                <label for=off_id>"Off"</label>
            </div>
        </ConfigElement>
    }
}

#[component]
pub fn Select<E, V>(
    on_change: E,
    value: V,
    name: String,
    #[prop(default = Box::new(|| true))] is_default: Box<dyn Fn() -> bool + Send>,
    children: Children,
) -> impl IntoView
where
    E: FnMut(Event) + Send + 'static,
    V: Fn() -> String + Send + 'static,
{
    let label = name.clone();
    view! {
        <ConfigElement name=name is_default=is_default>
            <select class=styles::select aria-label=label prop:value=value on:change=on_change>
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
