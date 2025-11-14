use crate::connection::{ReadStream, WriteStream};
use std::fmt::Debug;

pub trait ClientShellProxy: Debug + Send + Sync + 'static {
    fn run<W, R>(write: W, read: R) -> impl Future<Output = anyhow::Result<()>> + Send
    where
        W: WriteStream,
        R: ReadStream;
}
