use sqlight::{WorkerRequest, WorkerResponse, worker};
use tokio::sync::mpsc::UnboundedReceiver;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::spawn_local;
use web_sys::{DedicatedWorkerGlobalScope, MessageEvent};

fn main() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).unwrap();

    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<JsValue>();

    let scope: DedicatedWorkerGlobalScope = JsValue::from(js_sys::global()).into();
    spawn_local(execute_task(scope.clone(), rx));

    let on_message = Closure::<dyn Fn(MessageEvent)>::new(move |ev: MessageEvent| {
        tx.send(ev.data()).unwrap();
    });

    scope.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    scope
        .post_message(&serde_wasm_bindgen::to_value(&WorkerResponse::Ready).unwrap())
        .expect("Faild to send ready to window");
    on_message.forget();
}

async fn execute_task(scope: DedicatedWorkerGlobalScope, mut rx: UnboundedReceiver<JsValue>) {
    while let Some(request) = rx.recv().await {
        let request = serde_wasm_bindgen::from_value::<WorkerRequest>(request).unwrap();
        let resp = match request {
            WorkerRequest::Open(options) => WorkerResponse::Open(worker::open(options).await),
            WorkerRequest::Prepare(options) => {
                WorkerResponse::Prepare(worker::prepare(options).await)
            }
            WorkerRequest::Continue => WorkerResponse::Continue(worker::r#continue().await),
            WorkerRequest::StepOver => WorkerResponse::StepOver(worker::step_over().await),
            WorkerRequest::StepIn => WorkerResponse::StepIn(worker::step_in().await),
            WorkerRequest::StepOut => WorkerResponse::StepOut(worker::step_out().await),
            WorkerRequest::LoadDb(options) => {
                WorkerResponse::LoadDb(worker::load_db(options).await)
            }
            WorkerRequest::DownloadDb => WorkerResponse::DownloadDb(worker::download_db().await),
        };
        if let Err(err) = scope.post_message(&serde_wasm_bindgen::to_value(&resp).unwrap()) {
            log::error!("Failed to send task to window: {resp:?}, {err:?}");
        }
    }
}
