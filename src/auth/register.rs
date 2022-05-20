use crate::auth::password_db::PasswordDatabase;
use axum::{extract::Form, http::StatusCode, response::IntoResponse, Extension};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum RegisterError {
    TokenError,
    DBError,
    PasswordError,
}

#[derive(Deserialize)]
pub struct SignUp {
    pub username: String,
    pub password: String,
    pub token: Option<String>,
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
