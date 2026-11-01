//! Route protection middleware.
//!
//! Verifies the session cookie and either:
//! - Injects the authenticated user into request extensions, or
//! - Redirects unauthenticated users away from protected routes to `/login`.

use std::collections::HashSet;
use std::sync::LazyLock;

use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::http::Response;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::extract::cookie::CookieJar;
use sqlx::PgPool;

use crate::auth::context::{AuthUser, AuthUserProfile, UserProfile};
use crate::auth::session;
use crate::db;

/// Check if a path matches a protected route pattern.
///
/// Handles both exact matches and parameterized routes like `/notes/:uuid`.
fn is_protected_path(path: &str) -> bool {
    path == "/dashboard"
        || path == "/notes"
        || path.starts_with("/notes/")
        || path == "/settings/profile"
        || path == "/settings/accounts"
}

/// Routes that redirect authenticated users away.
static REDIRECT_IF_AUTHED_PATHS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| ["/login"].into_iter().collect());

/// Axum middleware handler for route protection.
///
/// Reads the session cookie, verifies the JWT, and either:
/// - Injects `AuthUser` and `AuthUserProfile` into request extensions for downstream handlers
/// - Returns a 302 redirect to `/login` for unauthenticated users on protected paths
/// - Returns a 302 redirect to `/home` for authenticated users on `/login`
pub async fn handle_auth(
    State(pool): State<PgPool>,
    mut req: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = req.uri().path().to_string();
    let original_target = req
        .uri()
        .path_and_query()
        .map_or_else(|| path.clone(), |target| target.as_str().to_string());

    // Extract session cookie from headers
    let jar = CookieJar::from_headers(req.headers());

    // Check for valid session
    let configured_cookie_name = session::cookie_name();
    let session_token = jar.get(configured_cookie_name.as_ref());
    let verified_user_id = if let Some(cookie) = session_token {
        session::verify_session(&pool, cookie.value()).await.ok()
    } else {
        None
    };

    let is_authenticated = verified_user_id.is_some();

    // Check if path is protected
    let is_protected = is_protected_path(&path);
    let is_redirect_if_authed = REDIRECT_IF_AUTHED_PATHS.contains(path.as_str());

    // Redirect authenticated users away from login
    if is_authenticated && is_redirect_if_authed {
        return redirect_to("/dashboard");
    }

    // Redirect unauthenticated users from protected paths to login
    if !is_authenticated && is_protected {
        let encoded = percent_encoding::utf8_percent_encode(
            &original_target,
            percent_encoding::NON_ALPHANUMERIC,
        )
        .to_string();
        let location = format!("/login?redirect_uri={encoded}");
        return redirect_to(&location);
    }

    // Inject user into extensions if authenticated
    if let Some(user_id) = verified_user_id {
        req.extensions_mut().insert(AuthUser { user_id });
        if let Ok(Some(user)) = db::get_user_by_id(&pool, user_id).await {
            let profile = UserProfile {
                id: user.id,
                username: user.username,
                display_name: user.display_name,
                email: user.email,
                avatar_url: user.avatar_url,
                bio: user.bio,
            };
            req.extensions_mut().insert(AuthUserProfile { profile });
        }
    }

    // Continue to the next handler
    let mut response: Response<Body> = next.run(req).await;

    // Auth-derived documents and server-function/profile responses must not be shared-cached.
    let document = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.starts_with("text/html"));
    if document || path.starts_with("/api/") {
        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("private, no-store"),
        );
    }

    // Rolling session refresh: if token is old but still valid, issue a new one
    if let Some(_user_id) = verified_user_id {
        if let Some(cookie) = session_token {
            if session::should_refresh(&pool, cookie.value())
                .await
                .unwrap_or(false)
            {
                if let Ok(new_token) = session::refresh_session(&pool, cookie.value()).await {
                    let new_cookie = session::build_session_cookie(&new_token);
                    response.headers_mut().insert(
                        axum::http::header::SET_COOKIE,
                        new_cookie
                            .to_string()
                            .parse()
                            .expect("cookie string is valid HeaderValue"),
                    );
                }
            }
        }
    }

    response
}

fn redirect_to(location: &str) -> Response<Body> {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::LOCATION,
        location
            .parse()
            .expect("redirect location must be a valid HeaderValue"),
    );
    (StatusCode::FOUND, headers, Body::empty()).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn documents_and_api_responses_are_not_shared_cached() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        let router = axum::Router::new()
            .route(
                "/",
                axum::routing::get(|| async { axum::response::Html("page") }),
            )
            .route("/api/example", axum::routing::get(|| async { "data" }))
            .layer(axum::middleware::from_fn_with_state(pool, handle_auth));
        for path in ["/", "/api/example"] {
            let response = router
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(
                response.headers()[axum::http::header::CACHE_CONTROL],
                "private, no-store"
            );
        }
    }
    #[tokio::test]
    async fn guest_gate_preserves_the_exact_return_path() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@127.0.0.1/test")
            .unwrap();
        let router = axum::Router::new()
            .route("/notes/new", axum::routing::get(|| async { "private" }))
            .layer(axum::middleware::from_fn_with_state(pool, handle_auth));
        let path = "/notes/new?from=saved%20ideas";
        let response = router
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FOUND);
        let expected = format!(
            "/login?redirect_uri={}",
            percent_encoding::utf8_percent_encode(path, percent_encoding::NON_ALPHANUMERIC)
        );
        assert_eq!(response.headers()[axum::http::header::LOCATION], expected);
    }
}
