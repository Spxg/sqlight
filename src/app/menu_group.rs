use istyles::istyles;
use leptos::prelude::*;

istyles!(styles, "assets/module.postcss/menu_group.module.css.map");

#[component]
pub fn MenuGroup(title: String, children: Children, #[prop(optional)] wide: bool) -> impl IntoView {
    let class = if wide {
        format!("{} {}", styles::container, styles::wide)
    } else {
        styles::container.into()
    };

    view! {
        <div class=class>
            <h2 class=styles::title>{title}</h2>
            <div class=styles::content>{children()}</div>
        </div>
    }
}
