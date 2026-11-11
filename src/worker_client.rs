use std::{cell::RefCell, collections::HashMap, rc::Rc};

use leptos::prelude::*;
use reactive_stores::Store;
use tokio::sync::{Mutex, mpsc};
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType};

use crate::{
    FileSnapshot, OpenOptions, SQLightError, SQLiteRunResult, SendWrapper, WorkerError,
    WorkerRequest, WorkerResponse,
    app::{
        Engine, Exported, Focus, GlobalState, GlobalStateStoreFields, change_focus,
        embed_query_result,
    },
};

type Result<T> = std::result::Result<T, WorkerError>;
type Session = Rc<Mutex<Option<WorkerSession>>>;

thread_local! {
    static SESSIONS: RefCell<HashMap<Engine, Session>> = RefCell::default();
}

struct WorkerSession {
    worker: Worker,
    responses: mpsc::UnboundedReceiver<Result<WorkerResponse>>,
    opened: Option<OpenOptions>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
}

impl WorkerSession {
    async fn start(uri: &str) -> Result<Self> {
        let options = WorkerOptions::new();
        options.set_type(WorkerType::Module);
        let worker = Worker::new_with_options(uri, &options).map_err(transport_error)?;
        let (tx, responses) = mpsc::unbounded_channel();
        let errors = tx.clone();
        let on_message = Closure::new(move |event: MessageEvent| {
            let response = serde_wasm_bindgen::from_value(event.data())
                .map_err(|err| WorkerError::Transport(err.to_string()));
            let _ = tx.send(response);
        });
        let on_error = Closure::new(move |event: ErrorEvent| {
            let _ = errors.send(Err(WorkerError::Transport(event.message())));
        });
        worker.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        worker.set_onerror(Some(on_error.as_ref().unchecked_ref()));

        let mut session = Self {
            worker,
            responses,
            opened: None,
            _on_message: on_message,
            _on_error: on_error,
        };
        match session.receive().await? {
            WorkerResponse::Ready => Ok(session),
            _ => Err(WorkerError::Unexpected),
        }
    }

    async fn receive(&mut self) -> Result<WorkerResponse> {
        self.responses
            .recv()
            .await
            .ok_or_else(|| WorkerError::Transport("Worker disconnected".into()))?
    }

    async fn request(&mut self, request: WorkerRequest) -> Result<WorkerResponse> {
        let value = serde_wasm_bindgen::to_value(&request)
            .map_err(|err| WorkerError::Transport(err.to_string()))?;
        self.worker.post_message(&value).map_err(transport_error)?;
        self.receive().await
    }

    async fn connect(&mut self, options: OpenOptions) -> Result<()> {
        if self.opened.as_ref() == Some(&options) {
            return Ok(());
        }
        match self.request(WorkerRequest::Open(options.clone())).await? {
            WorkerResponse::Open(Ok(())) => {
                self.opened = Some(options);
                Ok(())
            }
            WorkerResponse::Open(Err(error)) => Err(error),
            _ => Err(WorkerError::Unexpected),
        }
    }
}

impl Drop for WorkerSession {
    fn drop(&mut self) {
        self.worker.set_onmessage(None);
        self.worker.set_onerror(None);
        self.worker.terminate();
    }
}

fn transport_error(error: wasm_bindgen::JsValue) -> WorkerError {
    WorkerError::Transport(format!("{error:?}"))
}

