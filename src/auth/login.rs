use crate::{
    auth::{
        middleware::CookieSession, password_db::PasswordDatabase, COOKIE_AUTH_LEN, COOKIE_NAME,
    },
    HtmlTemplate,
};
use askama::Template;
use axum::{
    extract::{Form, Query},
    http::{
        header::{self, HeaderMap},
        StatusCode,
    },
    response::IntoResponse,
    Extension,
};
use rand::Rng;
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum LoginError {
    WrongUsernameOrPassword,
    PasswordError,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    url: String,
}

#[derive(Template)]
#[template(path = "login_success.html")]
struct LoginSuccessTemplate {
    url: String,
}

#[derive(Deserialize, Debug)]
pub struct RedirectUrlQuery {
    redirect: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LogIn {
    username: String,
    password: String,
    remember: String,
}

pub async fn serve_login(Query(url_query): Query<RedirectUrlQuery>) -> impl IntoResponse {
    let redirect_val = url_query.redirect.unwrap_or("none".to_string());
    tracing::debug!(
        "unauthorized request, will redirect to {} after login",
        redirect_val
    );
    let template = LoginTemplate { url: redirect_val };
    (StatusCode::OK, HtmlTemplate(template))
}

pub async fn check_username_password(
    form: Form<LogIn>,
    Extension(pdb): Extension<Arc<Mutex<PasswordDatabase>>>,
    Query(url_query): Query<RedirectUrlQuery>,
) -> impl IntoResponse {
    let log_in: LogIn = form.0;
    let mut headers = HeaderMap::new();
    let mut pdb = pdb.lock().await;
    match pdb
        .verify_password(&log_in.username, &log_in.password)
        .await
    {
        Ok(user_id) => {
            let mut rng = rand::thread_rng();
            let cookie: String = (&mut rng)
                .sample_iter(rand::distributions::Alphanumeric)
                .take(COOKIE_AUTH_LEN)
                .map(char::from)
                .collect();
            pdb.sessions.push(CookieSession {
                cookie: cookie.as_bytes().try_into().unwrap(),
                user_id,
            });
            tracing::debug!("adding cookie {:?}", pdb.sessions[pdb.sessions.len() - 1]);
            let mut header_value_str =
                format!("{}={}; Secure; SameSite=Strict", COOKIE_NAME, cookie);
            if log_in.remember == "true" {
                header_value_str.push_str("; Max-Age=172800");
            }
            headers.insert(
                header::SET_COOKIE,
                header::HeaderValue::from_str(&header_value_str).unwrap(),
            );
            let template = LoginSuccessTemplate {
                url: url_query.redirect.unwrap_or("/".to_string()),
            };
            Ok((StatusCode::OK, headers, HtmlTemplate(template)))
        }
        Err(e) => match e {
            LoginError::PasswordError => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                "Error fetching info".to_string(),
            )),
            LoginError::WrongUsernameOrPassword => Err((
                StatusCode::UNAUTHORIZED,
                headers,
                "Wrong username/password".to_string(),
            )),
        },
    }
}
