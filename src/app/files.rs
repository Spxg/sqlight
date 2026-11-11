use std::collections::{BTreeMap, HashSet};

use istyles::istyles;
use leptos::prelude::*;
use reactive_stores::Store;

use crate::{
    FileInfo, FileSnapshot, WorkerRequest,
    app::{
        GlobalState, GlobalStateStoreFields,
        button_set::{Button, ButtonSet},
        menu_group::MenuGroup,
    },
    send_request,
};

istyles!(styles, "assets/module.postcss/files.module.css.map");

#[derive(Clone, PartialEq, Eq)]
struct FileGroup {
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

fn file_groups(snapshot: &FileSnapshot) -> BTreeMap<String, FileGroup> {
    let mains = snapshot
        .files
        .iter()
        .filter(|file| main_name(&file.filename).is_none())
        .map(|file| file.filename.as_str())
        .collect::<HashSet<_>>();
    let mut groups = BTreeMap::new();
    for file in &snapshot.files {
        let name = main_name(&file.filename)
            .filter(|name| mains.contains(name))
            .unwrap_or(&file.filename);
        groups
            .entry(name.to_owned())
            .or_insert_with(|| FileGroup {
                files: Vec::new(),
                database: mains.contains(name),
            })
            .files
            .push(file.clone());
    }
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
pub fn DatabaseMenu(upload: Callback<()>) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let busy = Signal::derive(move || state.is_busy().get());
    let selected = RwSignal::new(HashSet::<String>::new());
    let zip = RwSignal::new(true);
    let name = RwSignal::new(String::new());
    let creating = RwSignal::new(false);
    let refreshed = StoredValue::new(None);
    let groups = Memo::new(move |_| {
        state
            .files()
            .with(|files| files.as_ref().map(file_groups).unwrap_or_default())
    });
    let group_names = Memo::new(move |_| {
        groups.with(|groups| {
            let mut names = Vec::with_capacity(groups.len());
            for database in [true, false] {
                names.extend(
                    groups
                        .iter()
                        .filter(|(_, group)| group.database == database)
                        .map(|(name, _)| name.clone()),
                );
            }
            names
        })
    });
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
        groups.with(|groups| {
            selected.maybe_update(|selected| {
                let previous = selected.len();
                selected.retain(|filename| groups.contains_key(filename));
                selected.len() != previous
            });
        });
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

    let selected_connected = Memo::new(move |_| {
        selected.with(|selected| {
            groups.with(|groups| {
                selected
                    .iter()
                    .any(|name| groups.get(name).is_some_and(FileGroup::connected))
            })
        })
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
        <MenuGroup title="Database".into() wide=true>
            <p class=styles::scope>
                {move || format!(
                    "{} · Files are shared by workspaces using this engine.",
                    state.engine().get().value(),
                )}
            </p>
            <div class=styles::toolbar>
                <div>
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
                </div>
                <div>
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
                </div>
                <div>
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
                                disabled=Signal::derive(move || {
                                    busy.get() || name.get().trim().is_empty()
                                })
                                on_click=move |_| create()
                            >
                                "Create database"
                            </Button>
                        </ButtonSet>
                    </div>
                    <p class=styles::hint>
                        "Creates an empty database. Select it under Workspace → Main database."
                    </p>
                </div>
                <div>
                    <label class=styles::downloadFormat>
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
                            disabled=move || busy.get() || groups.with(BTreeMap::is_empty)
                            prop:checked=move || {
                                groups.with(|groups| {
                                    !groups.is_empty() && selected.with(|selected| {
                                        groups.keys().all(|name| selected.contains(name))
                                    })
                                })
                            }
                            on:change=move |event| {
                                selected.set(if event_target_checked(&event) {
                                    groups.with(|groups| groups.keys().cloned().collect())
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
                state.files().with(|files| files.is_some())
                    && groups.with(BTreeMap::is_empty) && !busy.get()
            }>
                <p class=styles::empty>"No files in this engine yet. Create a database or upload files."</p>
            </Show>
            <div class=styles::groups>
                <For
                    each=move || group_names.get()
                    key=|filename| filename.clone()
                    children=move |filename| view! {
                        <FileGroupCard
                            filename=filename
                            groups=groups
                            selected=selected
                            zip=zip
                        />
                    }
                />
            </div>
        </MenuGroup>
    }
}

#[component]
fn FileGroupCard(
    filename: String,
    groups: Memo<BTreeMap<String, FileGroup>>,
    selected: RwSignal<HashSet<String>>,
    zip: RwSignal<bool>,
) -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();
    let busy = Signal::derive(move || state.is_busy().get());
    let database = main_name(&filename).is_none();
    let filename = StoredValue::new(filename);
    let files = Memo::new(move |_| {
        groups.with(|groups| {
            groups
                .get(&filename.get_value())
                .map(|group| group.files.clone())
                .unwrap_or_default()
        })
    });
    let connected = Memo::new(move |_| files.with(|files| files.iter().any(|file| file.in_use)));
    let label = format!(
        "{} {}",
        if database { "Database" } else { "File" },
        filename.get_value()
    );

    view! {
        <section class=styles::group aria-label=label>
            <div class=styles::groupHeader>
                <input
                    type="checkbox"
                    aria-label=format!("Select {}", filename.get_value())
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
                <span class=styles::name>{filename.get_value()}</span>
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
