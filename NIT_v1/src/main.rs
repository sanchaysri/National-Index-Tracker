use std::{
    fs::{self, File, OpenOptions},
    io,
    process::ExitCode,
    sync::Mutex,
};

use nit_v1::{build_router, config::Config, db::connection::create_pool, state::AppState};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

fn open_log_file() -> io::Result<File> {
    fs::create_dir_all("logs")?;
    OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/nit.log")
}

#[tokio::main]
async fn main() -> ExitCode {
    dotenvy::dotenv().ok();

    let Ok(log_file) = open_log_file() else {
        return ExitCode::FAILURE;
    };

    tracing_subscriber::fmt()
        .json()
        .with_ansi(false)
        .with_writer(Mutex::new(log_file))
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("nit_v1=trace")),
        )
        .init();

    if let Err(stage) = run_server().await {
        error!(
            event = "server_failed",
            stage, "NIT server stopped with an error"
        );
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

async fn run_server() -> Result<(), &'static str> {
    info!(event = "server_starting", "NIT server starting");

    debug!(
        event = "configuration_loading",
        "Loading application configuration"
    );
    let config = Config::from_env().map_err(|_| "load_configuration")?;

    debug!(event = "database_connecting", "Connecting to PostgreSQL");
    let pool = create_pool(&config.database_url)
        .await
        .map_err(|_| "connect_database")?;

    info!(
        event = "database_connected",
        "PostgreSQL connected successfully"
    );

    let app = build_router(AppState { db: pool });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .map_err(|_| "bind_listener")?;

    info!(event = "server_started", port = 3000, "NIT server running");

    axum::serve(listener, app)
        .await
        .map_err(|_| "serve_requests")
}
