use std::collections::HashSet;

use istyles::istyles;
use leptos::{ev, portal::Portal, prelude::*, tachys::html};
use reactive_stores::Store;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;

use crate::{
    FileInfo, FileSnapshot, WorkerRequest,
    app::{
        GlobalState, GlobalStateStoreFields,
        button_set::{Button, ButtonSet},
        use_database,
    },
    send_request,
};

istyles!(styles, "assets/module.postcss/files.module.css.map");

#[derive(Clone)]
struct FileGroup {
    filename: String,
    files: Vec<FileInfo>,
    database: bool,
}

impl FileGroup {
    fn connected(&self) -> bool {
        self.files.iter().any(|file| file.in_use)
    }
}

fn main_name(filename: &str) -> Option<&str> {
    ["-journal", "-wal", "-shm"]
        .into_iter()
        .find_map(|suffix| filename.strip_suffix(suffix))
}

fn file_groups(snapshot: &FileSnapshot) -> Vec<FileGroup> {
    let mains = snapshot
        .files
        .iter()
        .filter(|file| main_name(&file.filename).is_none())
        .map(|file| file.filename.as_str())
        .collect::<HashSet<_>>();
    let mut groups = Vec::new();
    for file in &snapshot.files {
        if let Some(main) = main_name(&file.filename) {
            if !mains.contains(main) {
                groups.push(FileGroup {
                    filename: file.filename.clone(),
                    files: vec![file.clone()],
                    database: false,
                });
            }
        } else {
            let files = snapshot
                .files
                .iter()
                .filter(|component| {
                    component.filename == file.filename
                        || main_name(&component.filename) == Some(file.filename.as_str())
                })
                .cloned()
                .collect();
            groups.push(FileGroup {
                filename: file.filename.clone(),
                files,
                database: true,
            });
        }
    }
    groups.sort_by(|a, b| {
        b.database
            .cmp(&a.database)
            .then(a.filename.cmp(&b.filename))
    });
    groups
}

fn file_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KiB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn workspace_references(state: Store<GlobalState>, filename: &str) -> Vec<String> {
    let engine = state.engine().get();
    let mut names = Vec::new();
    if state.database().get() == filename {
        names.push(state.workspace_name().get());
    }
    state.workspaces().with(|workspaces| {
        names.extend(
            workspaces
                .iter()
                .filter(|workspace| workspace.engine == engine && workspace.database == filename)
                .map(|workspace| workspace.name.clone()),
        );
    });
    names
}

fn download_files(state: Store<GlobalState>, filenames: Vec<String>, zip: bool) {
    if state.is_busy().get_untracked() || filenames.is_empty() {
        return;
    }
    let request = if zip || filenames.len() > 1 {
        WorkerRequest::DownloadBundle(filenames)
    } else {
        WorkerRequest::DownloadFile(filenames.into_iter().next().unwrap())
    };
    send_request(state, request);
}

fn delete_files(state: Store<GlobalState>, filenames: Vec<String>) {
    if state.is_busy().get_untracked() || filenames.is_empty() {
        return;
    }
    let mut references = untrack(|| {
        filenames
            .iter()
            .flat_map(|filename| workspace_references(state, filename))
            .collect::<Vec<_>>()
    });
    references.sort();
    references.dedup();
    let mut message = format!(
        "Permanently delete these file groups and their logs?\n\n{}",
        filenames.join("\n"),
    );
    if !references.is_empty() {
        message.push_str(&format!(
            "\n\nUsed by workspaces: {}. Their main database will be marked Missing.",
            references.join(", "),
        ));
    }
    if window().confirm_with_message(&message).unwrap_or(false) {
        send_request(state, WorkerRequest::DeleteFiles(filenames));
    }
}

