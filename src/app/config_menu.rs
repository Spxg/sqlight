use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen::JsValue;
use web_sys::{Event, HtmlSelectElement};

use crate::{
    SQLightError,
    app::{
        GlobalState, GlobalStateStoreFields, Orientation, Theme,
        config_element::Select as SelectConfig, menu_group::MenuGroup,
    },
};

const ACE_KEYBOARDS: [&str; 5] = ["ace", "emacs", "sublime", "vim", "vscode"];
const ACE_THEMES: [&str; 3] = ["github", "github_dark", "gruvbox"];

fn selecet_view(s: &str, selected: &str) -> AnyView {
    if s == selected {
        view! {
            <option selected value=s>
                {s}
            </option>
        }
        .into_any()
    } else {
        view! { <option value=s>{s}</option> }.into_any()
    }
}

#[component]
pub fn ConfigMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let ace_keyboard_change = move |event: Event| {
        if let Some(target) = event.target() {
            let select = HtmlSelectElement::from(JsValue::from(target));
            state.editor_config().write().keyboard = select.value();
            if let Some(Err(err)) = state.editor().read().as_ref().map(|editor| {
                let keyboard = select.value();
                let handler = (keyboard != "ace").then_some(format!("ace/keyboard/{keyboard}"));
                editor.set_keyboard_handler(handler.as_deref())
            }) {
                state
                    .last_error()
                    .set(Some(SQLightError::new_ace_editor(err)));
            }
        }
    };

    let ace_theme_change = move |event: Event| {
        if let Some(target) = event.target() {
            let select = HtmlSelectElement::from(JsValue::from(target));
            state.editor_config().write().theme = select.value();
            if let Some(Err(err)) = state
                .editor()
                .read()
                .as_ref()
                .map(|editor| editor.set_theme(&format!("ace/theme/{}", select.value())))
            {
                state
                    .last_error()
                    .set(Some(SQLightError::new_ace_editor(err)));
            }
        }
    };

    let theme_change = move |event: Event| {
        if let Some(target) = event.target() {
            let select = HtmlSelectElement::from(JsValue::from(target));
            state.theme().set(Theme::from_select(&select.value()));
        }
    };

    let orientation_change = move |event: Event| {
        if let Some(target) = event.target() {
            let select = HtmlSelectElement::from(JsValue::from(target));
            state
                .orientation()
                .set(Orientation::from_select(&select.value()));
        }
    };

    view! {
        <MenuGroup title="Editor".into()>
            <SelectConfig name="Keybinding".into() on_change=ace_keyboard_change>
                {move || {
                    ACE_KEYBOARDS
                        .into_iter()
                        .map(|s| selecet_view(s, &state.editor_config().read().keyboard))
                        .collect_view()
                }}
            </SelectConfig>
            <SelectConfig name="Theme".into() on_change=ace_theme_change>
                {move || {
                    ACE_THEMES
                        .into_iter()
                        .map(|s| selecet_view(s, &state.editor_config().read().theme))
                        .collect_view()
                }}
            </SelectConfig>
        </MenuGroup>

        <MenuGroup title="UI".into()>
            <SelectConfig name="Theme".into() on_change=theme_change>
                {move || {
                    ["System", "Light", "Dark"]
                        .into_iter()
                        .map(|s| selecet_view(s, &state.theme().read().select()))
                        .collect_view()
                }}
            </SelectConfig>
            <SelectConfig name="Orientation".into() on_change=orientation_change>
                {move || {
                    ["Automatic", "Horizontal", "Vertical"]
                        .into_iter()
                        .map(|s| selecet_view(s, &state.orientation().read().select()))
                        .collect_view()
                }}
            </SelectConfig>
        </MenuGroup>
    }
}
