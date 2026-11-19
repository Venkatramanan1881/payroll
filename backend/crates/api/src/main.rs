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
    // The vendor console (D-22) is served only where both are set: the vendor service, never the public API.
    let vendor_verifier = match (std::env::var("VENDOR_GOOGLE_CLIENT_ID"), std::env::var("VENDOR_GOOGLE_DOMAIN")) {
        (Ok(c), Ok(d)) if !c.trim().is_empty() && !d.trim().is_empty() => {
            Some(std::sync::Arc::new(domain_iam::firebase::GoogleIdVerifier::new(c.trim(), d.trim())) as std::sync::Arc<dyn domain_iam::firebase::VendorTokenVerifier>)
        }
        _ => None,
    };
    // MFA_ENCRYPTION_KEY: 32 random bytes, base64. Without it AC-08 sign-ins fail closed.
    let mfa_key = std::env::var("MFA_ENCRYPTION_KEY").ok().and_then(|k| {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.decode(k.trim()).ok().and_then(|b| <[u8; 32]>::try_from(b).ok())
    });
    if mfa_key.is_none() {
        eprintln!("MFA_ENCRYPTION_KEY is missing or not 32 bytes (base64): admin sign-in with the second step will fail");
    }
    let app = api::router(api::AppState { pool, jwt_secret, phone_verifier, job_secret, store, vendor_verifier, mfa_key });
    // The peer address backs PC-05 when no trusted proxy header is present.
    axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await?;
    Ok(())
}
