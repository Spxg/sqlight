use std::{sync::Arc, time::Duration};

use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use crate::app::{GlobalState, GlobalStateStoreFields, icon::clipboard_icon};

istyles!(styles, "assets/module.postcss/output/share.module.css.map");

#[component]
fn Copied<S, H>(shared: S, href: H, children: Children) -> impl IntoView
where
    S: Fn() -> String + Send + Sync + 'static,
    H: Fn() -> String + Send + Sync + 'static,
{
    let (copied, set_copied) = signal(false);
    let shared = Arc::new(shared);

    let copy = move |_| {
        let shared = Arc::clone(&shared);
        spawn_local(async move {
            set_copied.set(true);
            if let Err(err) =
                JsFuture::from(window().navigator().clipboard().write_text(&shared())).await
            {
                log::error!("Failed to write href to clipboard: {err:?}");
            }
            set_timeout(move || set_copied.set(false), Duration::from_millis(1000));
        });
    };

    view! {
        <p class=move || { if *copied.read() { styles::active } else { styles::container } }>
            <a href=href target="_blank">
                {children()}
            </a>
            <button class=styles::button on:click=copy>
                {clipboard_icon()}
            </button>
            <span class=styles::text>"Copied!"</span>
        </p>
    }
}

#[component]
fn EmbeddedLinks() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let shared = move || state.share_href().get_untracked().unwrap_or_default();
    let href = move || state.share_href().get_untracked().unwrap_or_default();
    view! {
        <Copied shared=shared href=href>
            "Embedded code in link"
        </Copied>
    }
}

#[component]
pub fn Share() -> impl IntoView {
    view! {
        <>
            <EmbeddedLinks />
        </>
    }
}
