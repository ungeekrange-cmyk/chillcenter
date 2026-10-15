//! Bound sockets before TLS; permits live until the TCP/TLS stream is dropped.
use axum_server::accept::Accept;
use std::{
    future::{Ready, ready},
    io,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::TcpStream,
    sync::{OwnedSemaphorePermit, Semaphore},
};

#[derive(Clone)]
pub struct LimitedAcceptor {
    permits: Arc<Semaphore>,
}
impl LimitedAcceptor {
    pub fn new(max: usize) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(max)),
        }
    }
    fn permit(&self) -> io::Result<OwnedSemaphorePermit> {
        self.permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| io::Error::from(io::ErrorKind::ConnectionRefused))
    }
}
pub struct LimitedStream {
    stream: TcpStream,
    _permit: OwnedSemaphorePermit,
}
impl<S> Accept<TcpStream, S> for LimitedAcceptor {
    type Stream = LimitedStream;
    type Service = S;
    type Future = Ready<io::Result<(LimitedStream, S)>>;
    fn accept(&self, stream: TcpStream, service: S) -> Self::Future {
        let result = (|| {
            if !crate::config::is_local_address(stream.peer_addr()?.ip()) {
                return Err(io::Error::from(io::ErrorKind::PermissionDenied));
            }
            stream.set_nodelay(true)?;
            Ok((
                LimitedStream {
                    stream,
                    _permit: self.permit()?,
                },
                service,
            ))
        })();
        ready(result)
    }
}
impl AsyncRead for LimitedStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_read(cx, buf)
    }
}
impl AsyncWrite for LimitedStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream).poll_write(cx, buf)
    }
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_flush(cx)
    }
    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(cx)
    }
}

/// Clear-text listener (ADR-0018): same connection cap, plus a deadline for the FIRST BYTE. hyper starts its header-read
/// timeout only after a byte has arrived, so without this, `max` silent sockets would starve the pool for good (review 6).
/// The TLS listener has the equivalent in its handshake timeout.
#[derive(Clone)]
pub struct FirstByteAcceptor {
    inner: LimitedAcceptor,
    timeout: std::time::Duration,
}
impl FirstByteAcceptor {
    pub fn new(max: usize, timeout: std::time::Duration) -> Self {
        Self {
            inner: LimitedAcceptor::new(max),
            timeout,
        }
    }
}
impl<S: Send + 'static> Accept<TcpStream, S> for FirstByteAcceptor {
    type Stream = LimitedStream;
    type Service = S;
    type Future = Pin<Box<dyn std::future::Future<Output = io::Result<(LimitedStream, S)>> + Send>>;
    fn accept(&self, stream: TcpStream, service: S) -> Self::Future {
        let accepted = Accept::<TcpStream, S>::accept(&self.inner, stream, service);
        let timeout = self.timeout;
        Box::pin(async move {
            let (stream, service) = accepted.await?;
            let mut probe = [0u8; 1];
            // `peek` does not consume the byte; on timeout, EOF or error the stream (and its permit) is dropped here.
            match tokio::time::timeout(timeout, stream.stream.peek(&mut probe)).await {
                Ok(Ok(n)) if n > 0 => Ok((stream, service)),
                _ => Err(io::Error::from(io::ErrorKind::TimedOut)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn silent_connections_are_dropped_and_return_their_permit() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let acceptor = FirstByteAcceptor::new(1, std::time::Duration::from_millis(200));
        let _silent = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server_side, _) = listener.accept().await.unwrap();
        let started = std::time::Instant::now();
        let refused = Accept::<TcpStream, ()>::accept(&acceptor, server_side, ()).await;
        assert!(refused.is_err(), "a silent connection must be refused");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        // The permit came back: the next connection, which does speak, is accepted with its byte still readable.
        let mut talker = tokio::net::TcpStream::connect(address).await.unwrap();
        let (server_side, _) = listener.accept().await.unwrap();
        talker.write_all(b"G").await.unwrap();
        let (mut stream, ()) = Accept::<TcpStream, ()>::accept(&acceptor, server_side, ())
            .await
            .unwrap();
        let mut byte = [0u8; 1];
        tokio::io::AsyncReadExt::read_exact(&mut stream, &mut byte)
            .await
            .unwrap();
        assert_eq!(&byte, b"G");
    }
    #[test]
    fn capacity_is_bounded_and_returns_on_disconnect() {
        let acceptor = LimitedAcceptor::new(2);
        let a = acceptor.permit().unwrap();
        let b = acceptor.permit().unwrap();
        assert!(acceptor.permit().is_err());
        drop(a);
        assert!(acceptor.permit().is_ok());
        drop(b);
    }
}
