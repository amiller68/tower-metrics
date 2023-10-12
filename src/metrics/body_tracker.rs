use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use bytes::Buf;
use http_body::Body;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    pub struct BodyTracker<B> {
        read: Arc<AtomicUsize>,
        #[pin]
        body: B,
    }
}

impl<B> BodyTracker<B> {
    /// Create a new [`BodyTracker`]
    pub fn new(body: B) -> Self {
        Self {
            read: Arc::new(AtomicUsize::new(0)),
            body,
        }
    }

    /// Get a [`BodyTrackerHandle`] that can be used to get the number of bytes polled
    pub fn handle(&self) -> BodyTrackerHandle {
        BodyTrackerHandle {
            read: self.read.clone(),
        }
    }
}

impl<ReqBody> Body for BodyTracker<ReqBody>
where
    ReqBody: Body,
    ReqBody::Error: std::fmt::Display + 'static,
{
    type Data = ReqBody::Data;
    type Error = ReqBody::Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.as_mut().project();
        let res = this.body.poll_data(cx);
        if let Poll::Ready(Some(Ok(data))) = &res {
            this.read.fetch_add(data.remaining(), Ordering::Relaxed);
        }
        res
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
        self.project().body.poll_trailers(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.body.is_end_stream()
    }
}

/// A handle to a [`BodyTracker`] that can be used to get the number of bytes
/// read and/or written even though the [`BodyTracker`] is consumed by a protocol
/// consumer.
#[derive(Debug, Clone)]
pub struct BodyTrackerHandle {
    read: Arc<AtomicUsize>,
}

impl BodyTrackerHandle {
    /// Get the number of bytes read (so far).
    pub fn read(&self) -> usize {
        self.read.load(Ordering::Relaxed)
    }
}
