use http::{Request, Response};
use http_body::Body;
use tower::Service;
use tower::Layer;

mod bytes_rw_tracker;

use bytes_rw_tracker::BytesRWTracker;

pub struct BytesRWTrackerLayer;

impl<S> Layer<S> for BytesRWTrackerLayer {
    type Service = BytesRWTrackerService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        BytesRWTrackerService::new(inner)
    }
}

#[derive(Debug, Clone)]
pub struct BytesRWTrackerService<S> {
    inner: S,
}

impl<S> BytesRWTrackerService<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for BytesRWTrackerService<S>
where
    S: Service<Request<BytesRWTracker<ReqBody>>, Response = Response<ResBody>>,
    ReqBody: Body + Send + 'static,
    ResBody: Body + Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let (parts, body) = req.into_parts();
        let tracker = BytesRWTracker::new(body);
        let handle = tracker.handle();
        let mut req = Request::from_parts(parts, tracker);
        req.extensions_mut().insert(handle);
        self.inner.call(req)
    }
}
