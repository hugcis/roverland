/// Module containing all the authentication, registration, cookies, etc. logic.

mod login;
mod middleware;
mod password_db;
mod register;

pub use login::{check_username_password, serve_login};
pub use middleware::auth as auth_middleware;
pub use password_db::{new_shared_db, PasswordDatabase, SharedPdb};
pub use register::{insert_username_password, SignUp};

/// A structure representing the user currently logged in.
#[derive(Clone)]
pub struct CurrentUser {
    /// The user identification number.
    pub user_id: i32,
    /// A bool representing wether the user is an administrator.
    pub is_admin: bool,
}

const COOKIE_AUTH_LEN: usize = 64;
const COOKIE_NAME: &str = "__Secure-roverland-auth";
