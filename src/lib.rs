pub mod api;
mod app;
pub mod auth;
mod create_admin;
mod register_token;
pub mod settings;

pub use app::run_server;
pub use create_admin::create_admin;
pub use register_token::add_register_token;

use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use std::fmt::Display;

pub async fn handle_static_error<T: Display>(error: T) -> (StatusCode, HeaderMap, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        HeaderMap::new(),
        format!("Unhandled internal error: {}", error),
    )
}

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {}", err),
            )
                .into_response(),
        }
    }
}
