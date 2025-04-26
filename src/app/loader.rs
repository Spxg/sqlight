use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/loader.module.css.map");

#[component]
pub fn Loader() -> impl IntoView {
    view! {
        <div>
            <span class=styles::dot>"⬤"</span>
            <span class=styles::dot>"⬤"</span>
            <span class=styles::dot>"⬤"</span>
        </div>
    }
}
