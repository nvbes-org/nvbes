use std::{
    future::Future,
    io,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use axum::http::Request;
use axum_server::accept::Accept;
use base64::Engine;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{TlsAcceptor, server::TlsStream};
use tower::Service;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MtlsCertificateThumbprint(pub String);

#[derive(Clone)]
pub struct PeerCertificateAcceptor {
    config: axum_server::tls_rustls::RustlsConfig,
    handshake_timeout: Duration,
}

impl PeerCertificateAcceptor {
    pub fn new(config: axum_server::tls_rustls::RustlsConfig) -> Self {
        Self {
            config,
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

impl<I, S> Accept<I, S> for PeerCertificateAcceptor
where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    S: Send + 'static,
{
    type Stream = TlsStream<I>;
    type Service = PeerCertificateService<S>;
    type Future = Pin<Box<dyn Future<Output = io::Result<(Self::Stream, Self::Service)>> + Send>>;

    fn accept(&self, stream: I, service: S) -> Self::Future {
        let acceptor = TlsAcceptor::from(self.config.get_inner());
        let handshake_timeout = self.handshake_timeout;
        Box::pin(async move {
            let stream = tokio::time::timeout(handshake_timeout, acceptor.accept(stream))
                .await
                .map_err(|error| io::Error::new(io::ErrorKind::TimedOut, error))??;
            let certificate = stream
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|certificates| certificates.first())
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "mTLS connection has no verified client certificate",
                    )
                })?;
            let thumbprint = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(Sha256::digest(certificate.as_ref()));
            Ok((
                stream,
                PeerCertificateService {
                    inner: service,
                    thumbprint: MtlsCertificateThumbprint(thumbprint),
                },
            ))
        })
    }
}

#[derive(Clone)]
pub struct PeerCertificateService<S> {
    inner: S,
    thumbprint: MtlsCertificateThumbprint,
}

impl<S, B> Service<Request<B>> for PeerCertificateService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, context: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(context)
    }

    fn call(&mut self, mut request: Request<B>) -> Self::Future {
        request.extensions_mut().insert(self.thumbprint.clone());
        self.inner.call(request)
    }
}
