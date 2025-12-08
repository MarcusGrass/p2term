use crate::proto::ClientOpt;
use crate::server::config::ShellCfg;
use crate::streams::{ReadStream, WriteStream};
use std::fmt::Debug;

pub trait ServerShellProxy: Debug + Send + Sync + 'static {
    fn run<W, R>(
        write: W,
        read: R,
        shell_cfg: &ShellCfg,
        client_opt: ClientOpt,
    ) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        W: WriteStream,
        R: ReadStream;
}
