use crate::auth::SharedPdb;
use axum::{extract::Form, http::StatusCode, response::IntoResponse, Extension};
use serde::Deserialize;

#[derive(Debug)]
pub enum RegisterError {
    Token,
    DB,
    Password,
}

/// A registration request object, with a username, password and token.
#[derive(Deserialize)]
pub struct SignUp {
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Registration token (optional, for restricted registration)
    pub token: Option<String>,
}

impl SignUp {
    /// Creates a new `Signup` object.
    pub fn new(username: &str, password: &str, token: Option<&str>) -> SignUp {
        SignUp {
            username: username.to_string(),
            password: password.to_string(),
            token: token.map(|x| x.to_string()),
        }
    }
}

/// This function inserts a new user in the database.
pub async fn insert_username_password(
    form: Form<SignUp>,
    Extension(pdb): Extension<SharedPdb>,
) -> impl IntoResponse {
    let pdb = pdb.lock().await;
    let sign_up: SignUp = form.0;
    let store_password_res = pdb.store_password(sign_up, false).await;
    match store_password_res {
        Ok(_) => (StatusCode::OK, "New user created"),
        Err(RegisterError::Token) => {
            tracing::debug!("wrong token used");
            (StatusCode::UNAUTHORIZED, "Incorrect token")
        }
        Err(RegisterError::DB) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error"),
        Err(RegisterError::Password) => {
            tracing::debug!("error hashing password");
            (StatusCode::INTERNAL_SERVER_ERROR, "Password error")
        }
    }
}
