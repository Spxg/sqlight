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

    let show = move || match &*state.last_error().read() {
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
                    WorkerError::NotFound | WorkerError::OpfsSAHError => {
                        "This shouldn't happen, please create an issue on github."
                    }
                    WorkerError::InvaildState => {
                        "SQLite is in an abnormal state when executing SQLite."
                    }
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

    view! {
        <SimplePane>
            <Section label="Last Error".into()>{show}</Section>
        </SimplePane>
    }
}
