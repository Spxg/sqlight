use leptos::prelude::*;

#[component]
pub fn SimplePane(children: Children) -> impl IntoView {
    view! { <div>{children()}</div> }
}
