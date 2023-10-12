use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};

use http_body::Body;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    pub struct BodyTracker<S> {
        read: Arc<AtomicUsize>,
        #[pin]
        stream: S,
    }
}

impl<S> BodyTracker<S> {
    /// Create a new [`BodyTracker`] 
    pub fn new(stream: S) -> Self {
        Self {
            read: Arc::new(AtomicUsize::new(0)),
            stream,
        }
    }

    /// Get a [`BodyTrackerHandle`] that can be used to get the number of bytes polled
    pub fn handle(&self) -> BodyTrackerHandle {
        BodyTrackerHandle {
            read: self.read.clone(),
        }
    }
}

impl Body for BodyTracker<hyper::body::Body> {
    type Data = hyper::body::Bytes;
    type Error = hyper::Error;

    fn poll_data(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.as_mut().project();
        let res = this.stream.poll_data(cx);
        if let Poll::Ready(Some(Ok(data))) = &res {
            this.read.fetch_add(data.len(), Ordering::Relaxed);
        }
        res
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<hyper::HeaderMap>, Self::Error>> {
        self.project().stream.poll_trailers(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.stream.is_end_stream()
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