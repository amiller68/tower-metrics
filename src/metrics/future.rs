use http::Response;
use tower_http::classify::{ClassifiedResponse, ClassifyResponse};
use http_body::Body;
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
};

// use super::body_tracker::{BodyTracker, BodyTrackerHandle};
use super::body_tracker::BodyTrackerHandle;
use super::body::ResponseBody;

pin_project! {
    pub struct ResponseFuture<F, C> {
        #[pin]
        pub(crate) inner: F,
        pub(crate) classifier: Option<C>,
        pub(crate) request_body_tracker_handle: BodyTrackerHandle,
        pub(crate) start: Instant,
    }
}

impl<Fut, ResBody, E, C> Future
    for ResponseFuture<Fut, C>
where
    Fut: Future<Output = Result<Response<ResBody>, E>>,
    ResBody: Body,
    ResBody::Error: std::fmt::Display + 'static,
    E: std::fmt::Display + 'static,
    C: ClassifyResponse,
{
    type Output = Result<
        // Response<ResponseBody<BodyTracker<ResBody>, C::ClassifyEos>>,
        Response<ResponseBody<ResBody, C::ClassifyEos>>,
        E,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = futures_util::ready!(this.inner.poll(cx));

        let read = this.request_body_tracker_handle.read();
        println!("Observed {} bytes of request body after {:?}", read, this.start);

        let classifier = this.classifier.take().unwrap();
        match result {
            // At this point, the request body has been fully read.
            Ok(res) => {
                let classification = classifier.classify_response(&res);
                let start = *this.start;

                let header_size = res.headers().iter().fold(0, |acc, (k, v)| {
                    acc + k.as_str().len() + v.as_bytes().len()
                });

                println!("Observed response header_size: {} | start = {:?}", header_size, start);

                match classification {
                    ClassifiedResponse::Ready(classification) => {
                        if let Err(_failure_class) = classification {
                            // TODO: What do you want to do with the failure class?
                        }

                        let res = res.map(|body| ResponseBody {
                            // inner: BodyTracker::new(body),
                            inner: body,
                            classify_eos: None,
                            start,
                        });

                        Poll::Ready(Ok(res))
                    }
                    ClassifiedResponse::RequiresEos(classify_eos) => {
                        let res = res.map(|body| ResponseBody {
                            // inner: BodyTracker::new(body),
                            inner: body,
                            classify_eos: Some(classify_eos),
                            start,
                        });
                        Poll::Ready(Ok(res))
                    }
                }
            }
            Err(err) => {
                let _failure_class = classifier.classify_error(&err);
                Poll::Ready(Err(err))
            }
        }
    }
}