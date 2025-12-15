use std::path::PathBuf;

pub const ALPN: &[u8] = b"p2term-proto";

pub const HELLO: &[u8; 8] = b"hello   ";
pub const WELCOME: &[u8; 8] = b"welcome ";

pub const DEFAULT_TERM: &str = "xterm-256color";

pub const OPT_MAX_LEN: usize = 4096;

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct ClientOpt {
    pub shell: Option<String>,
    pub cwd: Option<PathBuf>,
    pub term: Option<String>,
}
