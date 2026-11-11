use leptos::prelude::*;
use reactive_stores::Store;

use crate::{
    SQLightError, SQLitendError, WorkerError,
    app::{GlobalState, GlobalStateStoreFields, output::section::Section},
};

fn error_details(error: &SQLightError) -> &'static str {
    match error {
        SQLightError::Worker(error) => match error {
            WorkerError::SQLite(error) => match error {
                SQLitendError::ToCStr => "Remove null bytes from the SQL or database filename.",
                SQLitendError::Utf8Text => "Some result values cannot be displayed as UTF-8 text.",
                SQLitendError::GetColumnName(_) | SQLitendError::UnsupportedColumnType(_) => {
                    "Please report this error on GitHub."
                }
                SQLitendError::OpenDb(error)
                | SQLitendError::Prepare(error)
                | SQLitendError::Step(error)
                    if error.code == 8 =>
                {
                    "This workspace is read only. Turn off Read only in its connection \
                    settings to allow SQL writes."
                }
                SQLitendError::OpenDb(error)
                | SQLitendError::Prepare(error)
                | SQLitendError::Step(error)
                    if error.code == 26 =>
                {
                    "Check the SQLite engine, encryption key, and cipher settings."
                }
                SQLitendError::OpenDb(_) => {
                    "Check the selected main database in the workspace settings."
                }
                SQLitendError::Prepare(_) => {
                    "Check the SQL syntax. Earlier statements may have completed; \
                    failed batches are not automatically rolled back."
                }
                SQLitendError::Step(_) => {
                    "Earlier statements may have completed; failed batches are not \
                    automatically rolled back."
                }
            },
            WorkerError::NotOpened => {
                "Select a main database in the workspace settings and run the query again."
            }
            WorkerError::Unexpected => "Please report this error on GitHub.",
            WorkerError::InvalidState => {
                "Enable Fresh database per run to rebuild the selected main database, then run again."
            }
            WorkerError::LoadDb(_) => "Check that the imported file is a SQLite database.",
            WorkerError::CreateDb(_) => {
                "Choose an unused database filename. Existing files are not overwritten."
            }
            WorkerError::DownloadDb(_) => {
                "Commit or roll back any active transaction, then try downloading again."
            }
            WorkerError::ActiveTransaction => {
                "Previous database settings were restored. Run COMMIT or ROLLBACK, \
                then try again."
            }
            WorkerError::OpfsSAHPoolOpened => {
                "OPFS storage is already open in another browser tab. \
                Close that tab and try again."
            }
            WorkerError::Transport(_) => {
                "Reload the page and check the browser console for details."
            }
        },
        SQLightError::AceEditor(_) => "Please report this error on GitHub.",
        SQLightError::ImportDb(_) => {
            "The file could not be read. Check that it is accessible and fits in memory."
        }
        SQLightError::EmbedChanged => {
            "Your current SQL was preserved. Open the Execution tab to view the results."
        }
    }
}

#[component]
pub fn Status() -> impl IntoView {
    let state = expect_context::<Store<GlobalState>>();

    let last_error = move || match state.last_error().read().as_deref() {
        Some(error) => {
            let summary = error.to_string();
            let details = error_details(error);
            view! {
                <details open>
                    <summary style="white-space: pre-wrap;">{summary}</summary>
                    <p>{details}</p>
                </details>
            }
            .into_any()
        }
        None => view! { <p>"No errors."</p> }.into_any(),
    };

    let import_progress = move || match &*state.import_progress().read() {
        Some(progress) => {
            let filename = progress.filename.clone();
            let bytes = format!(
                "Read: {:.0} of {:.0} bytes",
                progress.loaded, progress.total
            );
            let status = match progress.opened {
                Some(true) => "Database imported.",
                Some(false) => "Import failed.",
                None if progress.loaded == progress.total => "Opening database…",
                None => "Reading file…",
            };

            view! {
                <p>{filename}</p>
                <progress
                    aria-label="Database import"
                    value=progress.loaded
                    max=progress.total
                />
                <p>{bytes}</p>
                <p role="status">{status}</p>
            }
            .into_any()
        }
        None => view! { <p>"No files are being imported."</p> }.into_any(),
    };

    view! {
        <>
            <Section label="Last Error".into()>{last_error}</Section>
            <Section label="Import Progress".into()>{import_progress}</Section>
        </>
    }
}
