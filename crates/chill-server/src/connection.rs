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
#[cfg(test)]
mod tests {
    use super::*;
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
