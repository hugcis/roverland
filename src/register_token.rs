use crate::settings::Settings;
use sqlx::postgres::PgPoolOptions;
use rand::Rng;

const REGISTER_TOKEN_LEN: usize = 64;

pub async fn add_register_token() -> Result<(), sqlx::Error> {
    let settings = Settings::new().unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(settings.database.max_connections)
        .connect(&settings.database.url)
        .await
        .expect("Cannot connect to postgres database.");

    let register_token: String = {
        let mut rng = rand::thread_rng();
        (&mut rng)
            .sample_iter(rand::distributions::Alphanumeric)
            .take(REGISTER_TOKEN_LEN)
            .map(char::from)
            .collect()
    };
    sqlx::query!(
        r#"INSERT INTO register_tokens (register_token, used) VALUES ( $1, $2 )"#,
        register_token,
        false,
    )
    .execute(&pool)
    .await?;

    Ok(())
}