#[component]
pub fn FilesDrawer(upload: Callback<()>, trigger: NodeRef<html::element::Button>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let was_open = StoredValue::new(false);

    Effect::new(move || {
        let open = state.files_open().get();
        if !open
            && was_open.get_value()
            && let Some(button) = trigger.get_untracked()
        {
            let _ = button.focus();
        }
        was_open.set_value(open);
    });
    let keyboard = window_event_listener(ev::keydown, move |event| {
        if state.files_open().get_untracked() {
            if event.key() == "Escape" {
                event.prevent_default();
                state.files_open().set(false);
            } else if event.key() == "Tab" {
                trap_focus(event);
            }
        }
    });
    on_cleanup(move || keyboard.remove());

    view! {
        <Show when=move || state.files_open().get()>
            <Portal>
                <div class=styles::backdrop on:click=move |_| state.files_open().set(false)>
                    <section
                        class=styles::drawer
                        role="dialog"
                        aria-label="Files"
                        aria-modal="true"
                        on:click=move |event| event.stop_propagation()
                    >
                        <div class=styles::header>
                            <h2>"Files"</h2>
                            <CloseFiles />
                        </div>
                        <div class=styles::content>
                            <FilesContent upload=upload />
                        </div>
                    </section>
                </div>
            </Portal>
        </Show>
    }
}

#[component]
fn CloseFiles() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let button = NodeRef::<html::element::Button>::new();
    button.on_load(|button| {
        let _ = button.focus();
    });
    view! {
        <ButtonSet>
            <Button is_small=true node_ref=button on_click=move |_| state.files_open().set(false)>
                "Close Files"
            </Button>
        </ButtonSet>
    }
}

fn trap_focus(event: KeyboardEvent) {
    let Some(dialog) = document()
        .query_selector("[role='dialog'][aria-label='Files']")
        .unwrap()
    else {
        return;
    };
    let controls = dialog
        .query_selector_all(
            "button:not([disabled]), input:not([disabled]), select:not([disabled]), summary",
        )
        .unwrap();
    let controls = (0..controls.length())
        .filter_map(|index| {
            controls
                .item(index)?
                .dyn_into::<web_sys::HtmlElement>()
                .ok()
        })
        .filter(|element| {
            let rect = element.get_bounding_client_rect();
            rect.width() > 0.0 && rect.height() > 0.0
        })
        .collect::<Vec<_>>();
    let (Some(first), Some(last), Some(active)) = (
        controls.first(),
        controls.last(),
        document().active_element(),
    ) else {
        return;
    };
    if !dialog.contains(Some(active.as_ref())) {
        event.prevent_default();
        let _ = if event.shift_key() {
            last.focus()
        } else {
            first.focus()
        };
    } else if event.shift_key() && active.is_same_node(Some(first.as_ref())) {
        event.prevent_default();
        let _ = last.focus();
    } else if !event.shift_key() && active.is_same_node(Some(last.as_ref())) {
        event.prevent_default();
        let _ = first.focus();
    }
}

