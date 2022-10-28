use crate::{
    auth::{
        middleware::{random_cookie, CookieSession},
        SharedPdb, COOKIE_NAME,
    }, HtmlTemplate,
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
use serde::Deserialize;

#[derive(Debug)]
pub enum LoginError {
    WrongUsernameOrPassword,
    PasswordError,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    url: Option<String>,
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

/// The login template page method.
pub async fn serve_login(Query(url_query): Query<RedirectUrlQuery>) -> impl IntoResponse {
    let template = LoginTemplate { url: url_query.redirect };
    (StatusCode::OK, HtmlTemplate(template))
}

/// The method called by a login form POST request. Authentifies a user.
pub async fn check_username_password(
    form: Form<LogIn>,
    Extension(pdb): Extension<SharedPdb>,
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
            let cookie = random_cookie();
            pdb.sessions.push(CookieSession { cookie, user_id });
            tracing::debug!("adding cookie {:?}", pdb.sessions[pdb.sessions.len() - 1]);
            let mut header_value_str = format!(
                "{}={}; Secure; SameSite=Strict",
                COOKIE_NAME,
                std::str::from_utf8(&cookie).unwrap()
            );
            if log_in.remember == "true" {
                header_value_str.push_str("; Max-Age=172800");
            }
            headers.insert(
                header::SET_COOKIE,
                header::HeaderValue::from_str(&header_value_str).unwrap(),
            );
            let template = LoginSuccessTemplate {
                url: url_query.redirect.unwrap_or_else(|| "/".to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn should_serve_login_page() {
        let router = Router::new().route("/", get(serve_login));
        let response = router
            .oneshot(
                Request::builder()
                    .uri("/?redirect=/random")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
