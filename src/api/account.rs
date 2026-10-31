use crate::auth::context::UserProfile;
use crate::types::LinkedAccount;
use dioxus::prelude::*;
use uuid::Uuid;
#[server]
pub async fn linked_accounts() -> Result<Vec<LinkedAccount>, ServerFnError> {
    let user = crate::auth::session::extract_user_id_from_fullstack()
        .await
        .ok_or_else(|| ServerFnError::new("Not authenticated"))?;
    let accounts = crate::db::get_oauth_accounts_by_user_id(&crate::db::get_pool(), user)
        .await
        .map_err(super::database_error)?;
    Ok(accounts
        .into_iter()
        .map(|a| LinkedAccount {
            id: a.id,
            provider: a.provider,
            email: a.email,
        })
        .collect())
}
#[server]
pub async fn update_profile(
    expected: Option<Uuid>,
    name: String,
    bio: String,
) -> Result<UserProfile, ServerFnError> {
    if name.trim().is_empty() || name.chars().count() > 100 || bio.chars().count() > 2000 {
        return Err(ServerFnError::new(
            "Use a name of 1–100 characters and a bio up to 2,000 characters.",
        ));
    }
    let (user, token) = super::mutation_identity(expected).await?;
    let mut tx = crate::db::get_pool()
        .begin()
        .await
        .map_err(super::database_error)?;
    crate::auth::session::lock_active_session_for_user(&mut tx, &token, user)
        .await
        .map_err(|_| ServerFnError::new("Not authenticated"))?;
    let row=sqlx::query_as::<_,crate::db::User>("UPDATE users SET display_name=$2,bio=$3,updated_at=NOW() WHERE id=$1 RETURNING id,username,display_name,email,avatar_url,bio,created_at,updated_at").bind(user).bind(name.trim()).bind(bio).fetch_one(&mut *tx).await.map_err(super::database_error)?;
    tx.commit().await.map_err(super::database_error)?;
    Ok(UserProfile {
        id: row.id,
        username: row.username,
        display_name: row.display_name,
        email: row.email,
        avatar_url: row.avatar_url,
        bio: row.bio,
    })
}
#[server]
pub async fn unlink_account(expected: Option<Uuid>, id: Uuid) -> Result<(), ServerFnError> {
    let (user, token) = super::mutation_identity(expected).await?;
    let mut tx = crate::db::get_pool()
        .begin()
        .await
        .map_err(super::database_error)?;
    crate::auth::session::lock_active_session_for_user(&mut tx, &token, user)
        .await
        .map_err(|_| ServerFnError::new("Not authenticated"))?;
    // Serialize concurrent unlinks so they cannot both remove the last method.
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::database_error)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM oauth_accounts WHERE user_id=$1")
        .bind(user)
        .fetch_one(&mut *tx)
        .await
        .map_err(super::database_error)?;
    if count < 2 {
        return Err(ServerFnError::new(
            "Keep at least one sign-in method linked.",
        ));
    }
    let account = sqlx::query_as::<_, crate::db::OauthAccount>(
        "DELETE FROM oauth_accounts WHERE id=$1 AND user_id=$2 RETURNING *",
    )
    .bind(id)
    .bind(user)
    .fetch_optional(&mut *tx)
    .await
    .map_err(super::database_error)?
    .ok_or_else(|| ServerFnError::new("Account unavailable"))?;
    tx.commit().await.map_err(super::database_error)?;
    crate::auth::revoke::revoke_account(&account).await;
    Ok(())
}
