//! Owner-scoped read models for the starter Home, Dashboard and Search pages.
use crate::types::{NoteSearchResults, WorkspaceOverview};
use dioxus::prelude::*;

pub const MAX_QUERY_CHARS: usize = 120;
#[cfg(feature = "server")]
const SEARCH_LIMIT: usize = 50;

#[server]
pub async fn workspace_overview() -> Result<WorkspaceOverview, ServerFnError> {
    let owner = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    overview_for_owner(&crate::db::get_pool(), owner)
        .await
        .map_err(super::database_error)
}

#[server]
pub async fn search_notes(query: String) -> Result<NoteSearchResults, ServerFnError> {
    let owner = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let query = normalize_query(&query)?;
    search_for_owner(&crate::db::get_pool(), owner, &query)
        .await
        .map_err(super::database_error)
}

#[cfg(any(feature = "server", test))]
fn normalize_query(query: &str) -> Result<String, ServerFnError> {
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(ServerFnError::new("Use a search of up to 120 characters."));
    }
    Ok(query.trim().to_owned())
}

#[cfg(feature = "server")]
async fn overview_for_owner(
    pool: &sqlx::PgPool,
    owner: uuid::Uuid,
) -> Result<WorkspaceOverview, sqlx::Error> {
    let (note_count, updated_last_seven_days, last_updated_at) =
        sqlx::query_as::<_, (i64, i64, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT COUNT(*), COUNT(*) FILTER (WHERE updated_at >= NOW() - INTERVAL '7 days'),
                MAX(updated_at) FROM notes WHERE user_id = $1",
        )
        .bind(owner)
        .fetch_one(pool)
        .await?;
    let recent_notes = sqlx::query_as::<_, crate::types::NotePreview>(
        "SELECT id, title, LEFT(body, 180) AS excerpt, updated_at
         FROM notes WHERE user_id = $1 ORDER BY updated_at DESC, id LIMIT 6",
    )
    .bind(owner)
    .fetch_all(pool)
    .await?;
    Ok(WorkspaceOverview {
        note_count,
        updated_last_seven_days,
        last_updated_at,
        recent_notes,
    })
}

#[cfg(feature = "server")]
const SEARCH_SQL: &str = "
    WITH matches AS (
        SELECT id, title, body, updated_at, STRPOS(LOWER(body), LOWER($2)) AS hit
        FROM notes
        WHERE user_id = $1
          AND (STRPOS(LOWER(title), LOWER($2)) > 0 OR STRPOS(LOWER(body), LOWER($2)) > 0)
        ORDER BY updated_at DESC, id LIMIT $3
    )
    SELECT id, title,
           (CASE WHEN hit > 40 THEN '…' ELSE '' END) ||
           SUBSTRING(body FROM GREATEST(hit - 40, 1) FOR 180) AS excerpt,
           updated_at
    FROM matches ORDER BY updated_at DESC, id";

