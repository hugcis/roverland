use crate::auth::{PasswordDatabase, SignUp};
use crate::settings::Settings;
use sqlx::postgres::PgPoolOptions;

/// Creates an administrator user directly from the command line.
pub async fn create_admin() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.url)
        .await
        .expect("Cannot connect to postgres database.");

    let password_db = PasswordDatabase::new(&pool);

    let mut username = String::new();
    println!("Admin username:");
    std::io::stdin().read_line(&mut username).unwrap();
    username.pop();
    let password = rpassword::prompt_password("Admin password: ").unwrap();
    password_db
        .store_password(SignUp::new(&username, &password, None), true)
        .await
        .unwrap();
    Ok(())
}
