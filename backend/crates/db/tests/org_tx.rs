use sqlx::PgPool;
use uuid::Uuid;

const ORG_A: Uuid = Uuid::from_u128(0xA);
const ORG_B: Uuid = Uuid::from_u128(0xB);

/// Two organisations with one branch each, inserted as the superuser (bypasses RLS).
async fn seed(pool: &PgPool) {
    sqlx::query("INSERT INTO platform.organisation (id, legal_name, display_name) VALUES ($1,'A','A'), ($2,'B','B')")
        .bind(ORG_A).bind(ORG_B).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO org.branch (org_id, code, name) VALUES ($1,'A1','A1'), ($2,'B1','B1')")
        .bind(ORG_A).bind(ORG_B).execute(pool).await.unwrap();
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn sees_only_own_org(pool: PgPool) {
    seed(&pool).await;
    let mut tx = db::begin_org(&pool, ORG_A).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM org.branch").fetch_one(&mut **tx).await.unwrap();
    assert_eq!(n, 1);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn unset_org_sees_nothing(pool: PgPool) {
    seed(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL ROLE app_api").execute(&mut *tx).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM org.branch").fetch_one(&mut *tx).await.unwrap();
    assert_eq!(n, 0);
}

#[sqlx::test(migrator = "db::MIGRATOR")]
async fn cannot_write_other_org(pool: PgPool) {
    seed(&pool).await;
    let mut tx = db::begin_org(&pool, ORG_B).await.unwrap();
    let err = sqlx::query("INSERT INTO org.branch (org_id, code, name) VALUES ($1,'X','X')")
        .bind(ORG_A).execute(&mut **tx).await.unwrap_err();
    assert_eq!(err.as_database_error().unwrap().code().as_deref(), Some("42501"));
}
