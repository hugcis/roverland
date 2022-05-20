use crate::{
    auth::{password_db::PasswordDatabase, CurrentUser, COOKIE_AUTH_LEN, COOKIE_NAME},
    HtmlTemplate,
};
use askama::Template;
use axum::{
    http::{header, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, Debug)]
pub struct CookieSession {
    pub cookie: [u8; COOKIE_AUTH_LEN],
    pub user_id: i32,
}

#[derive(Template)]
#[template(path = "unauthorized.html")]
struct UnauthorizedTemplate {
    url: String,
}

fn get_cookie_map<B>(req: &Request<B>) -> Option<HashMap<String, String>> {
    req.headers()
        .get(header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .map(|cookie_str| {
            cookie_str
                .split("; ")
                .map(|cookie_pair| {
                    let split: Vec<&str> = cookie_pair.split("=").collect();
                    (split[0].to_string(), split[1].to_string())
                })
                .collect()
        })
}

pub async fn auth<B>(
    mut req: Request<B>,
    next: Next<B>,
    pdb: Arc<Mutex<PasswordDatabase>>,
) -> impl IntoResponse {
    let cookies: Option<HashMap<String, String>> = get_cookie_map(&req);
    // Check for cookies first and then for the valid token.
    let mut user_id_auth = match cookies {
        Some(hashmap) => {
            if let Some(val) = hashmap.get(COOKIE_NAME) {
                tracing::debug!("found a cookie");
                pdb.lock()
                    .await
                    .sessions
                    .iter()
                    .find(|ck| {
                        ck.cookie
                            .into_iter()
                            .zip(val.as_bytes())
                            .all(|(a, &b)| a == b)
                    })
                    .map(|cookie: &CookieSession| cookie.user_id)
            } else {
                None
            }
        }
        None => None,
    };
    if user_id_auth.is_none() {
        user_id_auth = get_token_from_uri(req.uri().query().unwrap_or(""), pdb.clone()).await;
    }

    if let Some(user_id) = user_id_auth {
        let current_user = CurrentUser {
            user_id,
            is_admin: false,
        };
        req.extensions_mut().insert(current_user);
        Ok(next.run(req).await)
    } else {
        tracing::debug!("unauthorized request");
        let template = UnauthorizedTemplate {
            url: req.uri().to_string(),
        };
        Err((StatusCode::UNAUTHORIZED, HtmlTemplate(template)))
    }
}

async fn get_token_from_uri(query: &str, pdb: Arc<Mutex<PasswordDatabase>>) -> Option<i32> {
    // Parse the query parameters for the token and check it.
    let mut user_id = None;
    for x in query.split("&") {
        let split: Vec<String> = x.split("=").map(|x| x.to_string()).collect();
        if (split.len() == 2) && (split[0] == "token") {
            user_id = token_is_valid(&split[1], pdb.clone()).await;
            if user_id.is_some() {
                tracing::debug!("valid token found");
                break;
            }
        }
    }
    user_id
}

async fn token_is_valid(token: &str, pdb: Arc<Mutex<PasswordDatabase>>) -> Option<i32> {
    let pdb = pdb.lock().await;
    sqlx::query!(
        r#"SELECT user_id, is_admin FROM input_tokens JOIN users ON
           input_tokens.user_id=users.id WHERE input_token=$1"#,
        token
    )
    .fetch_one(&pdb.storage.pool)
    .await
    .unwrap()
    .user_id
}
