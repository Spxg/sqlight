use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/menu_aside.module.css.map");

#[component]
pub fn MenuAside(children: Children) -> impl IntoView {
    view! { <p class=styles::aside>{children()}</p> }
}
