//! Each integration test owns a disposable PostgreSQL database.
use sqlx::PgPool;
pub fn uid() -> String {
    uuid::Uuid::new_v4().simple().to_string()[..8].to_string()
}
pub async fn setup_test_db() -> (pgtemp::PgTempDB, PgPool) {
    let db = pgtemp::PgTempDB::async_new().await;
    let pool = PgPool::connect(&db.connection_uri()).await.unwrap();
    sqlx::raw_sql(include_str!("../migrations/schema.sql"))
        .execute(&pool)
        .await
        .unwrap();
    (db, pool)
}
