use std::rc::Rc;

use leptos::prelude::*;
use reactive_stores::Store;
use tokio::sync::{Mutex, mpsc};
use wasm_bindgen::{JsCast, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{ErrorEvent, MessageEvent, Worker, WorkerOptions, WorkerType};

use crate::{
    OpenOptions, SQLightError, SQLiteRunResult, SendWrapper, WorkerError, WorkerRequest,
    WorkerResponse,
    app::{
        Engine, Exported, Focus, GlobalState, GlobalStateStoreFields, Vfs, change_focus,
        embed_query_result,
    },
};

type Result<T> = std::result::Result<T, WorkerError>;
type Session = Rc<Mutex<Option<WorkerSession>>>;

thread_local! {
    static SQLITE: Session = Session::default();
    static SQLITE_MC: Session = Session::default();
    static SQLCIPHER: Session = Session::default();
}

struct WorkerSession {
    worker: Worker,
    responses: mpsc::UnboundedReceiver<Result<WorkerResponse>>,
    persist: Option<bool>,
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
            persist: None,
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

    async fn connect(&mut self, persist: bool) -> Result<()> {
        if self.persist == Some(persist) {
            return Ok(());
        }
        self.persist = None;
        match self
            .request(WorkerRequest::Open(OpenOptions {
                filename: "test.db".into(),
                persist,
            }))
            .await?
        {
            WorkerResponse::Open(result) => {
                result?;
                self.persist = Some(persist);
                Ok(())
            }
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
    if running && state.is_running().get_untracked() {
        return;
    }
    if running {
        state.is_running().set(true);
    }
    state.last_error().set(None);

    let embed_sql = match &request {
        WorkerRequest::Run(options) if options.embed => Some(options.sql.clone()),
        _ => None,
    };
    let persist = state.vfs().get_untracked() == Vfs::OPFS;
    let (session, uri) = match state.engine().get_untracked() {
        Engine::SQLite => (SQLITE.with(Clone::clone), "./sqlite3_loader.js"),
        Engine::MultipleCiphers => (SQLITE_MC.with(Clone::clone), "./sqlite3mc_loader.js"),
        Engine::SQLCipher => (SQLCIPHER.with(Clone::clone), "./sqlcipher_loader.js"),
    };
    let importing = matches!(request, WorkerRequest::LoadDb(_));

    spawn_local(async move {
        let mut session = session.lock().await;
        let response = async {
            if session.is_none() {
                *session = Some(WorkerSession::start(uri).await?);
            }
            let worker = session.as_mut().unwrap();
            worker.connect(persist).await?;
            worker.request(request).await
        }
        .await;

        if matches!(response, Err(WorkerError::Transport(_))) {
            *session = None;
        }
        if running {
            state.is_running().set(false);
        }
        match response {
            Ok(response) => handle_response(state, response, embed_sql.as_deref()),
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
    });
}

fn handle_response(state: Store<GlobalState>, response: WorkerResponse, embed_sql: Option<&str>) {
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
            if success {
                state.keep_ctx().set(true);
            }
            if let Err(err) = result {
                state.last_error().set(Some(SQLightError::new_worker(err)));
            }
        }
        WorkerResponse::DownloadDb(result) => match result {
            Ok(response) => state.exported().set(Some(Exported {
                filename: response.filename,
                data: SendWrapper::new(response.data),
            })),
            Err(err) => state.last_error().set(Some(SQLightError::new_worker(err))),
        },
        WorkerResponse::Open(_) | WorkerResponse::Ready => unreachable!(),
    }
}
