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
    let job_secret = match std::env::var("JOB_SECRET").map(|s| s.trim().to_string()) {
        Ok(s) if s.len() >= 32 => Some(s.into_bytes()),
        Ok(_) => {
            eprintln!("JOB_SECRET is shorter than 32 characters: /internal/jobs stays disabled");
            None
        }
        Err(_) => None,
    };
    // EXPORT_BUCKET (D-19) enables payroll input exports; the service account needs object create on the bucket and
    // roles/iam.serviceAccountTokenCreator on itself for signed links.
    let store = std::env::var("EXPORT_BUCKET").ok().filter(|b| !b.trim().is_empty()).map(|b| {
        std::sync::Arc::new(api::storage::Gcs::new(b.trim().to_string())) as std::sync::Arc<dyn api::storage::ObjectStore>
    });
    if store.is_none() {
        eprintln!("EXPORT_BUCKET is not set: payroll input exports are disabled");
    }
    let app = api::router(api::AppState { pool, jwt_secret, phone_verifier, job_secret, store });
    // The peer address backs PC-05 when no trusted proxy header is present.
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
    Ok(())
}
