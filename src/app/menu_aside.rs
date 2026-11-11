use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/menu_aside.module.css.map");

#[component]
pub fn MenuAside(children: Children) -> impl IntoView {
    view! { <span class=styles::aside>{children()}</span> }
}
