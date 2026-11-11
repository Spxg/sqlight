use std::{ops::Deref, sync::Arc};

use floating_ui::{auto_update, compute_options, compute_position};
use istyles::istyles;
use leptos::{ev, portal::Portal, prelude::*, tachys::html};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;

use crate::SendWrapper;

istyles!(styles, "assets/module.postcss/pop_button.module.css.map");

#[component]
pub fn PopButton<B, M>(
    button: B,
    menu: M,
    #[prop(optional)] menu_container: NodeRef<html::element::Div>,
) -> impl IntoView
where
    B: FnOnce(Box<dyn FnMut(MouseEvent) + Send>, NodeRef<html::element::Button>) -> AnyView,
    M: Fn(WriteSignal<bool>) -> AnyView + Send + Sync + 'static,
{
    let (is_open, set_open) = signal(false);
    let reference_ref = NodeRef::<html::element::Button>::new();
    let floating_ref = NodeRef::<html::element::Div>::new();
    let arrow_ref = NodeRef::<html::element::Div>::new();
    let content_ref = NodeRef::<html::element::Div>::new();

    let keyboard = window_event_listener(ev::keydown, move |event| {
        if is_open.get_untracked() && event.key() == "Escape" {
            set_open.set(false);
            if let Some(button) = reference_ref.get_untracked() {
                button.focus().unwrap();
            }
        }
    });
    let outside_click = window_event_listener(ev::click, move |event| {
        if !is_open.get_untracked() {
            return;
        }
        let target = event
            .target()
            .and_then(|target| target.dyn_into::<web_sys::Node>().ok());
        let inside_button = reference_ref
            .get_untracked()
            .is_some_and(|button| button.contains(target.as_ref()));
        let inside_popup = floating_ref
            .get_untracked()
            .is_some_and(|popup| popup.contains(target.as_ref()));
        if !inside_button && !inside_popup {
            set_open.set(false);
        }
    });
    on_cleanup(move || {
        keyboard.remove();
        outside_click.remove();
    });

    Effect::new(move || {
        let open = is_open.get();
        if let Some(button) = reference_ref.get() {
            button.set_attribute("aria-haspopup", "dialog").unwrap();
            button
                .set_attribute("aria-expanded", if open { "true" } else { "false" })
                .unwrap();
        }
    });

    Effect::new(move || {
        if !is_open.get() {
            return;
        }
        let (Some(reference), Some(floating), Some(arrow), Some(content)) = (
            reference_ref.get(),
            floating_ref.get(),
            arrow_ref.get(),
            content_ref.get(),
        ) else {
            return;
        };
        let update = Closure::new(move || {
            let reference = reference.clone();
            let floating = floating.clone();
            let arrow = arrow.clone();
            let content = content.clone();
            spawn_local(async move {
                let options = compute_options(10, arrow.as_ref());
                let position =
                    compute_position(reference.into(), floating.clone().into(), options).await;
                if !floating.is_connected() {
                    return;
                }
                let position: floating_ui::ComputePosition =
                    serde_wasm_bindgen::from_value(position).unwrap();
                let style = floating.deref().style();
                style.set_property("position", &position.strategy).unwrap();
                style
                    .set_property("left", &format!("{}px", position.x))
                    .unwrap();
                style
                    .set_property("top", &format!("{}px", position.y))
                    .unwrap();
                let style = arrow.deref().style();
                if let Some(x) = position.middleware_data.arrow.x {
                    style.set_property("left", &format!("{x}px")).unwrap();
                }
                let above = position.placement.starts_with("top");
                style
                    .set_property("top", if above { "100%" } else { "auto" })
                    .unwrap();
                style
                    .set_property("bottom", if above { "auto" } else { "100%" })
                    .unwrap();
                style
                    .set_property("transform", if above { "none" } else { "rotate(180deg)" })
                    .unwrap();
                content.set_class_name(if above {
                    styles::contentTop
                } else {
                    styles::contentBottom
                });
            });
        });
        let cleanup = auto_update(
            reference_ref.get_untracked().unwrap().into(),
            floating_ref.get_untracked().unwrap().into(),
            &update,
            JsValue::UNDEFINED,
        );
        let cleanup = SendWrapper::new(cleanup);
        let update = SendWrapper::new(update);
        on_cleanup(move || {
            cleanup.call0(&JsValue::NULL).unwrap();
            drop(update);
        });
    });

    let menu = Arc::new(menu);
    let popup = move || {
        let menu = Arc::clone(&menu);
        view! {
            <Show when=move || is_open.get()>
                <div
                    class=styles::container
                    node_ref=floating_ref
                    style="position: absolute; width: max-content;"
                >
                    <div
                        class=styles::arrow
                        node_ref=arrow_ref
                        style="position: absolute; pointer-events: none;"
                        aria-hidden="true"
                    >
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path stroke="none" d="M0,0 H20 L10,10 Z"></path>
                        </svg>
                    </div>
                    <div node_ref=content_ref role="dialog">
                        {menu(set_open)}
                    </div>
                </div>
            </Show>
        }
    };
    let popup = Arc::new(popup);
    let menu_view = move || {
        let popup = Arc::clone(&popup);
        if let Some(container) = menu_container.get() {
            view! { <Portal mount=container>{popup()}</Portal> }.into_any()
        } else {
            popup().into_any()
        }
    };

    view! {
        {button(Box::new(move |_| set_open.update(|open| *open = !*open)), reference_ref)}
        {menu_view}
    }
}
