use leptos::prelude::*;

use crate::app::{loader::Loader as GenericLoader, output::header::Header};

#[component]
pub fn Loader() -> impl IntoView {
    view! {
        <div>
            <Header label="Progress".into() />
            <GenericLoader />
        </div>
    }
}
