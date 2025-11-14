pub const ALPN: &[u8] = b"p2term-proto";

pub const HELLO: &[u8; 8] = b"hello   ";
pub const WELCOME: &[u8; 8] = b"welcome ";

pub const OPT_MAX_LEN: usize = 4096;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct ClientOpt {
    shell: Option<String>,
    cwd: Option<String>,
}
