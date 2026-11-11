use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use crate::app::{GlobalState, GlobalStateStoreFields, icon::clipboard_icon};

istyles!(styles, "assets/module.postcss/output/share.module.css.map");

#[component]
pub fn Share() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let (copy_status, set_copy_status) = signal("");

    Effect::new(move || {
        state.share_href().track();
        set_copy_status.set("");
    });

    let copy = move |_| {
        let Some(href) = state.share_href().get_untracked() else {
            return;
        };

        set_copy_status.set("Copying…");
        spawn_local(async move {
            let result = JsFuture::from(window().navigator().clipboard().write_text(&href)).await;
            let _ = set_copy_status.try_set(if result.is_ok() {
                "Copied!"
            } else {
                "Copy failed. Copy the link directly."
            });
        });
    };

    view! {
        <Show when=move || state.share_href().read().is_some()>
            <p class=styles::container>
                <a
                    href=move || state.share_href().get()
                    target="_blank"
                    rel="noopener noreferrer"
                >
                    "Embedded code in link"
                </a>
                <button
                    type="button"
                    class=styles::button
                    aria-label="Copy share link"
                    disabled=move || copy_status.get() == "Copying…"
                    on:click=copy
                >
                    <span aria-hidden="true">{clipboard_icon()}</span>
                </button>
                <span role="status">{copy_status}</span>
            </p>
        </Show>
    }
}
