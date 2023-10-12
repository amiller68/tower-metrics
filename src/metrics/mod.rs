use http::{Request, Response};
use http_body::Body;
use tower::Layer;
use tower::Service;
use tower_http::classify::{MakeClassifier, ServerErrorsAsFailures, SharedClassifier};

mod body;
mod body_tracker;
mod future;

use body::ResponseBody;
use body_tracker::BodyTracker;
use future::ResponseFuture;

pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
    type Service = Metrics<S, SharedClassifier<ServerErrorsAsFailures>>;

    fn layer(&self, inner: S) -> Self::Service {
        Metrics::new(inner)
    }
}

#[derive(Debug, Clone)]
pub struct Metrics<S, M> {
    pub(crate) inner: S,
    pub(crate) make_classifier: M,
}

impl<S> Metrics<S, SharedClassifier<ServerErrorsAsFailures>> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            make_classifier: SharedClassifier::new(ServerErrorsAsFailures::default()),
        }
    }
}

impl<S, M, ReqBody, ResBody> Service<Request<ReqBody>> for Metrics<S, M>
where
    S: Service<Request<BodyTracker<ReqBody>>, Response = Response<ResBody>>,
    ReqBody: Body + Send + 'static,
    ResBody: Body + Send + 'static,
    ResBody::Error: std::fmt::Display + 'static,
    S::Error: std::fmt::Display + 'static,
    M: MakeClassifier,
    M::Classifier: Clone,
{
    type Response = Response<ResponseBody<BodyTracker<ResBody>, M::ClassifyEos>>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, M::Classifier>;
    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let start = std::time::Instant::now();
        let classifier = self.make_classifier.make_classifier(&req);
        let (parts, body) = req.into_parts();

        let header_size = parts
            .headers
            .iter()
            .fold(0, |acc, (k, v)| acc + k.as_str().len() + v.as_bytes().len());

        println!(
            "Observed request header_size: {} | start = {:?}",
            header_size, start
        );

        let tracker = BodyTracker::new(body);
        let handle = tracker.handle();
        let req = Request::from_parts(parts, tracker);
        let fut = { self.inner.call(req) };

        ResponseFuture {
            inner: fut,
            classifier: Some(classifier),
            request_body_tracker_handle: handle,
            start,
        }
    }
}
