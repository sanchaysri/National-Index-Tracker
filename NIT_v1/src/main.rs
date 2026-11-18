use nit_v1::{config::Config, db::connection::create_pool, state::AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    let pool = create_pool(&config.database_url).await?;

    let state = AppState { db: pool };

    // 5. Test the database connection
    let result: (i32,) = sqlx::query_as("SELECT 1").fetch_one(&state.db).await?;

    println!("Database connected successfully!");
    println!("PostgreSQL returned: {}", result.0);

    Ok(())
}
