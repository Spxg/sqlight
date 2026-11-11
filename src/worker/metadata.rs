use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::WorkerError;

#[wasm_bindgen(inline_js = "
export async function sqlightInitializationMarker(base, filename, write) {
    const root = await navigator.storage.getDirectory();
    const directory = await root.getDirectoryHandle(base, { create: true });
    try {
        const markers = await directory.getDirectoryHandle('.sqlight-initialized', { create: write });
        await markers.getFileHandle(encodeURIComponent(filename), { create: write });
        return true;
    } catch (error) {
        if (!write && error.name === 'NotFoundError') return false;
        throw error;
    }
}
")]
extern "C" {
    #[wasm_bindgen(catch, js_name = sqlightInitializationMarker)]
    fn initialization_marker(
        base: &str,
        filename: &str,
        write: bool,
    ) -> Result<js_sys::Promise, JsValue>;
}

pub async fn initialized(filename: &str, write: bool) -> Result<bool, WorkerError> {
    let error = |error| WorkerError::Filesystem(format!("Initialization marker: {error:?}"));
    let promise = initialization_marker(super::OPFS_VFS_DIR, filename, write).map_err(error)?;
    Ok(JsFuture::from(promise)
        .await
        .map_err(error)?
        .as_bool()
        .unwrap())
}