pub(crate) fn send_request(state: Store<GlobalState>, request: WorkerRequest) {
    let running = matches!(request, WorkerRequest::Run(_));
    let importing = matches!(
        request,
        WorkerRequest::LoadDb(_) | WorkerRequest::LoadDbs(_)
    );
    if state.is_busy().get_untracked() && !importing {
        return;
    }
    state.is_busy().set(true);
    if running {
        state.is_running().set(true);
    }
    if !matches!(request, WorkerRequest::ListDbs | WorkerRequest::ListFiles) {
        state.last_error().set(None);
    }

    let embed_sql = match &request {
        WorkerRequest::Run(options) if options.embed => Some(options.sql.clone()),
        _ => None,
    };
    let needs_connection = running
        || matches!(
            request,
            WorkerRequest::LoadDb(_) | WorkerRequest::DownloadDb
        );
    let listing = matches!(request, WorkerRequest::ListFiles);
    let workspace = state.workspace_id().get_untracked();
    let engine = state.engine().get_untracked();
    let pending_database = state.pending_database().get_untracked();
    let options = OpenOptions {
        filename: state.with_untracked(GlobalState::database_filename),
        read_only: state.read_only().get_untracked(),
        workspace: workspace.clone(),
    };
    let session =
        SESSIONS.with(|sessions| sessions.borrow_mut().entry(engine).or_default().clone());
    let uri = match engine {
        Engine::SQLite => "./sqlite3_loader.js",
        Engine::MultipleCiphers => "./sqlite3mc_loader.js",
        Engine::SQLCipher => "./sqlcipher_loader.js",
    };

    spawn_local(async move {
        let mut session = session.lock().await;
        let mut initialized = false;
        let response = async {
            if session.is_none() {
                *session = Some(WorkerSession::start(uri).await?);
            }
            let worker = session.as_mut().unwrap();
            if needs_connection {
                match worker.connect(options.clone()).await {
                    Err(WorkerError::MissingDatabase(_))
                        if running && (options.filename == "default.db" || pending_database) =>
                    {
                        let initialization = if options.filename == "default.db" {
                            WorkerRequest::BootstrapDefault
                        } else {
                            WorkerRequest::CreateScratch(options.filename.clone())
                        };
                        let result = match worker.request(initialization).await? {
                            WorkerResponse::BootstrapDefault(result) => result,
                            WorkerResponse::CreateScratch(result) => result.map(|_| ()),
                            _ => return Err(WorkerError::Unexpected),
                        };
                        initialized = pending_database
                            && (result.is_ok()
                                || matches!(result, Err(WorkerError::MissingDatabase(_))));
                        result?;
                        worker.connect(options).await?;
                    }
                    result => result?,
                }
                if running && pending_database {
                    initialized = true;
                }
            }
            worker.request(request).await
        }
        .await;

        // The filesystem snapshot also supplies the actual connection after an import,
        // failed switch, Fresh run, or explicit disconnect.
        let mut snapshot_error = None;
        let snapshot = if !listing
            && !matches!(response, Err(WorkerError::Transport(_)))
            && let Some(worker) = session.as_mut()
        {
            match worker.request(WorkerRequest::ListFiles).await {
                Ok(WorkerResponse::ListFiles(Ok(snapshot))) => {
                    worker.opened = snapshot.connection.clone();
                    Some(snapshot)
                }
                response => {
                    worker.opened = None;
                    snapshot_error = Some(match response {
                        Ok(WorkerResponse::ListFiles(Err(error))) | Err(error) => error,
                        _ => WorkerError::Unexpected,
                    });
                    None
                }
            }
        } else {
            None
        };
        if listing && let Some(worker) = session.as_mut() {
            worker.opened = match &response {
                Ok(WorkerResponse::ListFiles(Ok(snapshot))) => snapshot.connection.clone(),
                _ => None,
            };
        }
        if matches!(response, Err(WorkerError::Transport(_)))
            || matches!(snapshot_error, Some(WorkerError::Transport(_)))
        {
            *session = None;
        }
        if state.workspace_id().get_untracked() == workspace
            && state.engine().get_untracked() == engine
        {
            if initialized {
                state.pending_database().set(false);
                state.read_untracked().save();
            }
            if let Some(snapshot) = snapshot {
                apply_snapshot(state, snapshot);
            }
            if needs_connection
                && matches!(response, Err(WorkerError::ActiveTransaction))
                && let Some(options) = session.as_ref().and_then(|worker| worker.opened.as_ref())
            {
                batch(move || {
                    state.database().set(options.filename.clone());
                    state.pending_database().set(false);
                    state.read_only().set(options.read_only);
                    state.keep_ctx().set(true);
                });
                state.read_untracked().save();
            }
            if snapshot_error.is_some()
                || listing && !matches!(response, Ok(WorkerResponse::ListFiles(Ok(_))))
            {
                batch(move || {
                    state.files().set(None);
                    state.databases().set(Vec::new());
                });
            }
            match response {
                Ok(response) => {
                    handle_response(state, response, embed_sql.as_deref()).await;
                }
                Err(error) => {
                    if importing {
                        state.import_progress().update(|progress| {
                            if let Some(progress) = progress {
                                progress.opened = Some(false);
                            }
                        });
                    }
                    state
                        .last_error()
                        .set(Some(SQLightError::new_worker(error)));
                }
            }
            if let Some(error) = snapshot_error
                && state.last_error().with_untracked(|error| error.is_none())
            {
                state
                    .last_error()
                    .set(Some(SQLightError::new_worker(error)));
            }
        }
        batch(move || {
            if running {
                state.is_running().set(false);
            }
            state.is_busy().set(false);
        });
    });
}

