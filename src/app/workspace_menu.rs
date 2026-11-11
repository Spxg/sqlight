use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;
use web_sys::Event;

use crate::{
    SQLightError, WorkerError, WorkerRequest,
    app::{
        Engine, GlobalState, GlobalStateStoreFields,
        button_set::{Button, ButtonSet},
        config_element::{ConfigElement, Select},
        delete_workspace,
        menu_aside::MenuAside,
        menu_group::MenuGroup,
        new_workspace, rename_workspace, switch_workspace,
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
    let database_name = RwSignal::new(String::new());
    let busy = Signal::derive(move || state.is_busy().get());
    let refreshed = StoredValue::new(None::<(String, Engine)>);

    Effect::new(move || name.set(state.workspace_name().get()));
    Effect::new(move || {
        let database = state.database().get();
        if database_name.get_untracked().trim() == database {
            database_name.set(String::new());
        }
    });
    Effect::new(move || {
        let key = (state.workspace_id().get(), state.engine().get());
        if !busy.get() && refreshed.get_value().as_ref() != Some(&key) {
            refreshed.set_value(Some(key));
            state.databases().set(Vec::new());
            send_request(state, WorkerRequest::ListDbs);
        }
    });

    let create_database = move || {
        let filename = database_name.get_untracked().trim().to_owned();
        if !busy.get_untracked() && !filename.is_empty() {
            send_request(state, WorkerRequest::CreateDb(filename));
        }
    };
    let create_error = move || {
        state.last_error().with(|error| match error.as_deref() {
            Some(SQLightError::Worker(WorkerError::CreateDb(message))) => Some(message.clone()),
            _ => None,
        })
    };

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
                let label = name.clone();
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
                        "Choose the main database from files stored persistently in this browser’s OPFS for this site."
                    </MenuAside>
                }.into_any())
                on_change=move |event: Event| {
                    batch(move || {
                        state.database().set(event_target_value(&event));
                        state.keep_ctx().set(true);
                    });
                }
            >
                {databases}
            </Select>
            <div class=styles::databaseCreate>
                <input
                    type="text"
                    class=styles::input
                    aria-label="New database name"
                    placeholder="New database name"
                    prop:value=move || database_name.get()
                    disabled=move || busy.get()
                    on:input=move |event| database_name.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            create_database();
                        }
                    }
                />
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || database_name.get().trim().is_empty()
                        })
                        on_click=move |_| create_database()
                    >
                        "Create database"
                    </Button>
                </ButtonSet>
            </div>
            <MenuAside>"Creates an empty database and selects it as this workspace’s main database."</MenuAside>
            {move || create_error().map(|message| view! {
                <p class=styles::databaseError role="alert">{message}</p>
            })}
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
                    state.engine().set(Engine::from_select(&event_target_value(&event)));
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
