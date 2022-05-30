use overland_client::auth::{PasswordDatabase, PasswordStorage, SignUp};
use overland_client::settings::Settings;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.url)
        .await
        .expect("Cannot connect to postgres database.");

    let password_db = PasswordDatabase {
        db_salt_component: settings.database.url[..16]
            .as_bytes()
            .try_into()
            .expect("Slice with incorrect length"),
        storage: PasswordStorage { pool: pool.clone() },
        sessions: vec![],
    };

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
