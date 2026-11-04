use dioxus::prelude::*;
mod api;
mod auth;
mod components;
#[cfg(feature = "server")]
mod config;
#[cfg(feature = "server")]
mod db;
#[cfg(feature = "server")]
mod middleware;
mod pages;
#[cfg(all(feature = "server", feature = "storage-s3"))]
mod storage;
#[cfg(all(feature = "server", test))]
mod test_utils;
mod types;
mod utils;
use components::AppLayout;
use pages::{
    About, Account, Accounts, ComponentShowcase, Dashboard, EditNote, Home, Landing, Login,
    NewNote, NotFound, Notes, Profile, Search, Settings,
};
#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    Landing {},
    #[route("/home")]
    Home {},
    #[route("/dashboard")]
    Dashboard {},
    #[route("/search")]
    Search {},
    #[route("/about")]
    About {},
    #[route("/dev/components")]
    ComponentShowcase {},
    #[route("/account")]
    Account {},
    #[route("/settings")]
    Settings {},
    #[route("/login")]
    Login {},
    #[route("/notes")]
    Notes {},
    #[route("/notes/new")]
    NewNote {},
    #[route("/notes/:id")]
    EditNote { id: uuid::Uuid },
    #[route("/settings/profile")]
    Profile {},
    #[route("/settings/accounts")]
    Accounts {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}
