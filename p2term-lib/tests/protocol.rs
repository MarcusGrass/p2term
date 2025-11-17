use anyhow::Context as _;
use iroh_base::{PublicKey, SecretKey};
use p2term_lib::client::shell_proxy::ClientShellProxy;
use p2term_lib::client_handle::P2TermClientHandle;
use p2term_lib::connection::{P2TermServerConnection, ReadStream, WriteStream};
use p2term_lib::crypto::generate_secret_key;
use p2term_lib::proto::ClientOpt;
use p2term_lib::server::config::{P2TermdCfg, ShellCfg};
use p2term_lib::server::connection_handler::P2TermConnectionHandler;
use p2term_lib::server::router::P2TermRouter;
use p2term_lib::server::shell_proxy::ServerShellProxy;
use p2term_lib::server_handle::P2TermServerHandle;
use std::io::Error;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

#[derive(Debug)]
struct NoopShell;

impl ServerShellProxy for NoopShell {
    async fn run<W, R>(
        _write: W,
        _read: R,
        _shell_cfg: &ShellCfg,
        _client_opt: ClientOpt,
    ) -> anyhow::Result<()>
    where
        W: WriteStream,
        R: ReadStream,
    {
        Ok(())
    }
}

impl ClientShellProxy for NoopShell {
    async fn run<W, R>(_write: W, _read: R) -> anyhow::Result<()>
    where
        W: WriteStream,
        R: ReadStream,
    {
        Ok(())
    }
}

struct DummyRouter {
    incoming_connections: Option<tokio::sync::mpsc::UnboundedReceiver<DummyConnection>>,
}

struct DummyConnection {
    secret_key: SecretKey,
    channels: Mutex<Option<DummyConnectionChannels>>,
}

struct DummyConnectionChannels {
    server_send: MpscByteSenderStream,
    server_recv: MpscByteReceiverStream,
}

#[derive(Debug)]
struct MpscByteReceiverStream {
    inner: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
}

impl AsyncRead for MpscByteReceiverStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.inner.poll_recv(cx) {
            Poll::Ready(Some(v)) => {
                buf.put_slice(&v);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Ok(())),
            Poll::Pending => Poll::Pending,
        }
    }
}
impl ReadStream for MpscByteReceiverStream {}

#[derive(Debug)]
struct MpscByteSenderStream {
    inner: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
}

impl AsyncWrite for MpscByteSenderStream {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        match self.inner.send(buf.to_vec()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(_) => Poll::Ready(Ok(0)),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl WriteStream for MpscByteSenderStream {}

fn mpsc_pair() -> (MpscByteSenderStream, MpscByteReceiverStream) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    (
        MpscByteSenderStream { inner: tx },
        MpscByteReceiverStream { inner: rx },
    )
}

impl P2TermServerConnection<MpscByteSenderStream, MpscByteReceiverStream> for DummyConnection {
    fn peer(&self) -> PublicKey {
        self.secret_key.public()
    }

    async fn accept(
        &self,
        peer: PublicKey,
    ) -> anyhow::Result<P2TermClientHandle<MpscByteSenderStream, MpscByteReceiverStream>> {
        let channels = self
            .channels
            .lock()
            .unwrap()
            .take()
            .context("no channels available")?;
        Ok(P2TermClientHandle::new(
            peer,
            channels.server_send,
            channels.server_recv,
        ))
    }
}

impl P2TermRouter for DummyRouter {
    async fn start<S>(
        &mut self,
        _secret_key: SecretKey,
        handler: P2TermConnectionHandler<S>,
    ) -> anyhow::Result<()>
    where
        S: ServerShellProxy,
    {
        let mut con_recv = self
            .incoming_connections
            .take()
            .context("empty incoming connections channel")?;
        tokio::task::spawn(async move {
            while let Some(peer) = con_recv.recv().await {
                p2term_lib::server::connection_handler::ConnectionHandler::serve::<
                    MpscByteSenderStream,
                    MpscByteReceiverStream,
                >(&handler, peer)
                .await
                .unwrap();
            }
        });
        Ok(())
    }

    async fn shutdown(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_protocol() {
    let (client_send, server_recv) = mpsc_pair();
    let (server_send, client_recv) = mpsc_pair();
    let handle = P2TermServerHandle::new(client_send, client_recv);
    let cfg = P2TermdCfg::default();
    let (incoming_send, incoming_recv) = tokio::sync::mpsc::unbounded_channel();
    let router = DummyRouter {
        incoming_connections: Some(incoming_recv),
    };
    incoming_send
        .send(DummyConnection {
            secret_key: generate_secret_key(),
            channels: Mutex::new(Some(DummyConnectionChannels {
                server_send,
                server_recv,
            })),
        })
        .unwrap();
    let (finished_sig_send, finished_sig_recv) = tokio::sync::mpsc::channel(2);
    let opt = ClientOpt::default();
    let client_task = tokio::task::spawn(async move {
        p2term_lib::client::runtime::run::<_, _, NoopShell>(handle, &opt).await
    });
    let server_task = tokio::task::spawn(p2term_lib::server::runtime::run::<_, NoopShell>(
        cfg,
        router,
        finished_sig_recv,
    ));
    finished_sig_send.try_send(()).unwrap();
    client_task.await.unwrap().unwrap();
    server_task.await.unwrap().unwrap();
}
