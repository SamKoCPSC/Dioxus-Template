//! Development tools never render in a hosted or release application.
use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub(crate) async fn guard_showcase(
    State(local): State<bool>,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    let path = percent_encoding::percent_decode_str(request.uri().path()).decode_utf8_lossy();
    if (path == "/dev/components" || path.starts_with("/dev/components/"))
        && !crate::pages::showcase::available(local)
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;
    fn router(local: bool) -> axum::Router {
        axum::Router::new()
            .fallback(|| async { "example" })
            .layer(axum::middleware::from_fn_with_state(local, guard_showcase))
    }
    #[tokio::test]
    async fn hosted_showcase_urls_are_404_without_affecting_normal_pages() {
        for path in [
            "/dev/components",
            "/dev/components/",
            "/dev/%63omponents",
            "/dev/components?theme=dark",
        ] {
            let response = router(false)
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        }
        let response = router(false)
            .oneshot(
                Request::builder()
                    .uri("/about")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    #[tokio::test]
    async fn local_showcase_requires_a_debug_build_too() {
        let response = router(true)
            .oneshot(
                Request::builder()
                    .uri("/dev/components")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            if cfg!(debug_assertions) {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            }
        );
    }
}
