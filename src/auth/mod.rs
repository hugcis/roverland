mod login;
mod middleware;
mod password_db;
mod register;

pub use password_db::{PasswordDatabase, PasswordStorage};
pub use register::{SignUp, insert_username_password};
pub use login::{serve_login, check_username_password};
pub use middleware::auth as auth_middleware;

#[derive(Clone)]
pub struct CurrentUser {
    pub user_id: i32,
    pub is_admin: bool,
}

const COOKIE_AUTH_LEN: usize = 64;
const COOKIE_NAME: &'static str = "__Secure-roverland-auth";
