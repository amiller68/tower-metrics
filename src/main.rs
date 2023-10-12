use axum::{Router, ServiceExt};
use axum::routing::{get, post};
use futures::StreamExt;
use std::net::SocketAddr;
use tower::ServiceBuilder;

mod tracker;

use tracker::BytesRWTrackerLayer;

#[tokio::main]
async fn main() {
    // Define a simple GET route.
    let app = Router::new()
        .route("/", get(handler))
        .route("/stream", post(stream_handler))
        .route("/json", post(json_handler));

    let middleware_stack = ServiceBuilder::new()
        .layer(BytesRWTrackerLayer);

    let app = middleware_stack.service(app);
    
    // Start the server on the given address.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr).serve(app.into_make_service())
        .await.unwrap();
}

async fn handler() -> &'static str {
    "Hello, Axum!"
}

use axum::extract::{BodyStream, Json};

async fn stream_handler(mut body: BodyStream) -> &'static str {
    while let Some(chunk) = body.next().await {
        println!("chunk: {:?}", chunk);
    }
    "Hello, Axum!"
}

async fn json_handler(Json(body): Json<serde_json::Value>) -> &'static str {
    println!("body: {:?}", body);
    "Hello, Axum!"
}