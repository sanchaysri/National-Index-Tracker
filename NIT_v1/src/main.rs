use nit_v1::{build_router, config::Config, db::connection::create_pool, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load environment variables.
    dotenvy::dotenv().ok();

    // 2. Read application configuration.
    let config = Config::from_env()?;

    // 3. Create the database connection pool.
    let pool = create_pool(&config.database_url).await?;

    println!("PostgreSQL connected successfully!");

    // 4. Create shared application state.
    let state = AppState { db: pool };

    // 5. Construct the Axum router.
    let app = build_router(state);

    // 6. Bind the HTTP server to port 3000.
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    println!("NIT server running on http://localhost:3000");

    // 7. Start serving incoming requests.
    axum::serve(listener, app).await?;

    Ok(())
}
