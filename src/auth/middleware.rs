use crate::{
    auth::{CurrentUser, SharedPdb, COOKIE_AUTH_LEN, COOKIE_NAME},
    HtmlTemplate,
};
use askama::Template;
use axum::{
    http::{header, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use rand::Rng;
use std::collections::HashMap;

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

/// Create a random cookie token with alphanums characters.
pub fn random_cookie() -> [u8; COOKIE_AUTH_LEN] {
    let mut rng = rand::thread_rng();
    let cookie: String = (&mut rng)
        .sample_iter(rand::distributions::Alphanumeric)
        .take(COOKIE_AUTH_LEN)
        .map(char::from)
        .collect();
    cookie.as_bytes().try_into().unwrap()
}

/// Create a HashMap with the content of the cookie header.
fn get_cookie_map<B>(req: &Request<B>) -> HashMap<String, String> {
    req.headers()
        .get(header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .map(|cookie_str| {
            cookie_str
                .split("; ")
                .map(|cookie_pair| {
                    let split: Vec<&str> = cookie_pair.split('=').collect();
                    (split[0].to_string(), split[1].to_string())
                })
                .collect()
        })
        .unwrap_or_default()
}

pub async fn auth<B>(mut req: Request<B>, next: Next<B>, pdb: SharedPdb) -> impl IntoResponse {
    {
        let lock = pdb.lock().await;
        if lock.develop_mode {
            let current_user = CurrentUser {
                user_id: 1,
                is_admin: true,
            };
            req.extensions_mut().insert(current_user);
            return Ok(next.run(req).await);
        }
    }

    let cookies: HashMap<String, String> = get_cookie_map(&req);

    // Check for cookies first and then for the valid token.
    let mut user_id_auth = if let Some(val) = cookies.get(COOKIE_NAME) {
        tracing::debug!("found a cookie");
        pdb.lock()
            .await
            .sessions
            .iter()
            .enumerate()
            .find(|(_, ck)| {
                ck.cookie
                    .into_iter()
                    .zip(val.as_bytes())
                    .all(|(a, &b)| a == b)
            })
            .map(|(_idx, cookie_sess): (usize, &CookieSession)| cookie_sess.user_id)
    } else {
        None
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
        let response = next.run(req).await;
        Ok(response)
    } else {
        tracing::debug!("unauthorized request");
        let template = UnauthorizedTemplate {
            url: req.uri().to_string(),
        };
        Err((StatusCode::UNAUTHORIZED, HtmlTemplate(template)))
    }
}

async fn get_token_from_uri(query: &str, pdb: SharedPdb) -> Option<i32> {
    // Parse the query parameters for the token and check it.
    let mut user_id = None;
    for x in query.split('&') {
        let split: Vec<String> = x.split('=').map(|x| x.to_string()).collect();
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

async fn token_is_valid(token: &str, pdb: SharedPdb) -> Option<i32> {
    let pdb = pdb.lock().await;
    sqlx::query!(
        r#"SELECT user_id, is_admin FROM input_tokens JOIN users ON
           input_tokens.user_id=users.id WHERE input_token=$1"#,
        token
    )
    .fetch_one(pdb.pool())
    .await
    .unwrap()
    .user_id
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_generate_random_cookie_of_correct_len() {
        let cookie = random_cookie();
        assert_eq!(cookie.len(), COOKIE_AUTH_LEN)
    }

    #[test]
    fn should_generate_different_cookies() {
        let cookie1 = random_cookie();
        let cookie2 = random_cookie();
        assert_ne!(cookie1, cookie2);
    }

    #[test]
    fn should_parse_cookies_from_requests() {
        // No cookies header
        let request = Request::builder()
            .method("GET")
            .uri("https://www.rust-lang.org/")
            .header("X-Custom-Foo", "Bar")
            .body(())
            .unwrap();
        let cookie_map = get_cookie_map(&request);
        assert!(cookie_map.is_empty());

        // Multiple cookies in header
        let request = Request::builder()
            .method("GET")
            .uri("https://www.rust-lang.org/")
            .header("X-Custom-Foo", "Bar")
            .header(
                "Cookie",
                "guest_id=5356763797944027; \
                ct0=9e6eef8649dfec837a; \
                _twitter_sess=BAh7CSIKZmxhc2h; \
                kdt=r87TbYjr4icVWsEQk1Dq5yR; \
                twid=43503403326189569; \
                lang=en",
            )
            .body(())
            .unwrap();

        let cookie_map = get_cookie_map(&request);
        assert_eq!(
            cookie_map.get("guest_id"),
            Some(&"5356763797944027".to_string())
        );
        assert_eq!(
            cookie_map.get("ct0"),
            Some(&"9e6eef8649dfec837a".to_string())
        );
        assert_eq!(
            cookie_map.get("_twitter_sess"),
            Some(&"BAh7CSIKZmxhc2h".to_string())
        );
        assert_eq!(
            cookie_map.get("kdt"),
            Some(&"r87TbYjr4icVWsEQk1Dq5yR".to_string())
        );
        assert_eq!(
            cookie_map.get("twid"),
            Some(&"43503403326189569".to_string())
        );
        assert_eq!(cookie_map.get("lang"), Some(&"en".to_string()));
    }
}
