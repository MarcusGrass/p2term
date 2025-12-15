use crate::{TermSender, log};
use anyhow::{Context, bail};
use iroh::{PublicKey, SecretKey};
use p2term_lib::client::server_handle::P2TermServerHandle;
use p2term_lib::client::shell_proxy::ClientShellProxy;
use p2term_lib::convert::HexConvert;
use p2term_lib::error::unpack;
use p2term_lib::proto::{ClientOpt, DEFAULT_TERM};
use p2term_lib::streams::{ReadStream, WriteStream};
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::js_sys;
use wasm_bindgen_futures::js_sys::Uint8Array;

#[derive(Debug)]
pub(crate) struct Term(JsValue);

impl Term {
    pub fn new(js_value: JsValue) -> Self {
        Self(js_value)
    }

    fn writer(&self) -> anyhow::Result<js_sys::Function> {
        js_sys::Reflect::get(&self.0, &JsValue::from_str("write"))
            .map(js_sys::Function::from)
            .map_err(|e| anyhow::anyhow!("failed to get term write function: {e:?}"))
    }

    pub fn invoke_write(&self, write_fn: &js_sys::Function, data: &[u8]) -> anyhow::Result<()> {
        let u = Uint8Array::from(data);
        match write_fn.call1(&self.0, &u) {
            Ok(_) => Ok(()),
            Err(e) => {
                anyhow::bail!("failed to call term write: {e:?}");
            }
        }
    }
}

pub async fn start_connection(
    term: Term,
    secret_key: &str,
    peer_public_key: &str,
    shell: Option<&str>,
    cwd: Option<&str>,
    on_error: Option<js_sys::Function>,
) -> anyhow::Result<TermSender> {
    let secret_key =
        SecretKey::try_from_hex(secret_key.as_bytes()).context("failed to parse secret key")?;
    let pk = PublicKey::try_from_hex(peer_public_key.as_bytes()).context("invalid public key")?;
    let server_handle = P2TermServerHandle::connect(secret_key, pk)
        .await
        .context("failed to connect to server")?;
    let opt = ClientOpt {
        shell: shell.map(std::string::ToString::to_string),
        cwd: cwd.map(PathBuf::from),
        // I think this is legit for xterm.js, though not 100% sure
        term: Some(DEFAULT_TERM.to_string()),
    };
    let (send, recv) = tokio::sync::mpsc::channel(128);
    wasm_bindgen_futures::spawn_local(async move {
        let wsp = WebShellProxy {
            term,
            outbound_message_incoming: recv,
        };
        if let Err(e) = p2term_lib::client::runtime::run(server_handle, &opt, wsp).await {
            log(&format!("failed to run shell proxy: {}", unpack(&*e)));
            if let Some(func) = on_error {
                let _ = func.call1(
                    &JsValue::NULL,
                    &JsValue::from_str(&format!("failed to run shell proxy: {}", unpack(&*e))),
                );
            }
        } else if let Some(func) = on_error {
            let _ = func.call1(&JsValue::NULL, &JsValue::from_str("term exited"));
        }
    });
    Ok(TermSender(send))
}

#[derive(Debug)]
struct WebShellProxy {
    term: Term,
    outbound_message_incoming: tokio::sync::mpsc::Receiver<String>,
}

impl ClientShellProxy for WebShellProxy {
    async fn run<W, R>(self, mut write: W, mut read: R) -> anyhow::Result<()>
    where
        W: WriteStream,
        R: ReadStream,
    {
        let Self {
            term,
            mut outbound_message_incoming,
        } = self;
        let (reader_res_send, mut reader_res_recv) = tokio::sync::oneshot::channel();
        wasm_bindgen_futures::spawn_local(async move {
            let mut buf = [0u8; 1024];
            let write_fn = match term.writer() {
                Ok(f) => f,
                Err(e) => {
                    let _ = reader_res_send.send(e);
                    return;
                }
            };
            let err = loop {
                let read_bytes = match read.read(&mut buf).await {
                    Ok(rb) => rb,
                    Err(e) => {
                        break anyhow::anyhow!(
                            "failed to read from remote terminal: {}",
                            unpack(&e)
                        );
                    }
                };
                if read_bytes == 0 {
                    break anyhow::anyhow!("remote terminal EOF");
                }
                if let Err(e) = term.invoke_write(&write_fn, &buf[..read_bytes]) {
                    break anyhow::anyhow!(
                        "failed to write remote terminal message to web term: {}",
                        unpack(&*e)
                    );
                }
            };
            let _ = reader_res_send.send(err);
        });

        loop {
            tokio::select! {
                res = &mut reader_res_recv => {
                    match res {
                        Ok(e) => bail!("remote terminal error: {}", unpack(&*e)),
                        Err(_e) => bail!("remote terminal task failed"),
                    }
                }
                next = outbound_message_incoming.recv() => {
                    let Some(next) = next else {
                        return Ok(());
                    };
                    if let Err(e) = write.write_all(next.as_bytes()).await {
                        bail!("failed to write to remote terminal: {}", unpack(&e));
                    }
                }
            }
        }
    }
}
