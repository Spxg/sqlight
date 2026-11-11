use wasm_bindgen::{
    JsValue,
    prelude::{Closure, wasm_bindgen},
};
use web_sys::js_sys::{self, Array, Object, Reflect};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"], js_name = computePosition)]
    pub async fn compute_position(
        reference: JsValue,
        floating: JsValue,
        options: JsValue,
    ) -> JsValue;
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"])]
    pub fn offset(value: i32) -> JsValue;
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"])]
    pub fn flip() -> JsValue;
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"])]
    pub fn shift() -> JsValue;
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"])]
    pub fn arrow(options: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = ["globalThis", "FloatingUIDOM"], js_name = autoUpdate)]
    pub fn auto_update(
        reference: JsValue,
        floating: JsValue,
        callback: &Closure<dyn FnMut()>,
        options: JsValue,
    ) -> js_sys::Function;
}

#[wasm_bindgen(inline_js = r#"
export function viewportSize() {
    return globalThis.FloatingUIDOM.size({
        padding: 8,
        apply({ availableHeight, elements }) {
            elements.floating.style.maxHeight = `${Math.max(0, availableHeight)}px`;
        },
    });
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = viewportSize)]
    fn viewport_size() -> JsValue;
}

#[derive(serde::Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ComputePosition {
    pub x: f64,
    pub y: f64,
    pub placement: String,
    pub strategy: String,
    pub middleware_data: MiddlewareData,
}

#[derive(serde::Deserialize, Debug)]
pub struct MiddlewareData {
    pub arrow: ArrowPosition,
}

#[derive(serde::Deserialize, Debug)]
pub struct ArrowPosition {
    pub x: Option<f64>,
}

pub fn compute_options(offset_value: i32, element: &JsValue) -> JsValue {
    let middleware = Array::new();
    middleware.push(&offset(offset_value));
    middleware.push(&flip());
    middleware.push(&shift());
    middleware.push(&viewport_size());
    let options = {
        let options = Object::new();
        Reflect::set(&options, &JsValue::from("element"), element).unwrap();
        options
    };
    middleware.push(&arrow(JsValue::from(options)));

    let options = {
        let options = Object::new();
        Reflect::set(
            &options,
            &JsValue::from("middleware"),
            &JsValue::from(middleware),
        )
        .unwrap();
        options
    };

    JsValue::from(options)
}