#[component]
fn FilesContent(upload: Callback<()>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let busy = Signal::derive(move || state.is_busy().get());
    let selected = RwSignal::new(HashSet::<String>::new());
    let zip = RwSignal::new(true);
    let name = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    let refreshed = StoredValue::new(None);
    let groups = move || {
        state
            .files()
            .get()
            .map(|files| file_groups(&files))
            .unwrap_or_default()
    };
    let in_transaction = Signal::derive(move || {
        state
            .files()
            .with(|files| files.as_ref().is_some_and(|files| files.in_transaction))
    });

    Effect::new(move || {
        let engine = state.engine().get();
        if !busy.get() && refreshed.get_value() != Some(engine) {
            refreshed.set_value(Some(engine));
            send_request(state, WorkerRequest::ListFiles);
        }
    });
    Effect::new(move || {
        let available = groups()
            .into_iter()
            .map(|group| group.filename)
            .collect::<HashSet<_>>();
        selected.update(|selected| selected.retain(|filename| available.contains(filename)));
    });
    Effect::new(move || {
        if selected.with(|selected| selected.len() > 1) {
            zip.set(true);
        }
    });
    Effect::new(move || {
        if !busy.get() && creating.get_untracked() {
            creating.set(false);
            if state.last_error().with_untracked(|error| error.is_none()) {
                name.set(String::new());
            }
        }
    });

    let selected_connected = Signal::derive(move || {
        let selected = selected.get();
        groups()
            .iter()
            .any(|group| selected.contains(&group.filename) && group.connected())
    });
    let selected_names = move || {
        let mut names = selected.get_untracked().into_iter().collect::<Vec<_>>();
        names.sort();
        names
    };
    let create = move || {
        let filename = name.get_untracked().trim().to_owned();
        if !busy.get_untracked() && !filename.is_empty() {
            creating.set(true);
            send_request(state, WorkerRequest::CreateDb(filename));
        }
    };

    view! {
        <p class=styles::scope>
            {move || format!("{} · Files are shared by workspaces using this engine.", state.engine().get().value())}
        </p>
        <div class=styles::toolbar>
            <div class=styles::actions>
                <ButtonSet>
                    <Button is_small=true disabled=busy on_click=move |_| upload.run(())>
                        "Upload"
                    </Button>
                </ButtonSet>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=busy
                        on_click=move |_| send_request(state, WorkerRequest::ListFiles)
                    >
                        "Refresh"
                    </Button>
                </ButtonSet>
            </div>
            <p class=styles::hint>
                "Upload database files and ZIP archives together without changing \
                this workspace’s connection settings."
            </p>
            <div class=styles::actions>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || state.files().with(|files| {
                                files.as_ref().is_none_or(|files| files.connection.is_none())
                            })
                        })
                        on_click=move |_| send_request(state, WorkerRequest::Close)
                    >
                        "Disconnect"
                    </Button>
                </ButtonSet>
            </div>
            <p class=styles::hint>
                "Disconnect before deleting connected files. \
                Commit or roll back any active transaction first."
            </p>
            <div class=styles::createRow>
                <input
                    class=styles::input
                    type="text"
                    aria-label="New database name"
                    placeholder="New database name"
                    prop:value=move || name.get()
                    disabled=move || busy.get()
                    on:input=move |event| name.set(event_target_value(&event))
                    on:keydown=move |event| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            create();
                        }
                    }
                />
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || busy.get() || name.get().trim().is_empty())
                        on_click=move |_| create()
                    >
                        "Create database"
                    </Button>
                </ButtonSet>
            </div>
            <p class=styles::hint>
                "Creates an empty database. Use as main selects it for this workspace."
            </p>
            <label>
                "Download format "
                <select
                    class=styles::select
                    aria-label="Download format"
                    prop:value=move || if zip.get() { "zip" } else { "database" }
                    disabled=move || busy.get()
                    on:change=move |event| zip.set(event_target_value(&event) == "zip")
                >
                    <option value="zip">"ZIP (database + logs)"</option>
                    <option
                        value="database"
                        disabled=move || selected.with(|selected| selected.len() > 1)
                    >
                        "Database file"
                    </option>
                </select>
            </label>
            <p class=styles::hint>
                "ZIP keeps databases and their logs together. \
                Multiple selections are downloaded as ZIP."
            </p>
        </div>
        {move || {
            state.last_error()
                .with(|error| error.as_ref().map(|error| error.to_string()))
                .map(|message| view! {
                    <div class=styles::error role="alert">{message}</div>
                })
        }}
        {move || {
            state.import_progress()
                .with(|progress| progress.as_ref().map(|progress| {
                    let status = match progress.opened {
                        Some(true) => "Upload complete",
                        Some(false) => "Upload failed",
                        None => "Importing files",
                    };
                    format!("{status}: {}", progress.filename)
                }))
                .map(|message| view! {
                    <p class=styles::hint role="status">{message}</p>
                })
        }}
        <Show when=move || busy.get()>
            <p class=styles::hint role="status">"Working…"</p>
        </Show>
        <div class=styles::selection>
            <div class=styles::actions>
                <label>
                    <input
                        type="checkbox"
                        aria-label="Select all files"
                        disabled=move || busy.get() || groups().is_empty()
                        prop:checked=move || {
                            let groups = groups();
                            !groups.is_empty() && selected.with(|selected| {
                                groups.iter().all(|group| selected.contains(&group.filename))
                            })
                        }
                        on:change=move |event| {
                            selected.set(if event_target_checked(&event) {
                                groups().into_iter().map(|group| group.filename).collect()
                            } else {
                                HashSet::new()
                            });
                        }
                    />
                    " Select all"
                </label>
                <span>{move || format!("{} selected", selected.with(HashSet::len))}</span>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || busy.get() || selected.with(HashSet::is_empty))
                        on_click=move |_| download_files(state, selected_names(), zip.get_untracked())
                    >
                        "Download selected"
                    </Button>
                </ButtonSet>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || selected.with(HashSet::is_empty)
                                || selected_connected.get() || in_transaction.get()
                        })
                        on_click=move |_| delete_files(state, selected_names())
                    >
                        "Delete selected"
                    </Button>
                </ButtonSet>
            </div>
            <p class=styles::hint>
                "Deleting a database also removes its logs. Workspaces using it will show Missing. \
                Finish active transactions before deleting files."
            </p>
        </div>
        <Show when=move || {
            state.files().with(|files| files.is_some()) && groups().is_empty() && !busy.get()
        }>
            <p class=styles::empty>"No files in this engine yet. Create a database or upload files."</p>
        </Show>
        <For
            each=groups
            key=|group| group.filename.clone()
            children=move |group| view! {
                <FileGroupCard group=group selected=selected zip=zip />
            }
        />
    }
}

