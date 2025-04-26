use leptos::prelude::*;

use crate::app::selectable_menu_item::SelectableMenuItem;

#[component]
pub fn SelectOne<T, C, CH>(
    name: String,
    current_value: C,
    this_value: T,
    mut change_value: CH,
    children: Children,
) -> impl IntoView
where
    C: Fn() -> T + Send + 'static,
    CH: FnMut() + Send + 'static,
    T: PartialEq + Eq + Send + 'static,
{
    view! {
        <SelectableMenuItem
            name=name
            selected=move || { current_value() == this_value }
            on_click=move |_| change_value()
        >
            {children()}
        </SelectableMenuItem>
    }
}
