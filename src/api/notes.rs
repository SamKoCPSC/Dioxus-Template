use crate::types::Note;
use dioxus::prelude::*;
use uuid::Uuid;
#[server]
pub async fn list_notes() -> Result<Vec<Note>, ServerFnError> {
    let user = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    sqlx::query_as::<_,Note>("SELECT id,title,body,updated_at FROM notes WHERE user_id=$1 ORDER BY updated_at DESC,id LIMIT 100").bind(user).fetch_all(&crate::db::get_pool()).await.map_err(super::database_error)
}
#[server]
pub async fn get_note(id: Uuid) -> Result<Note, ServerFnError> {
    let user = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    sqlx::query_as::<_, Note>(
        "SELECT id,title,body,updated_at FROM notes WHERE id=$1 AND user_id=$2",
    )
    .bind(id)
    .bind(user)
    .fetch_optional(&crate::db::get_pool())
    .await
    .map_err(super::database_error)?
    .ok_or_else(|| ServerFnError::new("Note unavailable"))
}
#[cfg(any(feature = "server", test))]
fn validate(title: &str, body: &str) -> Result<(), ServerFnError> {
    if title.trim().is_empty() || title.chars().count() > 160 || body.chars().count() > 20000 {
        return Err(ServerFnError::new(
            "Use a title of 1–160 characters and a note of up to 20,000 characters.",
        ));
    }
    Ok(())
}
#[server]
pub async fn save_note(
    expected: Option<Uuid>,
    id: Option<Uuid>,
    title: String,
    body: String,
) -> Result<Note, ServerFnError> {
    validate(&title, &body)?;
    let (user, token) = super::mutation_identity(expected).await?;
    let mut tx = crate::db::get_pool()
        .begin()
        .await
        .map_err(super::database_error)?;
    crate::auth::session::lock_active_session_for_user(&mut tx, &token, user)
        .await
        .map_err(|_| ServerFnError::new("Not authenticated"))?;
    let note = write_note(&mut tx, user, id, &title, &body)
        .await
        .map_err(super::database_error)?
        .ok_or_else(|| ServerFnError::new("Note unavailable"))?;
    tx.commit().await.map_err(super::database_error)?;
    Ok(note)
}
#[cfg(feature = "server")]
async fn write_note(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user: Uuid,
    id: Option<Uuid>,
    title: &str,
    body: &str,
) -> Result<Option<Note>, sqlx::Error> {
    if let Some(id) = id {
        sqlx::query_as::<_,Note>("UPDATE notes SET title=$3,body=$4,updated_at=NOW() WHERE id=$1 AND user_id=$2 RETURNING id,title,body,updated_at").bind(id).bind(user).bind(title.trim()).bind(body).fetch_optional(&mut **tx).await
    } else {
        sqlx::query_as::<_,Note>("INSERT INTO notes(user_id,title,body) VALUES($1,$2,$3) RETURNING id,title,body,updated_at").bind(user).bind(title.trim()).bind(body).fetch_optional(&mut **tx).await
    }
}
#[server]
pub async fn delete_note(expected: Option<Uuid>, id: Uuid) -> Result<(), ServerFnError> {
    let (user, token) = super::mutation_identity(expected).await?;
    let mut tx = crate::db::get_pool()
        .begin()
        .await
        .map_err(super::database_error)?;
    crate::auth::session::lock_active_session_for_user(&mut tx, &token, user)
        .await
        .map_err(|_| ServerFnError::new("Not authenticated"))?;
    let affected = sqlx::query("DELETE FROM notes WHERE id=$1 AND user_id=$2")
        .bind(id)
        .bind(user)
        .execute(&mut *tx)
        .await
        .map_err(super::database_error)?
        .rows_affected();
    if affected != 1 {
        return Err(ServerFnError::new("Note unavailable"));
    }
    tx.commit().await.map_err(super::database_error)?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn note_bounds() {
        assert!(validate(" ", "body").is_err());
        assert!(validate("Title", &"a".repeat(20001)).is_err());
        assert!(validate("Title", "Body").is_ok());
    }
    #[cfg(feature = "server")]
    #[tokio::test]
    async fn another_user_cannot_update_or_read_private_note() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let alice = crate::db::insert_user(&pool, "alice", "Alice", "alice@example.test", None)
            .await
            .unwrap();
        let bob = crate::db::insert_user(&pool, "bob", "Bob", "bob@example.test", None)
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        let note = write_note(&mut tx, alice.id, None, "Private", "Original")
            .await
            .unwrap()
            .unwrap();
        tx.commit().await.unwrap();
        let mut tx = pool.begin().await.unwrap();
        assert!(write_note(&mut tx, bob.id, Some(note.id), "Stolen", "Oops")
            .await
            .unwrap()
            .is_none());
        tx.commit().await.unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM notes WHERE id=$1 AND user_id=$2")
                .bind(note.id)
                .bind(bob.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(count, 0);
        let body: String = sqlx::query_scalar("SELECT body FROM notes WHERE id=$1")
            .bind(note.id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(body, "Original");
    }
}