#[component]
fn FileGroupCard(
    group: FileGroup,
    selected: RwSignal<HashSet<String>>,
    zip: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let busy = Signal::derive(move || state.is_busy().get());
    let database = group.database;
    let filename = StoredValue::new(group.filename.clone());
    let files = Signal::derive(move || {
        let name = filename.get_value();
        state
            .files()
            .get()
            .map(|snapshot| {
                snapshot
                    .files
                    .into_iter()
                    .filter(|file| {
                        file.filename == name
                            || (database && main_name(&file.filename) == Some(name.as_str()))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let connected =
        Signal::derive(move || files.with(|files| files.iter().any(|file| file.in_use)));
    let label = format!(
        "{} {}",
        if group.database { "Database" } else { "File" },
        group.filename
    );

    view! {
        <section class=styles::group aria-label=label>
            <div class=styles::groupHeader>
                <input
                    type="checkbox"
                    aria-label=format!("Select {}", group.filename)
                    disabled=move || busy.get()
                    prop:checked=move || selected.with(|selected| selected.contains(&filename.get_value()))
                    on:change=move |event| selected.update(|selected| {
                        if event_target_checked(&event) {
                            selected.insert(filename.get_value());
                        } else {
                            selected.remove(&filename.get_value());
                        }
                    })
                />
                <span class=styles::name>{group.filename.clone()}</span>
                <span>{move || {
                    files.with(|files| file_size(files.iter().map(|file| file.size).sum()))
                }}</span>
            </div>
            <div class=styles::badges>
                <Show when=move || database && state.database().get() == filename.get_value()>
                    <span class=styles::badge>"Main"</span>
                </Show>
                <Show when=move || connected.get()>
                    <span class=styles::badge>"Connected"</span>
                </Show>
                {(!database).then(|| view! { <span class=styles::badge>"Orphan log"</span> })}
            </div>
            {move || {
                let references = workspace_references(state, &filename.get_value());
                (!references.is_empty()).then(|| view! {
                    <p class=styles::metadata>{format!("Used by: {}", references.join(", "))}</p>
                })
            }}
            <Show when=move || files.with(|files| files.len() > 1)>
                <details class=styles::componentList>
                    <summary>{move || format!("Files and logs ({})", files.with(Vec::len))}</summary>
                    <ul>{move || {
                        files.get().into_iter().map(|file| view! {
                            <li class=styles::component>
                                <span>{file.filename}</span>
                                <span>{file_size(file.size)}</span>
                            </li>
                        }).collect_view()
                    }}</ul>
                </details>
            </Show>
            <div class=styles::actions>
                {database.then(|| view! {
                    <ButtonSet>
                        <Button
                            is_small=true
                            disabled=Signal::derive(move || {
                                busy.get() || state.database().get() == filename.get_value()
                            })
                            on_click=move |_| use_database(state, filename.get_value())
                        >
                            "Use as main"
                        </Button>
                    </ButtonSet>
                })}
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=busy
                        on_click=move |_| {
                            download_files(state, vec![filename.get_value()], zip.get_untracked())
                        }
                    >
                        "Download"
                    </Button>
                </ButtonSet>
                <ButtonSet>
                    <Button
                        is_small=true
                        disabled=Signal::derive(move || {
                            busy.get() || connected.get() || state.files().with(|files| {
                                files.as_ref().is_some_and(|files| files.in_transaction)
                            })
                        })
                        on_click=move |_| delete_files(state, vec![filename.get_value()])
                    >
                        "Delete"
                    </Button>
                </ButtonSet>
            </div>
        </section>
    }
}
