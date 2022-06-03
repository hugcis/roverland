use crate::auth::{
    login::LoginError,
    middleware::CookieSession,
    register::{RegisterError, SignUp},
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::Rng;
use sqlx::postgres::PgPool;
use std::sync::Arc;
use tokio::sync::Mutex;

const INPUT_TOKEN_LEN: usize = 64;

pub type SharedPdb = Arc<Mutex<PasswordDatabase>>;

pub fn new_shared_db(db_pool: &PgPool) -> SharedPdb {
    Arc::new(Mutex::new(PasswordDatabase::new(db_pool)))
}

#[derive(Clone)]
struct PasswordStorage {
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
pub struct PasswordDatabase {
    storage: PasswordStorage,
    pub sessions: Vec<CookieSession>,
}

impl PasswordDatabase {
    pub fn new(db_pool: &PgPool) -> PasswordDatabase {
        PasswordDatabase {
            storage: PasswordStorage {
                pool: db_pool.clone(),
            },
            sessions: vec![],
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.storage.pool
    }

    pub async fn store_password(
        &self,
        sign_up: SignUp,
        is_admin: bool,
    ) -> Result<(), RegisterError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(sign_up.password.as_bytes(), &salt)
            .map_err(|_| RegisterError::Password)?
            .to_string();
        if is_admin
            || self
                .storage
                .check_token(&sign_up.token.unwrap_or_else(|| "".to_string()))
                .await
        {
            self.storage
                .insert(sign_up.username, password_hash, is_admin)
                .await
                .map_err(|_| RegisterError::DB)
        } else {
            Err(RegisterError::Token)
        }
    }

    pub async fn verify_password(
        &self,
        username: &str,
        attempted_password: &str,
    ) -> Result<i32, LoginError> {
        match self.storage.get(username).await.ok() {
            Some((id, actual_password)) => {
                let parsed_hash =
                    PasswordHash::new(&actual_password).map_err(|_| LoginError::PasswordError)?;
                Argon2::default()
                    .verify_password(attempted_password.as_bytes(), &parsed_hash)
                    .map_err(|_| LoginError::WrongUsernameOrPassword)?;
                Ok(id)
            }
            None => Err(LoginError::WrongUsernameOrPassword),
        }
    }
}