#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
#[cfg(feature = "server")]
fn main() {
    dioxus::logger::initialize_default();
    tokio::runtime::Runtime::new()
        .expect("server runtime")
        .block_on(async {
            let router = build_application_router().await;
            let address = dioxus::cli_config::fullstack_address_or_localhost();
            let listener = tokio::net::TcpListener::bind(address)
                .await
                .expect("bind application listener");
            tracing::info!(%address,"Application listening");
            serve_router_with_connect_info(listener, router, std::future::pending())
                .await
                .expect("serve application");
        });
}
#[cfg(feature = "server")]
async fn build_application_router() -> axum::Router {
    use dioxus::server::{DioxusRouterExt, ServeConfig};
    let config = config::RuntimeConfig::from_environment()
        .unwrap_or_else(|e| panic!("Startup rejected: {e}"));
    #[cfg(feature = "storage-s3")]
    let _object_store = storage::ObjectStore::from_environment()
        .await
        .expect("valid optional storage configuration");
    tracing::info!(mode=?config.mode(),host_only_cookie=config.cookie_domain().is_none(),"Validated application configuration");
    #[cfg(not(feature = "storage-s3"))]
    assert!(
        std::env::var("STORAGE_ENABLED").as_deref() != Ok("true"),
        "STORAGE_ENABLED=true requires a binary built with storage-s3"
    );
    db::init_pool().await;
    let pool = db::get_pool();
    let rate_limit =
        middleware::rate_limit::RateLimitState::from_environment().expect("valid client IP policy");
    let (google_client, github_client) = auth::oauth::build_oauth_clients(config.base_url());
    let state = auth::oauth::AppState {
        pool: pool.clone(),
        base_url_origin: url::Url::parse(config.base_url()).unwrap().origin(),
        auth_security_metrics: auth::oauth::AuthSecurityMetrics::default(),
        google_client,
        github_client,
        http_client: reqwest::Client::new(),
        google_userinfo_url: std::env::var("GOOGLE_USERINFO_URL").expect("GOOGLE_USERINFO_URL"),
        github_userinfo_url: std::env::var("GITHUB_USERINFO_URL").expect("GITHUB_USERINFO_URL"),
        github_emails_url: std::env::var("GITHUB_EMAILS_URL").expect("GITHUB_EMAILS_URL"),
        github_extra_scopes: auth::oauth::configured_github_extra_scopes(),
        rate_limit: rate_limit.clone(),
    };
    let application = axum::Router::new()
        .route("/healthz", axum::routing::get(|| async { "ok" }))
        .route(
            "/api/user_profile",
            axum::routing::get(auth::user_profile::handle_user_profile),
        )
        .with_state(auth::user_profile::UserProfileState { pool: pool.clone() })
        .serve_dioxus_application(ServeConfig::new(), App)
        .layer(axum::extract::DefaultBodyLimit::max(256 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            pool.clone(),
            middleware::auth::handle_auth,
        ))
        .layer(axum::middleware::from_fn_with_state(
            rate_limit.clone(),
            middleware::rate_limit::resolved_client_ip_middleware,
        ))
        .layer(axum::Extension(pool.clone()));
    // PostgreSQL extension-free maintenance. Expiry/revocation is checked on every request.
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limit.cleanup();
            for result in [
                db::cleanup_expired_auth_states(&pool).await,
                db::cleanup_expired_sessions(&pool, "24 hours").await,
            ] {
                if let Err(error) = result {
                    tracing::error!(%error,"Auth maintenance failed");
                }
            }
        }
    });
    build_oauth_router(state)
        .merge(application)
        .layer(axum::Extension(auth::local_origin::LocalCanonicalOrigin(
            config.local_loopback_origin(),
        )))
        .layer(axum::middleware::from_fn_with_state(
            config.clone(),
            middleware::local_origin::canonicalize_local_origin,
        ))
        .layer(axum::middleware::from_fn_with_state(
            config.local_loopback_origin().is_some(),
            middleware::development::guard_showcase,
        ))
        .layer(axum::middleware::from_fn_with_state(
            config.security_headers(),
            config::security_headers_middleware,
        ))
}
#[cfg(feature = "server")]
fn build_oauth_router(state: auth::oauth::AppState) -> axum::Router {
    axum::Router::new()
        .route(
            "/auth/{provider}/start",
            axum::routing::get(auth::oauth::start_handler),
        )
        .route(
            "/auth/{provider}/callback",
            axum::routing::get(auth::oauth::callback_route_handler),
        )
        .route(
            "/auth/logout",
            axum::routing::post(auth::logout::handle_logout),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .with_state(state)
}
#[cfg(feature = "server")]
async fn serve_router_with_connect_info<F>(
    listener: tokio::net::TcpListener,
    router: axum::Router,
    shutdown: F,
) -> std::io::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown)
    .await
}
#[component]
fn App() -> Element {
    let local_origin = use_server_cached(auth::local_origin::from_fullstack);
    provide_context(local_origin.clone());
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        if let (Some(canonical), Some(window)) = (&local_origin.0, web_sys::window()) {
            if let Ok(current) = window.location().href() {
                if let Some(target) =
                    auth::local_origin::canonical_local_navigation_target(canonical, &current)
                {
                    let _ = window.location().replace(&target);
                }
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let _ = &local_origin;
    });
    let initial = use_server_cached(auth::context::build_context_from_fullstack);
    let auth = use_signal(move || initial);
    provide_context(auth);
    rsx! {
     document::Title { "{app::NAME}" }
     document::Meta { name:"viewport",content:"width=device-width, initial-scale=1, viewport-fit=cover" }
     document::Link { rel:"stylesheet",href:asset!("/assets/app.css") }
     ErrorBoundary { handle_error:move|_|rsx!{section{class:"container card",h1{"Something went wrong"}p{"Reload this page to try again."}a{href:"/",class:"btn btn-primary","Go home"}}}, Router::<Route>{} }
    }
}

#[cfg(test)]
mod starter_route_tests {
    use super::Route;

    #[test]
    fn public_landing_and_private_home_have_distinct_routes() {
        assert_eq!("/".parse::<Route>().unwrap(), Route::Landing {});
        assert_eq!("/home".parse::<Route>().unwrap(), Route::Home {});
        assert_eq!("/dashboard".parse::<Route>().unwrap(), Route::Dashboard {});
        assert_eq!("/account".parse::<Route>().unwrap(), Route::Account {});
        assert_eq!("/settings".parse::<Route>().unwrap(), Route::Settings {});
        assert_eq!("/about".parse::<Route>().unwrap(), Route::About {});
    }

    #[test]
    fn search_route_matches_query_urls_without_decoding_their_contents() {
        assert_eq!(
            "/search?q=100%25_+%26+ideas".parse::<Route>().unwrap(),
            Route::Search {}
        );
    }
}
