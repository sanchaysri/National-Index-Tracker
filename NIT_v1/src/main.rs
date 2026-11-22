use nit_v1::{build_router, config::Config, db::connection::create_pool, state::AppState};

use tracing::{Level, info};
use tracing_subscriber::{
    EnvFilter, Layer, filter::filter_fn, layer::SubscriberExt, util::SubscriberInitExt,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load environment variables
    dotenvy::dotenv().ok();

    // 2. Create log directories
    std::fs::create_dir_all("logs/registration")?;
    std::fs::create_dir_all("logs/login")?;

    // 3. General application logs
    let general_file = tracing_appender::rolling::daily("logs", "nit.log");

    let (general_writer, _general_guard) = tracing_appender::non_blocking(general_file);

    // 4. Registration logs
    let registration_file =
        tracing_appender::rolling::daily("logs/registration", "registration.log");

    let (registration_writer, _registration_guard) =
        tracing_appender::non_blocking(registration_file);

    // 5. Login logs
    let login_file = tracing_appender::rolling::daily("logs/login", "login.log");

    let (login_writer, _login_guard) = tracing_appender::non_blocking(login_file);

    // 6. Console logging
    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("nit_v1=debug")),
        );

    // 7. General application logging
    let general_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(general_writer)
        .with_filter(filter_fn(|metadata| {
            metadata.target().starts_with("nit_v1")
                && metadata.target() != "nit_v1::auth::registration"
                && metadata.target() != "nit_v1::auth::login"
                && *metadata.level() <= Level::DEBUG
        }));

    // 8. Registration logging
    let registration_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(registration_writer)
        .with_filter(filter_fn(|metadata| {
            metadata.target() == "nit_v1::auth::registration" && *metadata.level() <= Level::DEBUG
        }));

    // 9. Login logging
    let login_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_writer(login_writer)
        .with_filter(filter_fn(|metadata| {
            metadata.target() == "nit_v1::auth::login" && *metadata.level() <= Level::DEBUG
        }));

    // 10. Initialize logging
    tracing_subscriber::registry()
        .with(console_layer)
        .with(general_layer)
        .with(registration_layer)
        .with(login_layer)
        .init();

    // 11. Read application configuration
    let config = Config::from_env()?;

    // 12. Connect to PostgreSQL
    let pool = create_pool(&config.database_url).await?;

    info!(
        event = "database_connected",
        "PostgreSQL connected successfully"
    );

    // 13. Create application state
    let state = AppState { db: pool };

    // 14. Build Axum router
    let app = build_router(state);

    // 15. Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    info!(event = "server_started", port = 3000, "NIT server running");

    axum::serve(listener, app).await?;

    Ok(())
}
