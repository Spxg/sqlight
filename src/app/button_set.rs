use istyles::istyles;
use leptos::{prelude::*, tachys::html};
use web_sys::MouseEvent;

istyles!(styles, "assets/module.postcss/button_set.module.css.map");

#[component]
pub fn ButtonSet(
    #[prop(default = String::new())] class_name: String,
    children: Children,
) -> impl IntoView {
    let class = format!("{} {}", styles::set, class_name);

    view! { <div class=class>{children()}</div> }
}

#[component]
pub fn Button<C>(
    #[prop(default = false)] is_primary: bool,
    #[prop(default = false)] is_small: bool,
    #[prop(optional)] icon_left: Option<AnyView>,
    #[prop(optional)] icon_right: Option<AnyView>,
    #[prop(optional)] node_ref: NodeRef<html::element::Button>,
    on_click: C,
    children: Children,
) -> impl IntoView
where
    C: FnMut(MouseEvent) + Send + 'static,
{
    let class = format!(
        "{} {}",
        if is_primary {
            styles::primary
        } else {
            styles::secondary
        },
        if is_small { styles::small } else { "" }
    );

    let icon_left = move || {
        if let Some(icon) = icon_left {
            view! { <span class=styles::iconLeft>{icon}</span> }.into_any()
        } else {
            ().into_any()
        }
    };

    let icon_right = move || {
        if let Some(icon) = icon_right {
            view! { <span class=styles::iconRight>{icon}</span> }.into_any()
        } else {
            ().into_any()
        }
    };

    view! {
        <button type="button" class=class on:click=on_click node_ref=node_ref>
            {icon_left()}
            {children()}
            {icon_right()}
        </button>
    }
}

#[component]
pub fn Rule() -> impl IntoView {
    view! { <span class=styles::rule /> }
}

#[component]
pub fn IconButton<C>(
    #[prop(default = false)] is_small: bool,
    #[prop(optional)] node_ref: NodeRef<html::element::Button>,
    on_click: C,
    children: Children,
) -> impl IntoView
where
    C: FnMut(MouseEvent) + Send + 'static,
{
    let style = if is_small { styles::small } else { "" };
    let style = format!("{} {style}", styles::icon);
    view! {
        <button node_ref=node_ref class=style on:click=on_click>
            {children()}
        </button>
    }
}
