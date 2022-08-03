use crate::api::{add_points, query_points};
use crate::auth::{
    auth_middleware, check_username_password, insert_username_password, serve_login,
};
use crate::auth::{new_shared_db, SharedPdb};
use crate::settings::Settings;
use crate::{handle_static_error, HtmlTemplate};
use askama::Template;
use axum::{
    handler::Handler,
    http::{StatusCode, Uri},
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

fn app(pool: PgPool, shared_pdb: SharedPdb) -> Router {
    let api_routes = Router::new()
        .route("/query", get(query_points))
        .route("/input", post(add_points))
        .layer(Extension(pool));
    let login_routes = Router::new()
        .route("/", get(serve_login).post(check_username_password))
        .layer(Extension(shared_pdb.clone()));
    let register_routes = Router::new().nest(
        "/",
        get_service(ServeDir::new("./static/register")).handle_error(handle_static_error),
    );
    let add_user_routes = Router::new()
        .route("/", post(insert_username_password))
        .layer(Extension(shared_pdb.clone()));

    Router::new()
        .nest(
            "/map",
            get_service(ServeDir::new("./static/map")).handle_error(handle_static_error),
        )
        .nest("/api", api_routes)
        .route_layer(middleware::from_fn(move |req, next| {
            auth_middleware(req, next, shared_pdb.clone())
        }))
        .route("/", get(|| async { HtmlTemplate(WelcomTemplate {}) }))
        .nest("/login", login_routes)
        .nest("/register", register_routes)
        .nest("/add_user", add_user_routes)
        .fallback(fallback.into_service())
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

async fn fallback(uri: Uri) -> impl IntoResponse {
    (StatusCode::NOT_FOUND, format!("No page found for {}", uri))
}
