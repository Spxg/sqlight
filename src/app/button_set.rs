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
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] aria_label: Signal<Option<String>>,
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

    let icon_left = icon_left.map(|icon| {
        view! { <span class=styles::iconLeft aria-hidden="true">{icon}</span> }
    });
    let icon_right = icon_right.map(|icon| {
        view! { <span class=styles::iconRight aria-hidden="true">{icon}</span> }
    });

    view! {
        <button
            type="button"
            class=class
            disabled=move || disabled.get()
            aria-label=move || aria_label.get()
            on:click=on_click
            node_ref=node_ref
        >
            {icon_left}
            {children()}
            {icon_right}
        </button>
    }
}

#[component]
pub fn LinkButton(href: String, label: &'static str, children: Children) -> impl IntoView {
    view! {
        <a class=styles::icon href=href aria-label=label target="_blank" rel="noopener noreferrer">
            {children()}
        </a>
    }
}
