use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::header::HeaderName;
use axum::http::{Request, StatusCode};
use axum::response::Response;
use tower::Service;

static X_CLIENT_IP: HeaderName = HeaderName::from_static("x-nvbes-client-ip");
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

type Counter = Arc<Mutex<HashMap<IpAddr, u32>>>;

struct ConnectionGuard {
    counter: Counter,
    ip: IpAddr,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let mut map = self.counter.lock().unwrap();
        let remaining = if let Some(count) = map.get_mut(&self.ip) {
            *count = count.saturating_sub(1);
            let remaining = *count;
            if remaining == 0 {
                map.remove(&self.ip);
            }
            Some(remaining)
        } else {
            None
        };
        match remaining {
            Some(n) => {
                tracing::trace!(ip = %self.ip, remaining = n, "PerIpConcurrency: connection released")
            }
            None => {
                tracing::warn!(ip = %self.ip, "PerIpConcurrency: connection released but IP not found")
            }
        }
    }
}

fn extract_client_ip<B>(req: &Request<B>) -> Option<IpAddr> {
    req.headers()
        .get(&X_CLIENT_IP)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<IpAddr>().ok())
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip())
        })
}

fn try_acquire(counter: &Counter, ip: IpAddr, max: u32) -> Option<ConnectionGuard> {
    let mut map = counter.lock().unwrap();
    let count = map.entry(ip).or_insert(0);
    if *count >= max {
        tracing::warn!(
            ip = %ip,
            count = *count,
            max = max,
            "PerIpConcurrency: max connections reached, returning 429"
        );
        return None;
    }
    *count += 1;
    tracing::trace!(
        ip = %ip,
        count = *count,
        "PerIpConcurrency: connection acquired"
    );
    Some(ConnectionGuard {
        counter: Arc::clone(counter),
        ip,
    })
}

#[derive(Clone)]
pub struct PerIpConcurrencyLayer {
    max: u32,
    counter: Counter,
}

impl PerIpConcurrencyLayer {
    pub fn new(max_per_ip: u32) -> Self {
        Self {
            max: max_per_ip,
            counter: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<S> tower::Layer<S> for PerIpConcurrencyLayer {
    type Service = PerIpConcurrency<S>;

    fn layer(&self, service: S) -> Self::Service {
        PerIpConcurrency {
            inner: service,
            max: self.max,
            counter: self.counter.clone(),
        }
    }
}

#[derive(Clone)]
pub struct PerIpConcurrency<S> {
    inner: S,
    max: u32,
    counter: Counter,
}

impl<S> Service<Request<Body>> for PerIpConcurrency<S>
where
    S: Service<Request<Body>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let req_id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = req.uri().path().to_string();
        let method = req.method().to_string();
        let ip = extract_client_ip(&req);
        tracing::debug!(
            req_id,
            method = %method,
            path = %path,
            ip = ?ip,
            "PerIpConcurrency: processing request"
        );
        let guard = ip.and_then(|ip| try_acquire(&self.counter, ip, self.max));

        if guard.is_none() && ip.is_some() {
            tracing::warn!(
                req_id,
                method = %method,
                path = %path,
                ip = ?ip,
                "PerIpConcurrency: rate limited"
            );
            return Box::pin(async {
                Ok(Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Retry-After", "5")
                    .body(Body::from("Too many concurrent connections"))
                    .unwrap())
            });
        }

        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            let result = inner.call(req).await;
            drop(guard);
            tracing::debug!(
                req_id,
                method = %method,
                path = %path,
                "PerIpConcurrency: request completed"
            );
            result
        })
    }
}
