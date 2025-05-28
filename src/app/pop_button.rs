use std::{ops::Deref, sync::Arc};

use floating_ui::{
    ArrowPosition, ComputePosition, MiddlewareData, auto_update, compute_options, compute_position,
};
use istyles::istyles;
use js_sys::Object;
use leptos::{portal::Portal, prelude::*, tachys::html};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{KeyboardEvent, MouseEvent};

use crate::FragileComfirmed;

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
    let toggle = move || set_open.set(!is_open.get());

    let arrow_ref = NodeRef::<html::element::Div>::new();
    let reference_ref = NodeRef::<html::element::Button>::new();
    let floating_ref = NodeRef::<html::element::Div>::new();
    let menu_ref = NodeRef::<html::element::Div>::new();

    Effect::new(move || {
        let key_listener = move |event: KeyboardEvent| {
            if !is_open.get_untracked() {
                return;
            }

            if event.key() == "Escape" {
                set_open.set(false);
            }
        };

        let callback = FragileComfirmed::new(Closure::<dyn Fn(KeyboardEvent)>::new(key_listener));

        window()
            .add_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())
            .unwrap();

        on_cleanup(move || {
            window()
                .remove_event_listener_with_callback("keydown", callback.as_ref().unchecked_ref())
                .unwrap();
            drop(callback)
        });
    });

    Effect::new(move || {
        let listener = move |event: MouseEvent| {
            if !is_open.get_untracked() {
                return;
            }

            if let Some(target) = event.target() {
                let node = target.dyn_into::<web_sys::Node>().ok();
                if !reference_ref.with_untracked(|reference| {
                    reference
                        .as_ref()
                        .is_some_and(|reference| reference.deref().contains(node.as_ref()))
                }) && !floating_ref.with_untracked(|floating| {
                    floating
                        .as_ref()
                        .is_some_and(|floating| floating.deref().contains(node.as_ref()))
                }) {
                    set_open.set(false);
                }
            }
        };

        let callback = FragileComfirmed::new(Closure::<dyn Fn(MouseEvent)>::new(listener));

        window()
            .add_event_listener_with_callback("click", callback.as_ref().unchecked_ref())
            .unwrap();

        on_cleanup(move || {
            window()
                .remove_event_listener_with_callback("click", callback.as_ref().unchecked_ref())
                .unwrap();
            drop(callback)
        });
    });

    Effect::new(move || {
        let callback = Closure::new(move || {
            let options = compute_options(10, &arrow_ref.get_untracked().into());

            spawn_local(async move {
                let value = compute_position(
                    reference_ref.get_untracked().into(),
                    floating_ref.get_untracked().into(),
                    options,
                )
                .await;

                let ComputePosition {
                    x,
                    y,
                    placement,
                    strategy,
                    middleware_data,
                } = serde_wasm_bindgen::from_value(value).unwrap();

                if let Some(element) = floating_ref.get_untracked() {
                    let style = element.deref().style();
                    #[derive(serde::Serialize)]
                    struct Style {
                        position: String,
                        left: String,
                        top: String,
                    }

                    let pos = serde_wasm_bindgen::to_value(&Style {
                        position: strategy,
                        left: format!("{x}px"),
                        top: format!("{y}px"),
                    })
                    .unwrap();
                    Object::assign(&style, &pos.into());
                }

                if let Some(element) = arrow_ref.get_untracked() {
                    let MiddlewareData {
                        arrow: ArrowPosition { x },
                    } = middleware_data;
                    let style = element.deref().style();
                    #[derive(serde::Serialize)]
                    struct Style {
                        left: String,
                    }

                    let pos = serde_wasm_bindgen::to_value(&Style {
                        left: format!("{x}px"),
                    })
                    .unwrap();
                    Object::assign(&style, &pos.into());
                }

                if let Some(menu_ref) = menu_ref.get_untracked() {
                    let class = if placement == "top" {
                        styles::contentTop
                    } else if placement == "bottom" {
                        styles::contentBottom
                    } else {
                        ""
                    };
                    menu_ref.set_class_name(class);
                }
            });
        });

        if let (Some(reference), Some(floating)) = (&*reference_ref.read(), &*floating_ref.read()) {
            let func = auto_update(
                reference.into(),
                floating.into(),
                &callback,
                JsValue::default(),
            );
            let func = FragileComfirmed::new(func);
            let callback = FragileComfirmed::new(callback);

            on_cleanup(move || {
                func.call0(&JsValue::null()).unwrap();
                drop(callback);
            });
        }
    });

    let menu = Arc::new(menu);
    let float = move || {
        let menu_clone = Arc::clone(&menu);
        view! {
            <Show when=move || is_open.get() fallback=|| ()>
                <div
                    class=styles::container
                    node_ref=floating_ref
                    style="position: absolute; width: max-content;"
                >
                    <div
                        class=styles::arrow
                        node_ref=arrow_ref
                        style="position: absolute; pointer-events: none; bottom: 100%; transform: rotate(180deg);"
                    >
                        <svg width="20" height="20" viewBox="0 0 20 20">
                            <path stroke="none" d="M0,0 H20 L10,10 Q10,10 10,10 Z"></path>
                            <clipPath id=":rh:">
                                <rect x="0" y="0" width="20" height="20"></rect>
                            </clipPath>
                        </svg>
                    </div>
                    <div node_ref=menu_ref>
                        <div>{menu_clone(set_open)}</div>
                    </div>
                </div>
            </Show>
        }
    };

    let float = Arc::new(float);
    let total = move || {
        let float_clone = Arc::clone(&float);
        if let Some(container) = menu_container.get() {
            view! { <Portal mount=container.clone()>{float_clone()}</Portal> }.into_any()
        } else {
            float_clone().into_any()
        }
    };

    view! { <>{button(Box::new(move |_| toggle()), reference_ref)} {total}</> }
}
