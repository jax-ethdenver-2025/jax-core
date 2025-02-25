use axum::extract::Json;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Serialize)]
pub struct HelloResponse {
    message: String,
}

// TODO: you could add state as an extractor here if you want
pub async fn handler() -> Result<impl IntoResponse, HelloError> {
    Ok((
        http::StatusCode::OK,
        Json(HelloResponse {
            message: "Hello, world!".to_string(),
        }),
    )
        .into_response())
}

#[derive(Debug, thiserror::Error)]
pub enum HelloError {}

impl IntoResponse for HelloError {
    fn into_response(self) -> Response {
        (
            http::StatusCode::INTERNAL_SERVER_ERROR,
            "this never happens",
        )
            .into_response()
    }
}