#[cfg(feature = "server")]
async fn search_for_owner(
    pool: &sqlx::PgPool,
    owner: uuid::Uuid,
    query: &str,
) -> Result<NoteSearchResults, sqlx::Error> {
    if query.is_empty() {
        return Ok(NoteSearchResults::default());
    }
    // STRPOS deliberately treats %, _, quotes and backslashes as literal text.
    // The existing owner/updated index scopes the read and supplies stable ordering.
    let mut notes = sqlx::query_as::<_, crate::types::NotePreview>(SEARCH_SQL)
        .bind(owner)
        .bind(query)
        .bind((SEARCH_LIMIT + 1) as i64)
        .fetch_all(pool)
        .await?;
    let has_more = notes.len() > SEARCH_LIMIT;
    notes.truncate(SEARCH_LIMIT);
    Ok(NoteSearchResults { notes, has_more })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_bounds_and_normalization() {
        assert_eq!(
            normalize_query("  100%_ & ideas  ").unwrap(),
            "100%_ & ideas"
        );
        assert_eq!(normalize_query(" \n ").unwrap(), "");
        assert!(normalize_query(&"é".repeat(MAX_QUERY_CHARS)).is_ok());
        assert!(normalize_query(&"a".repeat(MAX_QUERY_CHARS + 1)).is_err());
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn read_apis_reject_missing_authentication() {
        assert!(workspace_overview().await.is_err());
        assert!(search_notes("private".into()).await.is_err());
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn search_is_owner_scoped_literal_and_returns_bounded_context() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let alice = crate::db::insert_user(&pool, "alice", "Alice", "alice@example.test", None)
            .await
            .unwrap();
        let bob = crate::db::insert_user(&pool, "bob", "Bob", "bob@example.test", None)
            .await
            .unwrap();
        for (owner, title, body) in [
            (
                alice.id,
                "100%_ & ideas",
                "A literal title match".to_owned(),
            ),
            (
                alice.id,
                "Body match",
                format!("{} NEEDLE {}", "x".repeat(300), "y".repeat(300)),
            ),
            (
                bob.id,
                "100%_ & ideas",
                "NEEDLE from another account".to_owned(),
            ),
        ] {
            sqlx::query("INSERT INTO notes(user_id,title,body) VALUES($1,$2,$3)")
                .bind(owner)
                .bind(title)
                .bind(body)
                .execute(&pool)
                .await
                .unwrap();
        }
        let found = search_for_owner(&pool, alice.id, "100%_").await.unwrap();
        assert_eq!(found.notes.len(), 1);
        assert_eq!(found.notes[0].title, "100%_ & ideas");
        let found = search_for_owner(&pool, alice.id, "needle").await.unwrap();
        assert_eq!(found.notes.len(), 1);
        assert!(found.notes[0].excerpt.contains("NEEDLE"));
        assert!(found.notes[0].excerpt.starts_with('…'));
        assert!(found.notes[0].excerpt.chars().count() <= 181);
        assert!(search_for_owner(&pool, alice.id, "' OR TRUE --")
            .await
            .unwrap()
            .notes
            .is_empty());
        assert!(search_for_owner(&pool, alice.id, "")
            .await
            .unwrap()
            .notes
            .is_empty());
    }

    #[cfg(feature = "server")]
    #[tokio::test]
    async fn overview_and_search_limits_do_not_count_another_accounts_data() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let alice = crate::db::insert_user(&pool, "alice", "Alice", "alice@example.test", None)
            .await
            .unwrap();
        let bob = crate::db::insert_user(&pool, "bob", "Bob", "bob@example.test", None)
            .await
            .unwrap();
        assert_eq!(
            overview_for_owner(&pool, alice.id)
                .await
                .unwrap()
                .note_count,
            0
        );
        sqlx::query(
            "INSERT INTO notes(user_id,title,body,updated_at)
                     SELECT $1, 'Match ' || n, 'Body', NOW() - n * INTERVAL '1 minute'
                     FROM generate_series(1,55) AS n",
        )
        .bind(alice.id)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO notes(user_id,title,body,updated_at)
                     VALUES($1,'Old note','Body',NOW()-INTERVAL '8 days'),($2,'Match hidden','Body',NOW())")
            .bind(alice.id).bind(bob.id).execute(&pool).await.unwrap();
        let overview = overview_for_owner(&pool, alice.id).await.unwrap();
        assert_eq!(overview.note_count, 56);
        assert_eq!(overview.updated_last_seven_days, 55);
        assert_eq!(overview.recent_notes.len(), 6);
        assert_eq!(overview.recent_notes[0].title, "Match 1");
        assert!(overview.last_updated_at.is_some());
        let found = search_for_owner(&pool, alice.id, "match").await.unwrap();
        assert!(found.has_more);
        assert_eq!(found.notes.len(), SEARCH_LIMIT);
        assert_eq!(found.notes[0].title, "Match 1");
        assert_eq!(found.notes.last().unwrap().title, "Match 50");
        assert_eq!(
            overview_for_owner(&pool, bob.id).await.unwrap().note_count,
            1
        );
    }
}
