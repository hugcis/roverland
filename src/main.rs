mod api;
mod auth;
mod settings;

use api::{add_points, query_points};
use auth::{auth, check_username_password, insert_username_password};
use axum::{
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::IntoResponse,
    routing::{get, get_service, post},
    Extension, Router,
};
use settings::Settings;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::{convert::Infallible, sync::Arc};
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use auth::{PasswordDatabase, PasswordStorage};
#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings.database.url)
        .await?;

    let password_db = PasswordDatabase {
        db_salt_component: settings.database.url[..16]
            .as_bytes()
            .try_into()
            .expect("Slice with incorrect length"),
        storage: PasswordStorage { pool: pool.clone() },
        sessions: vec![],
    };
    let shared_pdb = Arc::new(password_db);

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let api_routes = Router::new()
        .route("/input", post(add_points))
        .route("/query", get(query_points))
        .layer(Extension(pool));
    let login_routes = Router::new().nest(
        "/",
        get_service(ServeDir::new("./static/login")).handle_error(
            |error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            },
        ),
    );
    let register_routes = Router::new().nest(
        "/",
        get_service(ServeDir::new("./static/register")).handle_error(
            |error: std::io::Error| async move {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Unhandled internal error: {}", error),
                )
            },
        ),
    );
    let check_routes = Router::new()
        .route("/", post(check_username_password))
        .layer(Extension(shared_pdb));

    let app = Router::new()
        .nest(
            "/map",
            get_service(ServeDir::new("./static/map")).handle_error(
                |error: std::io::Error| async move {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Unhandled internal error: {}", error),
                    )
                },
            ),
        )
        .nest("/api", api_routes)
        .route_layer(middleware::from_fn(move |req, next| {
            auth(req, next, Arc::clone(&assword_db))
        }))
        .nest("/login", login_routes)
        .nest("/register", register_routes)
        .nest("/check", check_routes)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 18032));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