fn apply_snapshot(state: Store<GlobalState>, snapshot: FileSnapshot) {
    let databases = snapshot
        .files
        .iter()
        .filter(|file| {
            !["-journal", "-wal", "-shm"]
                .iter()
                .any(|suffix| file.filename.ends_with(suffix))
        })
        .map(|file| file.filename.clone())
        .collect();
    batch(move || {
        state.databases().set(databases);
        state.files().set(Some(snapshot));
    });
}

async fn handle_response(
    state: Store<GlobalState>,
    response: WorkerResponse,
    embed_sql: Option<&str>,
) {
    match response {
        WorkerResponse::Run(result) => match result {
            Ok(SQLiteRunResult {
                embed,
                result,
                error,
            }) => {
                let can_embed = embed
                    && error.is_none()
                    && state.editor().with_untracked(|editor| {
                        editor.as_ref().is_some_and(|editor| {
                            embed_sql.is_some_and(|sql| editor.get_value() == sql)
                        })
                    });
                if can_embed {
                    let sql = embed_query_result(embed_sql.unwrap(), &result);
                    state.editor().with_untracked(|editor| {
                        editor.as_ref().unwrap().set_value(sql);
                    });
                } else {
                    state.output().set(result);
                    if let Some(error) = error {
                        change_focus(state, Some(Focus::Execute));
                        state
                            .last_error()
                            .set(Some(SQLightError::new_worker(error.into())));
                    } else if embed {
                        change_focus(state, Some(Focus::Execute));
                        state
                            .last_error()
                            .set(Some(SendWrapper::new(SQLightError::EmbedChanged)));
                    }
                }
            }
            Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
        },
        WorkerResponse::LoadDb(result) => {
            let success = result.is_ok();
            state.import_progress().update(|progress| {
                if let Some(progress) = progress {
                    progress.opened = Some(success);
                }
            });
            if let Err(err) = result {
                state.last_error().set(Some(SQLightError::new_worker(err)));
            }
        }
        WorkerResponse::LoadDbs(result) => {
            state.import_progress().update(|progress| {
                if let Some(progress) = progress {
                    progress.opened = Some(result.is_ok());
                }
            });
            match result {
                Ok(filenames) => {
                    state.databases().update(|databases| {
                        for filename in filenames {
                            if !databases.contains(&filename) {
                                databases.push(filename);
                            }
                        }
                        databases.sort();
                    });
                }
                Err(error) => state
                    .last_error()
                    .set(Some(SQLightError::new_worker(error))),
            }
        }
        WorkerResponse::ListDbs(result) => match result {
            Ok(databases) => state.databases().set(databases),
            Err(error) => state
                .last_error()
                .set(Some(SQLightError::new_worker(error))),
        },
        WorkerResponse::ListFiles(result) => match result {
            Ok(snapshot) => apply_snapshot(state, snapshot),
            Err(error) => state
                .last_error()
                .set(Some(SQLightError::new_worker(error))),
        },
        WorkerResponse::CreateDb(result) | WorkerResponse::CreateScratch(result) => {
            if let Err(error) = result {
                state
                    .last_error()
                    .set(Some(SQLightError::new_worker(error)));
            }
        }
        WorkerResponse::DeleteFiles(result) => match result {
            Ok(deleted) => {
                if deleted.contains(&state.database().get_untracked()) {
                    state.pending_database().set(false);
                }
                state.read_untracked().save();
            }
            Err(error) => state
                .last_error()
                .set(Some(SQLightError::new_worker(error))),
        },
        WorkerResponse::Close(result) | WorkerResponse::BootstrapDefault(result) => {
            if let Err(error) = result {
                state
                    .last_error()
                    .set(Some(SQLightError::new_worker(error)));
            }
        }
        WorkerResponse::DownloadDb(result) => match result {
            Ok(response) => {
                state.exported().set(Some(Exported {
                    filename: response.filename,
                    mime_type: "application/vnd.sqlite3",
                    data: SendWrapper::new(response.data),
                }));
            }
            Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
        },
        WorkerResponse::DownloadBundle(result) => match result {
            Ok(response) => match crate::archive::pack_zip(&response.files).await {
                Ok(data) => state.exported().set(Some(Exported {
                    filename: response.filename,
                    mime_type: "application/zip",
                    data: SendWrapper::new(data),
                })),
                Err(error) => {
                    state
                        .last_error()
                        .set(Some(SendWrapper::new(SQLightError::ExportDb(format!(
                            "{error:?}"
                        )))))
                }
            },
            Err(error) => state
                .last_error()
                .set(Some(SQLightError::new_worker(error))),
        },
        WorkerResponse::Open(_) | WorkerResponse::CloseWorkspace(_) | WorkerResponse::Ready => {
            unreachable!()
        }
    }
}
