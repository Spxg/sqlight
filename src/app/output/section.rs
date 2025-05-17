use istyles::istyles;
use leptos::prelude::*;

use crate::app::output::header::Header;

istyles!(
    styles,
    "assets/module.postcss/output/section.module.css.map"
);

#[component]
pub fn Section(label: String, children: Children) -> impl IntoView {
    view! {
        <div>
            <Header label=label />
            {children()}
        </div>
    }
}
