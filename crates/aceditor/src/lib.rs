use std::sync::Arc;

use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};

// <https://ajaxorg.github.io/ace-api-docs/index.html>
mod bindgen {
    use js_sys::Object;
    use wasm_bindgen::{
        JsValue,
        prelude::{Closure, wasm_bindgen},
    };

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ace, js_name = edit, catch)]
        pub fn edit(element: &str, options: Option<Object>) -> Result<Editor, JsValue>;

        #[wasm_bindgen(js_namespace = ["ace", "config"], js_name = loadModule)]
        pub fn load_module(module: &str, callback: &Closure<dyn FnMut(JsValue)>);

        #[wasm_bindgen(js_namespace = ace, js_name = require, catch)]
        pub fn require(module: &str) -> Result<JsValue, JsValue>;
    }

    #[wasm_bindgen]
    extern "C" {
        pub type Editor;

        #[wasm_bindgen(method, js_name = setTheme, catch)]
        pub fn set_theme(this: &Editor, theme: &str) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = setKeyboardHandler, catch)]
        pub fn set_keyboard_handler(this: &Editor, handler: &str) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = getValue)]
        pub fn get_value(this: &Editor) -> String;

        #[wasm_bindgen(method, js_name = getSelectedText)]
        pub fn get_selected_text(this: &Editor) -> String;
    }
}

type Result<T> = std::result::Result<T, EditorError>;

#[derive(Default, Serialize, Deserialize)]
pub struct EditorOptionsBuilder(EditorOptions);

impl EditorOptionsBuilder {
    pub fn mode(mut self, value: &str) -> Self {
        self.0.mode = value.into();
        self
    }

    pub fn theme(mut self, value: &str) -> Self {
        self.0.theme = value.into();
        self
    }

    pub fn keyboard(mut self, value: &str) -> Self {
        self.0.keyboard_handler = value.into();
        self
    }

    pub fn value(mut self, value: &str) -> Self {
        self.0.value = value.into();
        self
    }

    pub fn build(self) -> EditorOptions {
        self.0
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorOptions {
    pub mode: String,
    pub theme: String,
    pub keyboard_handler: String,
    pub value: String,
}

impl Default for EditorOptions {
    fn default() -> Self {
        EditorOptions {
            mode: "ace/mode/text".into(),
            theme: "ace/theme/textmate".into(),
            keyboard_handler: "ace/keyboard/ace".into(),
            value: String::new(),
        }
    }
}

impl EditorOptions {
    pub fn to_js(&self) -> Object {
        serde_wasm_bindgen::to_value(self).unwrap().into()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum EditorError {
    #[error(transparent)]
    Serde(#[from] serde_wasm_bindgen::Error),
    #[error("Failed to open an editor")]
    Open(JsValue),
    #[error("Failed to set theme")]
    SetTheme(JsValue),
    #[error("Failed to set keyboard handler")]
    SetKeyboardHandler(JsValue),
    #[error("Failed to define extenstion")]
    DefineEx(JsValue),
}

pub struct Editor {
    js: bindgen::Editor,
}

unsafe impl Send for Editor {}

unsafe impl Sync for Editor {}

impl Editor {
    /// ace.edit
    pub fn open(element: &str, options: Option<&EditorOptions>) -> Result<Self> {
        let editor = bindgen::edit(element, options.map(|options| options.to_js()))
            .map_err(EditorError::Open)?;
        Ok(Editor { js: editor })
    }

    pub fn set_theme(&self, theme: &str) -> Result<()> {
        self.js.set_theme(theme).map_err(EditorError::SetTheme)
    }

    pub fn set_keyboard_handler(&self, handler: &str) -> Result<()> {
        self.js
            .set_keyboard_handler(handler)
            .map_err(EditorError::SetKeyboardHandler)
    }

    pub async fn define_vim_w(callback: Box<dyn Fn() + 'static>) -> Result<()> {
        let notify = Arc::new(tokio::sync::Notify::new());
        let waiter = Arc::clone(&notify);

        let load_module = Closure::once(move |_: JsValue| {
            notify.notify_waiters();
        });

        bindgen::load_module("ace/keyboard/vim", &load_module);
        waiter.notified().await;

        let value = bindgen::require("ace/keyboard/vim").map_err(EditorError::DefineEx)?;

        let define_ex = Reflect::get(&value, &JsValue::from("CodeMirror"))
            .and_then(|code_mirror| Reflect::get(&code_mirror, &JsValue::from("Vim")))
            .and_then(|vim| Reflect::get(&vim, &JsValue::from("defineEx")))
            .map_err(EditorError::DefineEx)?;
        let callback = Closure::wrap(callback);

        js_sys::Function::from(define_ex)
            .call3(
                &JsValue::null(),
                &JsValue::from("write"),
                &JsValue::from("w"),
                callback.as_ref().unchecked_ref(),
            )
            .map_err(EditorError::DefineEx)?;
        callback.forget();

        Ok(())
    }

    pub fn get_value(&self) -> String {
        self.js.get_value()
    }

    pub fn get_selected_value(&self) -> String {
        self.js.get_selected_text()
    }
}
