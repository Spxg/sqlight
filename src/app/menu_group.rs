use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/menu_group.module.css.map");

#[component]
pub fn MenuGroup(title: String, children: Children) -> impl IntoView {
    view! {
        <div class=styles::container>
            <h1 class=styles::title>{title}</h1>
            <div class=styles::content>{children()}</div>
        </div>
    }
}
