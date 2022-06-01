use clap::{Parser, Subcommand};
use overland_client::{add_register_token, create_admin, run_server};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Possible commands
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run the server
    RunServer,
    /// Manually create an administrator user. This will prompt for a username
    /// and password.
    CreateAdmin,
    /// Manually create a registration token to let a user register on the app.
    AddRegisterToken,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let cli = Args::parse();

    match &cli.command {
        Commands::RunServer => run_server().await?,
        Commands::CreateAdmin => create_admin().await?,
        Commands::AddRegisterToken => add_register_token().await?,
    };
    Ok(())
}
