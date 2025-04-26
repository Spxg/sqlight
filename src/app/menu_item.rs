use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/menu_item.module.css.map");

#[component]
pub fn MenuItem(children: Children) -> impl IntoView {
    view! { <div class=styles::container>{children()}</div> }
}
