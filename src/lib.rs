#![deny(missing_docs)]

//! This crate contains the server and API for the roverland app.

/// The API module contains all the API method implementations for the REST
/// server.
pub mod api;
/// Module containing all the authentication, registration, cookies, etc. logic.
pub mod auth;
/// This module is used to parse and read from configuration files for the
/// server.
pub mod settings;
mod app;
mod create_admin;
mod register_token;

pub use app::run_server;
pub use create_admin::create_admin;
pub use register_token::add_register_token;

use askama::Template;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Html;
use axum::response::IntoResponse;
use axum::response::Response;
use std::fmt::Display;

/// This function is a fallback to handle errors and return responses that is
/// used in several place in the code.
pub async fn handle_static_error<T: Display>(error: T) -> impl IntoResponse {
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
