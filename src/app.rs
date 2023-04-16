use crate::api::{add_points, available, query_points};
use crate::auth::{
    auth_middleware, check_username_password, insert_username_password, serve_login,
};
use crate::auth::{new_shared_db, SharedPdb};
use crate::settings::Settings;
use crate::{handle_static_error, HtmlTemplate};
use askama::Template;
use axum::{
    handler::Handler,
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, get_service, post},
    Extension, Router,
};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

struct EnvError {}

/// Runs the server. Main entrypoint for the server app.
pub async fn run_server() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&settings.database.url)
        .await
        .expect("Cannot connect to postgres database.");
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
                .or_else(|_| settings.base.rust_log.ok_or(EnvError {}))
                .unwrap_or_else(|_| "debug,tower_http=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    sqlx::migrate!("database/migrations").run(&pool).await?;

    let shared_pdb = new_shared_db(&pool);
    if settings.auth.develop {
        tracing::warn!("Development mode should not be used in production!");
        let pdb_clone = shared_pdb.clone();
        let mut pdb_lock = pdb_clone.lock().await;
        pdb_lock.set_develop();
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 18032));
    tracing::debug!("listening on {}", addr);

    axum::Server::bind(&addr)
        .serve(app(pool, shared_pdb).into_make_service())
        .await
        .unwrap();
    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct WelcomTemplate {}

#[derive(Template)]
#[template(path = "map.html")]
struct MapTemplate {}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {}

fn app(pool: PgPool, shared_pdb: SharedPdb) -> Router {
    let api_routes = Router::new()
        .route("/query", get(query_points))
        .route("/input", post(add_points))
        .route("/available", get(available))
        .layer(Extension(pool));
    let login_routes = Router::new()
        .route("/", get(serve_login).post(check_username_password))
        .layer(Extension(shared_pdb.clone()));
    let register_routes =
        Router::new().nest("/", get(|| async { HtmlTemplate(RegisterTemplate {}) }));
    let add_user_routes = Router::new()
        .route("/", post(insert_username_password))
        .layer(Extension(shared_pdb.clone()));

    let map_routes = Router::new().route("/", get(|| async { HtmlTemplate(MapTemplate {}) }));

    Router::new()
        .nest("/map", map_routes)
        .nest("/api", api_routes)
        .route_layer(middleware::from_fn(move |req, next| {
            auth_middleware(req, next, shared_pdb.clone())
        }))
        .route("/", get(|| async { HtmlTemplate(WelcomTemplate {}) }))
        .nest("/login", login_routes)
        .nest("/register", register_routes)
        .nest("/add_user", add_user_routes)
        .nest(
            "/public",
            get_service(ServeDir::new("./static")).handle_error(handle_static_error),
        )
        .fallback(fallback.into_service())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

async fn fallback() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}
