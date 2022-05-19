use crate::HtmlTemplate;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use askama::Template;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Extension;
use axum::{extract::Form, middleware::Next};
use rand::Rng;
use serde::Deserialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

const COOKIE_AUTH_LEN: usize = 64;
const INPUT_TOKEN_LEN: usize = 64;
const COOKIE_NAME: &'static str = "__Secure-roverland-auth";

#[derive(Debug)]
pub enum Error {
    WrongUsernameOrPassword,
    PasswordError,
}

#[derive(Debug)]
pub enum RegisterError {
    TokenError,
    DBError,
    PasswordError,
}

#[derive(Clone)]
pub struct PasswordStorage {
    pub pool: PgPool,
}

impl PasswordStorage {
    // Retreive the user id and password hash from a username
    pub async fn get(&self, username: &str) -> sqlx::Result<(i32, String)> {
        sqlx::query!(
            r#"SELECT id, password from users where username=$1"#,
            username
        )
        .fetch_one(&self.pool)
        .await
        .map(|x| (x.id, x.password))
    }

    pub async fn insert(
        &self,
        username: String,
        password: String,
        is_admin: bool,
    ) -> sqlx::Result<()> {
        let user = sqlx::query!(
            r#"INSERT INTO users (username, password, is_admin) VALUES ( $1, $2, $3 ) RETURNING users.id"#,
            username,
            password,
            is_admin
        )
        .fetch_one(&self.pool)
        .await?;
        // Create an input token on user creation
        self.create_input_token(user.id).await?;
        Ok(())
    }

    pub async fn create_input_token(&self, user_id: i32) -> sqlx::Result<()> {
        let input_token: String = {
            let mut rng = rand::thread_rng();
            (&mut rng)
                .sample_iter(rand::distributions::Alphanumeric)
                .take(INPUT_TOKEN_LEN)
                .map(char::from)
                .collect()
        };
        sqlx::query!(
            r#"INSERT INTO input_tokens (input_token, valid, user_id) VALUES ( $1, $2, $3 )"#,
            input_token,
            true,
            user_id,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn check_token(&self, token: &str) -> bool {
        sqlx::query!(
            r#"UPDATE register_tokens SET used=TRUE
               WHERE register_tokens.register_token=$1 RETURNING
               register_tokens.register_token"#,
            token
        )
        .fetch_one(&self.pool)
        .await
        .is_ok()
    }
}

#[derive(Clone)]
pub struct CurrentUser {
    pub user_id: i32,
}

#[derive(Clone, Debug)]
pub struct CookieSession {
    cookie: [u8; COOKIE_AUTH_LEN],
    user_id: i32,
}

#[derive(Clone)]
pub struct PasswordDatabase {
    pub db_salt_component: [u8; 16],
    pub storage: PasswordStorage,
    pub sessions: Vec<CookieSession>,
}

impl PasswordDatabase {
    pub async fn store_password(
        &self,
        sign_up: SignUp,
        is_admin: bool,
    ) -> Result<(), RegisterError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(sign_up.password.as_bytes(), &salt)
            .map_err(|_| RegisterError::PasswordError)?
            .to_string();
        if is_admin
            || self
                .storage
                .check_token(&sign_up.token.unwrap_or("".to_string()))
                .await
        {
            self.storage
                .insert(String::from(sign_up.username), password_hash, is_admin)
                .await
                .map_err(|_| RegisterError::DBError)
        } else {
            Err(RegisterError::TokenError)
        }
    }

    pub async fn verify_password(
        &self,
        username: &str,
        attempted_password: &str,
    ) -> Result<i32, Error> {
        match self.storage.get(username).await.ok() {
            Some((id, actual_password)) => {
                let parsed_hash =
                    PasswordHash::new(&actual_password).map_err(|_| Error::PasswordError)?;
                Argon2::default()
                    .verify_password(attempted_password.as_bytes(), &parsed_hash)
                    .map_err(|_| Error::WrongUsernameOrPassword)?;
                Ok(id)
            }
            None => Err(Error::WrongUsernameOrPassword),
        }
    }

    // fn salt(&self, username: &str) -> Vec<u8> {
    //     let mut salt = Vec::with_capacity(self.db_salt_component.len() + username.as_bytes().len());
    //     salt.extend(self.db_salt_component.as_ref());
    //     salt.extend(username.as_bytes());
    //     salt
    // }
}

pub async fn auth<B>(
    mut req: Request<B>,
    next: Next<B>,
    pdb: Arc<Mutex<PasswordDatabase>>,
) -> impl IntoResponse {
    let cookies: Option<HashMap<String, String>> = req
        .headers()
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
        });

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
        let current_user = CurrentUser { user_id };
        req.extensions_mut().insert(current_user);
        Ok(next.run(req).await)
    } else {
        tracing::debug!("unauthorized request");
        let template = Unauthorized {};
        Err((StatusCode::UNAUTHORIZED, HtmlTemplate(template)))
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

pub async fn serve_login() -> impl IntoResponse {
    tracing::debug!("unauthorized request");
    let template = LoginTemplate {};
    (StatusCode::OK, HtmlTemplate(template))
}

#[derive(Template)]
#[template(path = "unauthorized.html")]
struct Unauthorized {}

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
        r#"SELECT user_id FROM input_tokens WHERE input_token=$1"#,
        token
    )
    .fetch_one(&pdb.storage.pool)
    .await
    .unwrap()
    .user_id
}

pub async fn check_username_password(
    form: Form<LogIn>,
    Extension(pdb): Extension<Arc<Mutex<PasswordDatabase>>>,
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
            Ok((StatusCode::OK, headers, "Login sucessful".to_string()))
        }
        Err(e) => match e {
            Error::PasswordError => Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                "Error fetching info".to_string(),
            )),
            Error::WrongUsernameOrPassword => Err((
                StatusCode::UNAUTHORIZED,
                headers,
                "Wrong username/password".to_string(),
            )),
        },
    }
}

#[derive(Deserialize, Debug)]
pub struct LogIn {
    username: String,
    password: String,
    remember: String,
}

#[derive(Deserialize, Debug)]
pub struct SignUp {
    username: String,
    password: String,
    token: Option<String>,
}

impl SignUp {
    pub fn new(username: &str, password: &str, token: Option<&str>) -> SignUp {
        SignUp {
            username: username.to_string(),
            password: password.to_string(),
            token: token.map(|x| x.to_string()),
        }
    }
}

pub async fn insert_username_password(
    form: Form<SignUp>,
    Extension(pdb): Extension<Arc<Mutex<PasswordDatabase>>>,
) -> impl IntoResponse {
    let pdb = pdb.lock().await;
    let sign_up: SignUp = form.0;
    let store_password_res = pdb.store_password(sign_up, false).await;
    match store_password_res {
        Ok(_) => (StatusCode::OK, "New user created"),
        Err(RegisterError::TokenError) => {
            tracing::debug!("wrong token used");
            (StatusCode::UNAUTHORIZED, "Incorrect token")
        }
        Err(RegisterError::DBError) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
        }
        Err(RegisterError::PasswordError) => {
            tracing::debug!("error hashing password");
            (StatusCode::INTERNAL_SERVER_ERROR, "Password error")
        }
    }
}
