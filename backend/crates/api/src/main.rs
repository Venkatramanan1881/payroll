use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL is required")?;
    // `api migrate`: apply migrations as the connected (owner) role and exit. Serving instances never migrate.
    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let pool = sqlx::PgPool::connect(&url).await?;
        db::MIGRATOR.run(&pool).await?;
        println!("migrations applied");
        return Ok(());
    }
    let jwt_secret = std::env::var("APP_JWT_SECRET").context("APP_JWT_SECRET is required")?.into_bytes();
    let firebase_project = std::env::var("FIREBASE_PROJECT_ID").context("FIREBASE_PROJECT_ID is required")?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());

    let pool = sqlx::PgPool::connect(&url).await?;

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    let phone_verifier = std::sync::Arc::new(domain_iam::firebase::GoogleVerifier::new(&firebase_project));
    // JOB_SECRET enables /internal/jobs (Cloud Scheduler); unset in environments without timed jobs.
    let job_secret = std::env::var("JOB_SECRET").ok().filter(|s| s.len() >= 32).map(String::into_bytes);
    let app = api::router(api::AppState { pool, jwt_secret, phone_verifier, job_secret });
    // The peer address backs PC-05 when no trusted proxy header is present.
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
    Ok(())
}
