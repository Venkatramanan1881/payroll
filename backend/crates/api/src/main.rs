use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    let jwt_secret = std::env::var("APP_JWT_SECRET").context("APP_JWT_SECRET is required")?.into_bytes();
    let firebase_project = std::env::var("FIREBASE_PROJECT_ID").context("FIREBASE_PROJECT_ID is required")?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    let pool = sqlx::PgPool::connect(&url).await?;
    // Migrations never run implicitly; deployment runs them as a separate, approved step (MIGRATIONS.md §3).
    if std::env::var("RUN_MIGRATIONS").as_deref() == Ok("1") {
        db::MIGRATOR.run(&pool).await?;
    }

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    let phone_verifier = std::sync::Arc::new(domain_iam::firebase::GoogleVerifier::new(&firebase_project));
    axum::serve(listener, api::router(api::AppState { pool, jwt_secret, phone_verifier })).await?;
    Ok(())
}
