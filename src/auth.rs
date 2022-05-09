use crate::settings;
use axum::body::HttpBody;
use axum::http::header::HeaderName;
use axum::http::{header, HeaderMap, Request, StatusCode};
use axum::response::IntoResponse;
use axum::Extension;
use axum::{extract::Form, middleware::Next};
use rand::Rng;

use serde::Deserialize;
use serde::Serializer;
use sqlx::{Executor, PgPool};
use std::io::BufRead;
use std::num::NonZeroU8;
use std::sync::Arc;
use std::{collections::HashMap, num::NonZeroU32};

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

const COOKIE_AUTH_LEN: usize = 64;

#[derive(Debug)]
pub enum Error {
    WrongUsernameOrPassword,
    PasswordError,
}

#[derive(Clone)]
pub struct PasswordStorage {
    pub pool: PgPool,
}

impl PasswordStorage {
    pub async fn get(&self, username: &str) -> sqlx::Result<String> {
        sqlx::query!(r#"SELECT password from users where username=$1"#, username)
            .fetch_one(&self.pool)
            .await
            .map(|x| x.password)
    }
    pub async fn insert(&self, username: String, password: String) -> sqlx::Result<()> {
        sqlx::query!(
            r#"INSERT INTO users (username, password) VALUES ( $1, $2 )"#,
            username,
            password
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct PasswordDatabase {
    pub db_salt_component: [u8; 16],
    pub storage: PasswordStorage,
    pub sessions: Vec<[u8; COOKIE_AUTH_LEN]>,
}

impl PasswordDatabase {
    pub async fn store_password(&mut self, username: &str, password: &str) -> Result<(), Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|_| Error::PasswordError)?
            .to_string();
        self.storage
            .insert(String::from(username), password_hash)
            .await
            .map_err(|_| Error::PasswordError)
    }

    pub async fn verify_password(
        &self,
        username: &str,
        attempted_password: &str,
    ) -> Result<(), Error> {
        match self.storage.get(username).await.ok() {
            Some(actual_password) => {
                let parsed_hash =
                    PasswordHash::new(&actual_password).map_err(|_| Error::PasswordError)?;
                Argon2::default()
                    .verify_password(attempted_password.as_bytes(), &parsed_hash)
                    .map_err(|_| Error::WrongUsernameOrPassword)
            }
            None => Err(Error::WrongUsernameOrPassword),
        }
    }

    // The salt should have a user-specific component so that an attacker cannot
    // crack one password for multiple users in the database. It should have a
    // database-unique component so that an attacker cannot crack the same
    // user's password across databases in the unfortunate but common case that
    // the user has used the same password for multiple systems.
    fn salt(&self, username: &str) -> Vec<u8> {
        let mut salt = Vec::with_capacity(self.db_salt_component.len() + username.as_bytes().len());
        salt.extend(self.db_salt_component.as_ref());
        salt.extend(username.as_bytes());
        salt
    }
}

pub async fn auth<B>(
    req: Request<B>,
    next: Next<B>,
    pdb: Arc<PasswordDatabase>,
) -> impl IntoResponse {
    let cookies = req
        .headers()
        .get(header::COOKIE)
        .and_then(|header| header.to_str().ok())
        .map(|cookie_str| {
            cookie_str
                .split("; ")
                .map(|cookie_pair| cookie_pair.split(" "))
        });
    // let auth_header = req.uri().query().unwrap_or("").split("&").any(|x| {
    //     let split: Vec<String> = x.split("=").map(|x| x.to_string()).collect();
    //     if (split.len() == 2) && (split[0] == "token") && token_is_valid(&split[1], auth.clone()) {
    //         true
    //     } else {
    //         false
    //     }
    // });
    let auth_header = true;

    if auth_header {
        Ok(next.run(req).await)
    } else {
        tracing::debug!("unauthorized request");
        Err(StatusCode::UNAUTHORIZED)
    }
}

fn token_is_valid(token: &str, auth: settings::Auth) -> bool {
    token == auth.token
}

pub async fn check_username_password(
    form: Form<LogIn>,
    Extension(mut pdb): Extension<Arc<PasswordDatabase>>,
) -> impl IntoResponse {
    let log_in: LogIn = form.0;
    let mut headers = HeaderMap::new();
    match pdb
        .verify_password(&log_in.username, &log_in.password)
        .await
    {
        Ok(()) => {
            let mut rng = rand::thread_rng();
            let cookie: String = (&mut rng)
                .sample_iter(rand::distributions::Alphanumeric)
                .take(COOKIE_AUTH_LEN)
                .map(char::from)
                .collect();
            pdb.sessions.push(cookie.as_bytes().try_into().unwrap());
            headers.insert(
                header::SET_COOKIE,
                header::HeaderValue::from_str(&format!(
                    "__Secure-roverland-auth={}; Secure; SameSite=Strict",
                    cookie
                ))
                .unwrap(),
            );
            (StatusCode::OK, headers, "Login sucessful")
        }
        Err(e) => match e {
            Error::PasswordError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                headers,
                "Error fetching info",
            ),
            Error::WrongUsernameOrPassword => {
                (StatusCode::UNAUTHORIZED, headers, "Wrong username/password")
            }
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
}

pub async fn insert_username_password(
    form: Form<SignUp>,
    Extension(mut pdb): Extension<PasswordDatabase>,
) -> impl IntoResponse {
    let sign_up: SignUp = form.0;
    pdb.store_password(&sign_up.username, &sign_up.password)
        .await
        .unwrap();
    StatusCode::OK
}
