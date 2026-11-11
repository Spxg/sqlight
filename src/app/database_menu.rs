use leptos::prelude::*;
use reactive_stores::Store;
use web_sys::{Event, MouseEvent};

use crate::{
    WorkerRequest,
    app::{
        GlobalState, GlobalStateStoreFields, button_menu_item::ButtonMenuItem,
        config_element::Select, menu_aside::MenuAside, menu_group::MenuGroup,
    },
    send_request,
};

#[component]
pub fn DatabaseMenu<L, D>(load: L, download: D, busy: Signal<bool>) -> impl IntoView
where
    L: Fn(MouseEvent) + Send + 'static,
    D: Fn(String) + Send + 'static,
{
    let state = expect_context::<Store<GlobalState>>();
    let download_database = RwSignal::new(state.database().get_untracked());
    let refreshed = StoredValue::new(false);
    Effect::new(move || {
        if !busy.get() && !refreshed.get_value() {
            refreshed.set_value(true);
            send_request(state, WorkerRequest::ListDbs);
        }
    });

    let databases = move || {
        let mut names = vec![state.database().get()];
        state.databases().with(|databases| {
            for name in databases {
                if !names.contains(name) {
                    names.push(name.clone());
                }
            }
        });
        names
            .into_iter()
            .map(|name| {
                let label = name.clone();
                view! { <option value=name>{label}</option> }
            })
            .collect_view()
    };

    view! {
        <MenuGroup title="Database".into()>
            <Select
                name="Download database".into()
                value=move || download_database.get()
                disabled=busy
                on_change=move |event: Event| {
                    download_database.set(event_target_value(&event));
                }
            >
                {databases}
            </Select>
            <ButtonMenuItem name="Load".into() on_click=load disabled=busy>
                <MenuAside>
                    "Import one or more databases with their filenames. Keeps this workspace’s connection settings."
                </MenuAside>
            </ButtonMenuItem>
            <ButtonMenuItem
                name="Download".into()
                on_click=move |_| download(download_database.get_untracked())
                disabled=busy
            >
                <MenuAside>"Save the selected shared database file without changing this workspace’s connection."</MenuAside>
            </ButtonMenuItem>
        </MenuGroup>
    }
}
