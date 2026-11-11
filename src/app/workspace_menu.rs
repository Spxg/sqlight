use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use web_sys::Event;

use crate::{
    WorkerRequest,
    app::{
        Engine, GlobalState, GlobalStateStoreFields,
        button_set::{Button, ButtonSet},
        config_element::{ConfigElement, Select},
        delete_workspace,
        menu_aside::MenuAside,
        menu_group::MenuGroup,
        new_workspace, rename_workspace, switch_workspace, use_database,
    },
    send_request,
};

istyles!(
    styles,
    "assets/module.postcss/config_element.module.css.map"
);

#[component]
pub fn WorkspaceMenu() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let name = RwSignal::new(state.workspace_name().get_untracked());
    let busy = Signal::derive(move || state.is_busy().get());
    let refreshed = StoredValue::new(None::<(String, Engine)>);

    Effect::new(move || name.set(state.workspace_name().get()));
    Effect::new(move || {
        let key = (state.workspace_id().get(), state.engine().get());
        if !busy.get() && refreshed.get_value().as_ref() != Some(&key) {
            refreshed.set_value(Some(key));
            state.databases().set(Vec::new());
            state.files().set(None);
            send_request(state, WorkerRequest::ListFiles);
        }
    });

    let options = move || {
        let mut workspaces = vec![(state.workspace_id().get(), state.workspace_name().get())];
        state.workspaces().with(|saved| {
            workspaces.extend(
                saved
                    .iter()
                    .map(|workspace| (workspace.id.clone(), workspace.name.clone())),
            );
        });
        workspaces
            .into_iter()
            .map(|(id, name)| view! { <option value=id>{name}</option> })
            .collect_view()
    };
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
                let exists = state.databases().with(|names| names.contains(&name));
                let label = if !exists && state.files().read().is_some() {
                    format!("{name} (Missing)")
                } else {
                    name.clone()
                };
                view! { <option value=name>{label}</option> }
            })
            .collect_view()
    };

    view! {
        <MenuGroup title="Workspace".into()>
            <Select
                name="Workspace".into()
                value=move || state.workspace_id().get()
                disabled=busy
                on_change=move |event: Event| switch_workspace(state, event_target_value(&event))
            >
                {options}
            </Select>
            <ConfigElement name="Name".into()>
                <input
                    type="text"
                    class=styles::input
                    aria-label="Workspace name"
                    prop:value=move || name.get()
                    disabled=move || busy.get()
                    on:input=move |event| name.set(event_target_value(&event))
                />
            </ConfigElement>
            <div class=styles::workspaceActions>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=busy
                        on_click=move |_| new_workspace(state)
                    >
                        "New workspace"
                    </Button>
                </ButtonSet>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || name.get().trim().is_empty()
                                || name.get().trim() == state.workspace_name().get()
                        })
                        on_click=move |_| rename_workspace(state, name.get_untracked())
                    >
                        "Rename workspace"
                    </Button>
                </ButtonSet>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || state.workspaces().read().is_empty()
                        })
                        on_click=move |_| delete_workspace(state)
                    >
                        "Delete workspace"
                    </Button>
                </ButtonSet>
            </div>
            <MenuAside>
                "Workspaces save SQL and connection settings. Deleting one keeps all database files."
            </MenuAside>
        </MenuGroup>
        <MenuGroup title="Connection".into()>
            <Select
                name="Main database".into()
                value=move || state.database().get()
                disabled=busy
                aside=Some(view! {
                    <MenuAside>
                        "Choose a database created or uploaded in Database. This selection is saved with the workspace."
                    </MenuAside>
                }.into_any())
                on_change=move |event: Event| {
                    use_database(state, event_target_value(&event));
                }
            >
                {databases}
            </Select>
            <ConfigElement
                name="Read only".into()
                aside=Some(view! {
                    <MenuAside>
                        "Prevents SQL writes while still allowing database creation and file imports."
                    </MenuAside>
                }.into_any())
            >
                <input
                    type="checkbox"
                    aria-label="Read only"
                    prop:checked=move || state.read_only().get()
                    disabled=move || busy.get()
                    on:change=move |event| {
                        state.read_only().set(event_target_checked(&event));
                    }
                />
            </ConfigElement>
            <ConfigElement
                name="Fresh database per run".into()
                aside=Some(view! {
                    <MenuAside>
                        "Rebuilds the selected main database before each run, affecting every workspace that shares that file."
                    </MenuAside>
                }.into_any())
            >
                <input
                    type="checkbox"
                    aria-label="Fresh database per run"
                    prop:checked=move || !state.keep_ctx().get()
                    disabled=move || busy.get()
                    on:change=move |event| {
                        state.keep_ctx().set(!event_target_checked(&event));
                    }
                />
            </ConfigElement>
            <Select
                name="SQLite Engine".into()
                value=move || state.engine().get().value().to_owned()
                disabled=busy
                aside=Some(view! {
                    <MenuAside>
                        "Workspaces using the same engine share files; different engines have separate files."
                    </MenuAside>
                }.into_any())
                on_change=move |event| {
                    batch(move || {
                        state.engine().set(Engine::from_select(&event_target_value(&event)));
                        state.files().set(None);
                        state.databases().set(Vec::new());
                    });
                }
            >
                {[Engine::SQLite, Engine::MultipleCiphers, Engine::SQLCipher]
                    .into_iter()
                    .map(|engine| {
                        let value = engine.value();
                        view! { <option value=value>{value}</option> }
                    })
                    .collect_view()}
            </Select>
        </MenuGroup>
    }
}
