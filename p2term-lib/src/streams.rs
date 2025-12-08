use core::fmt::Debug;
use iroh::endpoint::{RecvStream, SendStream};
use tokio::io::{AsyncRead, AsyncWrite};

pub trait WriteStream: AsyncWrite + Debug + Unpin + Send + Sync + 'static {}
impl WriteStream for SendStream {}
pub trait ReadStream: AsyncRead + Debug + Unpin + Send + Sync + 'static {}
impl ReadStream for RecvStream {}
