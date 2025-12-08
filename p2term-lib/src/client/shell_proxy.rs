use crate::streams::{ReadStream, WriteStream};
use std::fmt::Debug;

pub trait ClientShellProxy: Debug {
    fn run<W, R>(self, write: W, read: R) -> impl Future<Output = anyhow::Result<()>>
    where
        W: WriteStream,
        R: ReadStream;
}
