use js_sys::{Object, Reflect};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};

// <https://ajaxorg.github.io/ace-api-docs/index.html>
mod bindgen {
    use js_sys::Object;
    use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ace, js_name = edit, catch)]
        pub fn edit(element: &str, options: Option<Object>) -> Result<Editor, JsValue>;
        #[wasm_bindgen(js_namespace = ace, js_name = require, catch)]
        pub fn require(module: &str) -> Result<JsValue, JsValue>;
    }

    #[wasm_bindgen]
    extern "C" {
        pub type Editor;

        #[wasm_bindgen(method, js_name = setTheme, catch)]
        pub fn set_theme(this: &Editor, theme: &str) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = setKeyboardHandler, catch)]
        pub fn set_keyboard_handler(this: &Editor, handler: JsValue) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = getValue)]
        pub fn get_value(this: &Editor) -> String;

        #[wasm_bindgen(method, js_name = clearSelection)]
        pub fn clear_selection(this: &Editor);

        #[wasm_bindgen(method, js_name = setValue)]
        pub fn set_value(this: &Editor, value: String);

        #[wasm_bindgen(method, js_name = getSession)]
        pub fn get_session(this: &Editor) -> EditSession;

        #[wasm_bindgen(method, js_name = getSelection)]
        pub fn get_selection(this: &Editor) -> Selection;

        #[wasm_bindgen(method, js_name = getSelectedText)]
        pub fn get_selected_text(this: &Editor) -> String;

        #[wasm_bindgen(method, js_name = setReadOnly)]
        pub fn set_read_only(this: &Editor, value: bool);
    }

    #[wasm_bindgen]
    extern "C" {
        pub type Selection;

        #[wasm_bindgen(method, js_name = getRange)]
        pub fn get_range(this: &Selection) -> JsValue;
    }

    #[wasm_bindgen]
    extern "C" {
        pub type EditSession;

        #[wasm_bindgen(method, js_name = setValue)]
        pub fn set_value(this: &EditSession, value: String);

        #[wasm_bindgen(method, js_name = getLength)]
        pub fn get_length(this: &EditSession) -> usize;

        #[wasm_bindgen(method, js_name = getTextRange)]
        pub fn get_text_range(this: &EditSession, range: JsValue) -> String;

        #[wasm_bindgen(method, js_name = getLine)]
        pub fn get_line(this: &EditSession, row: usize) -> String;

    }

    #[wasm_bindgen]
    extern "C" {
        pub type CommandManager;

        #[wasm_bindgen(method, js_name = addCommand)]
        pub fn add_command(this: &CommandManager, command: JsValue);

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

    pub fn keyboard(mut self, value: Option<&str>) -> Self {
        let value = if let Some(value) = value {
            JsValue::from(value)
        } else {
            JsValue::null()
        };
        self.0.keyboard_handler = value;
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Range {
    pub start: Point,
    pub end: Point,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Point {
    pub row: usize,
    pub column: usize,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorOptions {
    pub mode: String,
    pub theme: String,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub keyboard_handler: JsValue,
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

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Command {
    pub name: String,
    pub bind_key: BindKey,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub exec: js_sys::Function,
    pub read_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BindKey {
    pub win: String,
    pub mac: String,
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

    pub fn set_keyboard_handler(&self, handler: Option<&str>) -> Result<()> {
        let handler = if let Some(handler) = handler {
            JsValue::from(handler)
        } else {
            JsValue::null()
        };

        self.js
            .set_keyboard_handler(handler)
            .map_err(EditorError::SetKeyboardHandler)
    }

    pub fn define_vim_w(callback: Box<dyn Fn() + 'static>) -> Result<()> {
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

    pub fn add_command(&self, command: Command) {
        let editor = self.js.clone();
        let manager = bindgen::CommandManager::from(
            Reflect::get(&editor, &JsValue::from("commands")).unwrap(),
        );
        manager.add_command(serde_wasm_bindgen::to_value(&command).unwrap());
    }

    pub fn get_value(&self) -> String {
        self.js.get_value()
    }

    pub fn set_value(&self, value: String) {
        self.js.set_value(value);
        self.js.clear_selection();
    }

    pub fn get_range(&self) -> Range {
        serde_wasm_bindgen::from_value(self.js.get_selection().get_range()).unwrap()
    }

    pub fn get_selected_value(&self) -> String {
        self.js.get_selected_text()
    }

    pub fn set_read_only(&self, value: bool) {
        self.js.set_read_only(value);
    }

    pub fn get_length(&self) -> usize {
        self.js.get_session().get_length()
    }

    pub fn get_line(&self, row: usize) -> String {
        self.js.get_session().get_line(row)
    }

    pub fn get_text_range(&self, range: Range) -> String {
        self.js
            .get_session()
            .get_text_range(serde_wasm_bindgen::to_value(&range).unwrap())
    }
}
