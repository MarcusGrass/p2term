mod connection;

use crate::connection::{Term, start_connection};
use p2term_lib::convert::HexConvert;
use p2term_lib::error::unpack;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen_futures::js_sys;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

}

#[wasm_bindgen]
pub struct TermSender(tokio::sync::mpsc::Sender<String>);

#[wasm_bindgen]
impl TermSender {
    pub async fn on_data(&self, data: &str) -> Result<(), JsValue> {
        self.0
            .send(data.to_string())
            .await
            .map_err(|e| JsValue::from_str(&format!("failed to send data: {}", unpack(&e))))
    }
}

#[wasm_bindgen]
#[must_use]
pub fn generate_private_key() -> String {
    p2term_lib::crypto::generate_secret_key().to_hex()
}

#[wasm_bindgen]
pub async fn connect(
    term: JsValue,
    secret_key: &str,
    public_key: &str,
    shell: Option<String>,
    cwd: Option<String>,
    on_error: Option<js_sys::Function>,
) -> Result<TermSender, JsValue> {
    start_connection(
        Term::new(term),
        secret_key,
        public_key,
        shell.as_deref(),
        cwd.as_deref(),
        on_error,
    )
    .await
    .map_err(|e| JsValue::from_str(&format!("failed to connect: {}", unpack(&*e))))
}
