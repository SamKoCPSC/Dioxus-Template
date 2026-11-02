pub mod account;
pub mod notes;
pub mod workspace;
#[cfg(feature = "server")]
pub(crate) async fn mutation_identity(
    expected: Option<uuid::Uuid>,
) -> Result<(uuid::Uuid, String), dioxus::prelude::ServerFnError> {
    use dioxus::prelude::ServerFnError;
    let token = {
        let context = dioxus::fullstack::FullstackContext::current()
            .ok_or_else(|| ServerFnError::new("Request required"))?;
        let parts = context.parts_mut();
        let configured = url::Url::parse(
            &std::env::var("BASE_URL").map_err(|_| ServerFnError::new("Request rejected"))?,
        )
        .map_err(|_| ServerFnError::new("Request rejected"))?
        .origin();
        let mut origins = parts.headers.get_all(axum::http::header::ORIGIN).iter();
        let origin = origins
            .next()
            .and_then(|v| v.to_str().ok())
            .and_then(|v| url::Url::parse(v).ok());
        if parts.method != axum::http::Method::POST
            || origins.next().is_some()
            || origin.as_ref().is_none_or(|v| {
                v.origin() != configured
                    || v.path() != "/"
                    || v.query().is_some()
                    || v.fragment().is_some()
                    || !v.username().is_empty()
                    || v.password().is_some()
            })
        {
            return Err(ServerFnError::new("Request rejected"));
        }
        let jar = axum_extra::extract::cookie::CookieJar::from_headers(&parts.headers);
        jar.get(crate::auth::session::cookie_name().as_ref())
            .ok_or_else(|| ServerFnError::new("Not authenticated"))?
            .value()
            .to_owned()
    };
    let user = crate::auth::account_binding::require_form_account(expected).await?;
    Ok((user, token))
}
#[cfg(feature = "server")]
pub(crate) fn database_error(error: impl std::fmt::Display) -> dioxus::prelude::ServerFnError {
    tracing::error!(error=%error,"Database operation failed");
    dioxus::prelude::ServerFnError::new("The request could not be completed. Please try again.")
}
