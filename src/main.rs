mod api;
mod settings;

use api::{add_points, query_points};
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
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&settings.database.url)
        .await?;

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let api_routes = Router::new()
        .route("/input", post(add_points))
        .route("/query", get(query_points))
        .layer(Extension(pool));
    let app = Router::new()
        .nest(
            "/map",
            get_service(ServeDir::new("./content")).handle_error(
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
            auth(req, next, settings.auth.clone())
        }))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 18032));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

pub async fn auth<B>(req: Request<B>, next: Next<B>, auth: settings::Auth) -> impl IntoResponse {
    let auth_header = req.uri().query().unwrap_or("").split("&").any(|x| {
        let split: Vec<String> = x.split("=").map(|x| x.to_string()).collect();
        if (split.len() == 2) && (split[0] == "token") && token_is_valid(&split[1], auth.clone()) {
            true
        } else {
            false
        }
    });

    if auth_header {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn token_is_valid(token: &str, auth: settings::Auth) -> bool {
    token == auth.token
}
