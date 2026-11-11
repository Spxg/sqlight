use leptos::prelude::*;
use reactive_stores::Store;
use web_sys::Event;

use crate::app::{
    GlobalState, GlobalStateStoreFields, Orientation, Theme,
    config_element::Select as SelectConfig, menu_group::MenuGroup,
};

const ACE_KEYBOARDS: [&str; 5] = ["ace", "emacs", "sublime", "vim", "vscode"];
const ACE_THEMES: [&str; 6] = [
    "github",
    "github_dark",
    "github_light_default",
    "gruvbox",
    "gruvbox_light_hard",
    "gruvbox_dark_hard",
];

fn options(values: impl IntoIterator<Item = &'static str>) -> impl IntoView {
    values
        .into_iter()
        .map(|value| view! { <option value=value>{value}</option> })
        .collect_view()
}

#[component]
pub fn ConfigMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    view! {
        <MenuGroup title="Editor".into()>
            <SelectConfig
                name="Keybinding".into()
                value=move || state.editor_config().read().keyboard.clone()
                on_change=move |event: Event| {
                    state.editor_config().write().keyboard = event_target_value(&event);
                }
            >
                {options(ACE_KEYBOARDS)}
            </SelectConfig>
            <SelectConfig
                name="Light Theme".into()
                value=move || state.editor_config().read().light_theme.clone()
                on_change=move |event: Event| {
                    state.editor_config().write().light_theme = event_target_value(&event);
                }
            >
                {options(ACE_THEMES)}
            </SelectConfig>
            <SelectConfig
                name="Dark Theme".into()
                value=move || state.editor_config().read().dark_theme.clone()
                on_change=move |event: Event| {
                    state.editor_config().write().dark_theme = event_target_value(&event);
                }
            >
                {options(ACE_THEMES)}
            </SelectConfig>
        </MenuGroup>

        <MenuGroup title="UI".into()>
            <SelectConfig
                name="Theme".into()
                value=move || state.theme().read().select().to_string()
                on_change=move |event: Event| {
                    state.theme().set(Theme::from_select(&event_target_value(&event)));
                }
            >
                {options(["System", "Light", "Dark"])}
            </SelectConfig>
            <SelectConfig
                name="Orientation".into()
                value=move || state.orientation().read().select().to_string()
                on_change=move |event: Event| {
                    state.orientation().set(Orientation::from_select(&event_target_value(&event)));
                }
            >
                {options(["Automatic", "Horizontal", "Vertical"])}
            </SelectConfig>
        </MenuGroup>
    }
}
