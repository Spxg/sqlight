use wasm_bindgen::{JsValue, prelude::wasm_bindgen};

#[wasm_bindgen]
extern "C" {
    pub type Grid;

    #[wasm_bindgen(js_namespace = ["globalThis"], js_name = Split)]
    pub fn split(options: &JsValue) -> Grid;

    #[wasm_bindgen(method)]
    pub fn destroy(this: &Grid);
}

unsafe impl Send for Grid {}
unsafe impl Sync for Grid {}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Gutter {
    pub track: i32,
    #[serde(with = "serde_wasm_bindgen::preserve")]
    pub element: JsValue,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SplitOptions {
    pub min_size: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_gutters: Option<Vec<Gutter>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_gutters: Option<Vec<Gutter>>,
}

impl From<SplitOptions> for JsValue {
    fn from(value: SplitOptions) -> Self {
        serde_wasm_bindgen::to_value(&value).unwrap()
    }
}
