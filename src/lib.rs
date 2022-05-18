pub mod api;
pub mod auth;
pub mod settings;

use axum::http::{HeaderMap, StatusCode};
use std::fmt::Display;

pub async fn handle_static_error<T: Display>(error: T) -> (StatusCode, HeaderMap, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        HeaderMap::new(),
        format!("Unhandled internal error: {}", error),
    )
}
