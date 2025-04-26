use std::{sync::Arc, time::Duration};

use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use crate::app::{GlobalState, GlobalStateStoreFields, icon::clipboard_icon};

istyles!(styles, "assets/module.postcss/output/share.module.css.map");

#[component]
fn Copied<H>(href: H, children: Children) -> impl IntoView
where
    H: Fn() -> String + Send + Sync + 'static,
{
    let (copied, set_copied) = signal(false);
    let href = Arc::new(href);
    let href1 = Arc::clone(&href);

    let copy = move |_| {
        let href = Arc::clone(&href1);
        spawn_local(async move {
            set_copied.set(true);
            if let Err(err) =
                JsFuture::from(window().navigator().clipboard().write_text(&href())).await
            {
                log::error!("Failed to write href to clipboard: {err:?}");
            }
            set_timeout(move || set_copied.set(false), Duration::from_millis(1000));
        });
    };

    view! {
        <p class=move || { if *copied.read() { styles::active } else { styles::container } }>
            <a href=move || href()>{children()}</a>
            <button class=styles::button on:click=copy>
                {clipboard_icon()}
            </button>
            <span class=styles::text>"Copied!"</span>
        </p>
    }
}

#[component]
fn Links() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let code_url = move || state.share_href().get_untracked().unwrap_or_default();
    view! { <Copied href=code_url>Embedded code in link</Copied> }
}

#[component]
pub fn Share() -> impl IntoView {
    view! { <Links /> }
}
