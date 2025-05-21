use std::{sync::Arc, time::Duration};

use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Blob, BlobPropertyBag, Url};

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
fn SQLWithResultLinks() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let shared = move || {
        state
            .share_sql_with_result()
            .get_untracked()
            .unwrap_or_default()
    };

    let href = move || {
        let text = state
            .share_sql_with_result()
            .get_untracked()
            .unwrap_or_default();
        let string_array = js_sys::Array::new();
        string_array.push(&JsValue::from(text));

        let blob_properties = BlobPropertyBag::new();
        blob_properties.set_type("text/plain");

        let blob =
            Blob::new_with_str_sequence_and_options(&string_array, &blob_properties).unwrap();

        let url = Url::create_object_url_with_blob(&blob).unwrap();
        let url1 = url.clone();

        set_timeout(
            move || Url::revoke_object_url(&url1).unwrap(),
            Duration::from_millis(5000),
        );

        url
    };

    view! {
        <Copied shared=shared href=href>
            "[Need run first] Copy sql with result"
        </Copied>
    }
}

#[component]
pub fn Share() -> impl IntoView {
    view! {
        <>
            <EmbeddedLinks />
            <SQLWithResultLinks />
        </>
    }
}
