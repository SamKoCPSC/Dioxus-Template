use super::base::SharedDrawer;
use crate::{auth::context::use_auth, Route};
use dioxus::prelude::*;

#[component]
pub fn AppNavigation() -> Element {
    let auth = use_auth();
    let theme = use_context::<crate::utils::theme::UseTheme>();
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    let mut open = use_signal(|| false);
    #[allow(unused_mut)]
    let mut ready = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    use_effect(move || ready.set(true));
    let path = router().full_route_string();
    let identity = (auth.is_authenticated, auth.current_user_id);
    // History/programmatic navigation and session changes dismiss old content.
    use_effect(use_reactive((&path, &identity), move |_| open.set(false)));
    let home = if auth.is_authenticated {
        Route::Home {}
    } else {
        Route::Landing {}
    };
    rsx! {
        header { class: "site-header",
            nav { class: "container nav", aria_label: "Primary navigation",
                Link { to: home.clone(), class: "brand", "{app::NAME}" }
                div { class: "nav-links",
                    if auth.is_authenticated {
                        for (to, label) in primary_links() { NavigationLink { to, label } }
                    }
                }
                div { class: "nav-actions",
                    button { r#type: "button", class: "btn btn-secondary header-theme", aria_label: "Toggle theme",
                        onclick: move |_| theme.toggle(), if theme.is_dark() { "Light" } else { "Dark" } }
                    if auth.is_authenticated {
                        form { method: "post", action: canonical.on_canonical_origin("/auth/logout?redirect_uri=/"),
                            button { r#type: "submit", class: "btn btn-secondary", "Sign out" } }
                    } else { Link { to: Route::Login {}, class: "btn btn-primary", "Sign in" } }
                    button { r#type: "button", class: "btn btn-secondary", aria_label: "Open navigation menu",
                        aria_haspopup: "dialog", aria_expanded: open(), disabled: !ready(),
                        onclick: move |_| open.set(true), span { aria_hidden: "true", "☰" } "Menu" }
                }
            }
        }
        if open() {
            SharedDrawer { title: "Navigation", on_dismiss: move |_| open.set(false),
                if let Some(profile) = auth.current_user.as_ref() {
                    p { class: "drawer-identity", "Signed in as " strong {
                        if profile.display_name.trim().is_empty() { "{profile.username}" } else { "{profile.display_name}" }
                    } }
                }
                nav { class: "drawer-links", aria_label: "All destinations",
                    NavigationLink { to: home, label: "Home", on_navigate: move |_| open.set(false) }
                    if auth.is_authenticated {
                        for (to, label) in primary_links().into_iter().skip(1) {
                            NavigationLink { to, label, on_navigate: move |_| open.set(false) }
                        }
                        NavigationLink { to: Route::NewNote {}, label: "New note", on_navigate: move |_| open.set(false) }
                        NavigationLink { to: Route::Account {}, label: "Account", on_navigate: move |_| open.set(false) }
                    }
                    NavigationLink { to: Route::About {}, label: "About", on_navigate: move |_| open.set(false) }
                }
                div { class: "drawer-actions",
                    if auth.is_authenticated {
                        // Keep the native form mounted until its POST completes.
                        form { method: "post", action: canonical.on_canonical_origin("/auth/logout?redirect_uri=/"),
                            button { r#type: "submit", class: "btn btn-secondary", "Sign out" } }
                    } else { Link { to: Route::Login {}, class: "btn btn-primary", onclick: move |_| open.set(false), "Sign in" } }
                    button { r#type: "button", class: "btn btn-secondary", aria_label: "Toggle navigation theme",
                        onclick: move |_| theme.toggle(), if theme.is_dark() { "Light appearance" } else { "Dark appearance" } }
                }
            }
        }
    }
}

fn primary_links() -> [(Route, &'static str); 5] {
    [
        (Route::Home {}, "Home"),
        (Route::Dashboard {}, "Dashboard"),
        (Route::Notes {}, "Notes"),
        (Route::Search {}, "Search"),
        (Route::Settings {}, "Settings"),
    ]
}

#[component]
fn NavigationLink(
    to: Route,
    label: String,
    #[props(default)] on_navigate: EventHandler<()>,
) -> Element {
    let current = use_route::<Route>();
    let exact = current == to;
    let selected = exact
        || (matches!(&to, Route::Notes {})
            && matches!(&current, Route::NewNote {} | Route::EditNote { .. }))
        || (matches!(&to, Route::Settings {})
            && matches!(
                &current,
                Route::Account {} | Route::Profile {} | Route::Accounts {}
            ));
    rsx! { Link { to, class: if selected { "is-active" } else { "" }, aria_current: exact.then_some("page"),
    onclick: move |_| on_navigate.call(()), "{label}" } }
}
