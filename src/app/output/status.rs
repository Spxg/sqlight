use std::ops::Deref;

use leptos::prelude::*;
use reactive_stores::Store;

use crate::{
    SQLightError, SQLitendError, WorkerError,
    app::{
        GlobalState, GlobalStateStoreFields,
        output::{section::Section, simple_pane::SimplePane},
    },
};

const OPFS_SAH_POOL_OPENED_DETAILS: &str = "Due to OPFS SyncAccessHandle restrictions, \
the db can only have one web tab access.

Please close other tabs and refresh, or switch to Memory VFS.";

#[component]
pub fn Status() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let last_error = move || match &*state.last_error().read() {
        Some(error) => {
            let summary = format!("{}", error.deref());
            let details = match error.deref() {
                SQLightError::Worker(worker) => match worker {
                    WorkerError::SQLite(sqlitend_error) => match sqlitend_error {
                        SQLitendError::ToCStr
                        | SQLitendError::GetColumnName(_)
                        | SQLitendError::Utf8Text => {
                            "This shouldn't happen, please create an issue on github."
                        }
                        SQLitendError::OpenDb(_) => {
                            "If database disk image is malformed, please enable the discard context option and use it once."
                        }
                        SQLitendError::Prepare(_) => "Please check if the syntax is correct.",
                        SQLitendError::Step(_) => {
                            "If database disk image is malformed, please enable the discard context option and use it once."
                        }
                        SQLitendError::UnsupportColumnType(_) => {
                            "An unsupported type was encountered, please create an issue on github."
                        }
                    },
                    WorkerError::NotFound | WorkerError::Unexpected => {
                        "This shouldn't happen, please create an issue on github."
                    }
                    WorkerError::InvaildState => {
                        "SQLite is in an abnormal state when executing SQLite."
                    }
                    WorkerError::LoadDb(_) => {
                        "Please check whether the imported DB is a SQLite3 file"
                    }
                    WorkerError::DownloadDb(_) => "It may be caused by OOM",
                    WorkerError::OpfsSAHPoolOpened => OPFS_SAH_POOL_OPENED_DETAILS,
                },
                SQLightError::AceEditor(ace_editor) => match ace_editor {
                    aceditor::EditorError::Serde(_)
                    | aceditor::EditorError::SetTheme(_)
                    | aceditor::EditorError::SetKeyboardHandler(_)
                    | aceditor::EditorError::Open(_)
                    | aceditor::EditorError::DefineEx(_) => {
                        "This shouldn't happen, please create an issue on github."
                    }
                },
                SQLightError::ImportDb(_) => {
                    "Maybe the db was not found, could not be read, or was too large."
                }
            };

            view! {
                <details open>
                    <summary>{summary}</summary>
                    <p>{details}</p>
                </details>
            }
            .into_any()
        }
        None => view! { "No Error" }.into_any(),
    };

    let import_progress = move || {
        if let Some(progress) = &*state.import_progress().read() {
            let filename = format!("Filename: {}", progress.filename);
            let loading = format!("Loading: {} of {} bytes", progress.loaded, progress.total);

            let status = if progress.loaded == progress.total {
                view! { <p>"Loading completed"</p> }.into_any()
            } else {
                ().into_any()
            };

            let process = if progress.loaded == progress.total {
                match &progress.opened {
                    Some(success) => {
                        if *success {
                            view! { <p>"Processing completed"</p> }.into_any()
                        } else {
                            view! { <p>"Processing failed"</p> }.into_any()
                        }
                    }
                    None => view! { <p>"Processing..."</p> }.into_any(),
                }
            } else {
                ().into_any()
            };

            view! {
                <p>{filename}</p>
                <p>{loading}</p>
                {status}
                {process}
            }
            .into_any()
        } else {
            view! { "No files are being imported." }.into_any()
        }
    };

    view! {
        <SimplePane>
            <Section label="Last Error".into()>
                <pre style="white-space: pre-wrap;">{last_error}</pre>
            </Section>
            <Section label="Import Progress".into()>
                <pre style="white-space: pre-wrap;">{import_progress}</pre>
            </Section>
        </SimplePane>
    }
}
