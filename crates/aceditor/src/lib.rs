use js_sys::Object;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};

// <https://ajaxorg.github.io/ace-api-docs/index.html>
mod bindgen {
    use js_sys::{Function, Object};
    use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ace, js_name = edit, catch)]
        pub fn edit(element: &str, options: Option<Object>) -> Result<Editor, JsValue>;
    }

    #[wasm_bindgen]
    extern "C" {
        #[derive(Clone)]
        pub type Editor;

        #[wasm_bindgen(method, js_name = setTheme, catch)]
        pub fn set_theme(this: &Editor, theme: &str) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = setKeyboardHandler, catch)]
        pub fn set_keyboard_handler(this: &Editor, handler: JsValue) -> Result<(), JsValue>;

        #[wasm_bindgen(method, js_name = getValue)]
        pub fn get_value(this: &Editor) -> String;

        #[wasm_bindgen(method, js_name = setValue)]
        pub fn set_value(this: &Editor, value: &str, cursor_position: i32);

        #[wasm_bindgen(method)]
        pub fn on(this: &Editor, event: &str, callback: &Function);

        #[wasm_bindgen(method)]
        pub fn off(this: &Editor, event: &str, callback: &Function);

        #[wasm_bindgen(method)]
        pub fn destroy(this: &Editor);

        #[wasm_bindgen(method)]
        pub fn resize(this: &Editor);

        #[wasm_bindgen(method, getter)]
        pub fn commands(this: &Editor) -> CommandManager;

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

        #[wasm_bindgen(method, js_name = getLength)]
        pub fn get_length(this: &EditSession) -> usize;

        #[wasm_bindgen(method, js_name = getTextRange)]
        pub fn get_text_range(this: &EditSession, range: JsValue) -> String;

        #[wasm_bindgen(method, js_name = getLine)]
        pub fn get_line(this: &EditSession, row: usize) -> String;

    }

    #[wasm_bindgen(inline_js = r#"
export function defineVimWrite() {
    const { CodeMirror } = ace.require("ace/keyboard/vim");
    CodeMirror.Vim.defineEx("write", "w", cm => cm.ace.execCommand("executeCode"));
}

export function bindCommandShortcut(editor, command) {
    const { HashHandler } = ace.require("ace/keyboard/hash_handler");
    const { keyCodeToString } = ace.require("ace/lib/keys");
    const shortcuts = new HashHandler([command], editor.commands.platform);
    const onKeyDown = event => {
        const modifiers = (event.ctrlKey ? 1 : 0)
            | (event.altKey ? 2 : 0)
            | (event.shiftKey ? 4 : 0)
            | (event.metaKey ? 8 : 0);
        if (!shortcuts.findKeyCommand(modifiers, keyCodeToString(event.keyCode))) return;
        event.preventDefault();
        event.stopPropagation();
        editor.execCommand(command.name);
    };
    // Capture application shortcuts before an editor preset handles the key.
    editor.container.addEventListener("keydown", onKeyDown, true);
    return () => editor.container.removeEventListener("keydown", onKeyDown, true);
}
"#)]
    extern "C" {
        #[wasm_bindgen(js_name = defineVimWrite, catch)]
        pub fn define_vim_write() -> Result<(), JsValue>;

        #[wasm_bindgen(js_name = bindCommandShortcut)]
        pub fn bind_command_shortcut(editor: &Editor, command: &JsValue) -> Function;
    }

    #[wasm_bindgen]
    extern "C" {
        pub type CommandManager;

        #[wasm_bindgen(method, js_name = addCommand)]
        pub fn add_command(this: &CommandManager, command: JsValue);

    }
}

type Result<T> = std::result::Result<T, EditorError>;

#[derive(Default)]
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
            keyboard_handler: JsValue::null(),
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
    #[error("Failed to define Vim extension")]
    DefineEx(JsValue),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Command {
    name: String,
    bind_key: BindKey,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    exec: js_sys::Function,
    read_only: bool,
}

#[derive(Serialize, Deserialize)]
pub struct BindKey {
    pub win: String,
    pub mac: String,
}

pub struct Editor {
    js: bindgen::Editor,
    change_listener: Option<Closure<dyn Fn()>>,
    command_callbacks: Vec<Closure<dyn Fn()>>,
    command_shortcuts: Vec<js_sys::Function>,
}

impl Editor {
    /// ace.edit
    pub fn open(element: &str, options: Option<&EditorOptions>) -> Result<Self> {
        let editor = bindgen::edit(element, options.map(|options| options.to_js()))
            .map_err(EditorError::Open)?;
        Ok(Editor {
            js: editor,
            change_listener: None,
            command_callbacks: Vec::new(),
            command_shortcuts: Vec::new(),
        })
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

    pub fn define_vim_w() -> Result<()> {
        bindgen::define_vim_write().map_err(EditorError::DefineEx)
    }

    pub fn add_command(&mut self, name: &str, bind_key: BindKey, callback: impl Fn() + 'static) {
        let callback = Closure::<dyn Fn()>::new(callback);
        let command = Command {
            name: name.into(),
            bind_key,
            exec: callback
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .clone(),
            read_only: true,
        };
        let command = serde_wasm_bindgen::to_value(&command).unwrap();
        self.js.commands().add_command(command.clone());
        self.command_shortcuts
            .push(bindgen::bind_command_shortcut(&self.js, &command));
        self.command_callbacks.push(callback);
    }

    pub fn on_change(&mut self, callback: impl Fn(String) + 'static) {
        if let Some(listener) = self.change_listener.take() {
            self.js.off("change", listener.as_ref().unchecked_ref());
        }
        let editor = self.js.clone();
        let listener = Closure::<dyn Fn()>::new(move || callback(editor.get_value()));
        self.js.on("change", listener.as_ref().unchecked_ref());
        self.change_listener = Some(listener);
    }

    pub fn resize(&self) {
        self.js.resize();
    }

    pub fn get_value(&self) -> String {
        self.js.get_value()
    }

    pub fn set_value(&self, value: String) {
        self.js.set_value(&value, -1);
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

impl Drop for Editor {
    fn drop(&mut self) {
        if let Some(listener) = &self.change_listener {
            self.js.off("change", listener.as_ref().unchecked_ref());
        }
        for remove_shortcut in &self.command_shortcuts {
            remove_shortcut.call0(&JsValue::NULL).unwrap();
        }
        self.js.destroy();
    }
}
