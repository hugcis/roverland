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
    RunServer,
    CreateAdmin,
    AddRegisterToken,
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let cli = Args::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Commands::RunServer => run_server().await?,
        Commands::CreateAdmin => create_admin().await?,
        Commands::AddRegisterToken => add_register_token().await?,
    };
    Ok(())
}
