use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/output/header.module.css.map");

#[component]
pub fn Header(label: String) -> impl IntoView {
    view! { <span class=styles::container>{label}</span> }
}
