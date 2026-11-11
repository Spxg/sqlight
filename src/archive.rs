use js_sys::{Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::DatabaseFile;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = SQLightArchive, js_name = unpackZip, catch)]
    fn unpack_zip_files(data: Uint8Array) -> Result<Promise, JsValue>;

    #[wasm_bindgen(js_namespace = SQLightArchive, js_name = packZip, catch)]
    fn pack_zip_files(files: JsValue) -> Result<Promise, JsValue>;
}

pub(crate) async fn unpack_zip(data: Uint8Array) -> Result<Vec<DatabaseFile>, JsValue> {
    let files = JsFuture::from(unpack_zip_files(data)?).await?;
    serde_wasm_bindgen::from_value(files).map_err(|error| JsValue::from_str(&error.to_string()))
}

pub(crate) async fn pack_zip(files: &[DatabaseFile]) -> Result<Uint8Array, JsValue> {
    let files = serde_wasm_bindgen::to_value(files)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;
    let data = JsFuture::from(pack_zip_files(files)?).await?;
    Ok(Uint8Array::new(&data))
}
