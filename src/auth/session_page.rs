//! Keep read views scoped to the current identity without discarding editor work.
use crate::auth::{account_binding::use_form_account, context::use_auth};
use crate::Route;
use dioxus::prelude::*;

fn retains_local_work(route: &Route) -> bool {
    matches!(
        route,
        Route::NewNote {} | Route::EditNote { .. } | Route::Profile {} | Route::Accounts {}
    )
}

/// Section links change focus/scroll position, not the form being edited.
/// Ignore fragments and the empty query delimiter SSR can retain during
/// hydration, but keep nonempty queries (such as a collection parent) distinct.
fn retained_form_visit_key(path: &str) -> String {
    let before_fragment = path
        .split_once('#')
        .map_or(path, |(before_fragment, _)| before_fragment);
    match before_fragment.split_once('?') {
        Some((path, "")) => path,
        _ => before_fragment,
    }
    .to_string()
}

#[component]
pub fn SessionPage() -> Element {
    let auth = use_auth();
    let route = use_route::<Route>();
    let path = router().full_route_string();
    let identity = auth
        .is_authenticated
        .then_some(auth.current_user_id)
        .flatten();
    if retains_local_work(&route) {
        let visit_key = retained_form_visit_key(&path);
        rsx! {
            for visit in [path] {
                RetainedFormPage { key: "form-{visit_key}", path: visit, Outlet::<Route> {} }
            }
        }
    } else {
        // Cancel old resources and remount every nested private/read-only view
        // together with its profile links. Profile-only updates do not reset it.
        rsx! { ReadOnlySessionScope { identity, Outlet::<Route> {} } }
    }
}

#[component]
fn ReadOnlySessionScope(identity: Option<uuid::Uuid>, children: Element) -> Element {
    // Dioxus keys reset scopes in a keyed fragment, not on a single static
    // component slot. The one-item list is intentional lifecycle ownership.
    rsx! {
        for account in [identity] {
            AccountReadContents { key: "account-{account:?}", children: children.clone() }
        }
    }
}

#[component]
fn AccountReadContents(children: Element) -> Element {
    rsx! { {children} }
}

#[component]
fn RetainedFormPage(path: String, children: Element) -> Element {
    let binding = use_form_account();
    let auth = use_auth();
    let identity = auth
        .is_authenticated
        .then_some(auth.current_user_id)
        .flatten();
    let blocked = binding().is_some() && binding() != identity;
    #[cfg(target_arch = "wasm32")]
    {
        let mut previously_blocked = use_signal(|| false);
        use_effect(use_reactive(&blocked, move |blocked| {
            crate::components::base::set_form_modals_suspended("session-retained-page", blocked);
            use wasm_bindgen::JsCast;
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                if blocked {
                    // Suspend native focus traps before focusing the visible notice.
                    if let Some(heading) = document
                        .get_element_by_id("session-page-notice")
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = heading.focus();
                    }
                } else if *previously_blocked.peek()
                    && document
                        .query_selector("dialog[open]")
                        .ok()
                        .flatten()
                        .is_none()
                {
                    if let Some(page) = document
                        .get_element_by_id("session-retained-page")
                        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
                    {
                        let _ = page.focus();
                    }
                }
            }
            previously_blocked.set(blocked);
        }));
    }
    rsx! {
        if blocked {
            section { class: "container", aria_label: "Session changed",
                padding_top: "var(--space-lg)", padding_bottom: "var(--space-lg)",
                div { class: "neumo-raised", padding: "var(--space-lg)", border_radius: "var(--radius-lg)", overflow_wrap: "anywhere",
                    h1 { id: "session-page-notice", tabindex: "-1", "Your session has changed" }
                    p { role: "status", "Sign in with the same account in another tab, then return here to continue. Keep this tab open to preserve your changes." }
                    a { href: path, target: "_blank", rel: "noopener noreferrer", class: "btn btn-primary touch-target", "Open a new tab" }
                }
            }
        }
        // Keep the component and owned blobs alive. Hidden/inert also prevents
        // keyboard access; CSS explicitly hides top-layer dialogs/popovers.
        // Dioxus's client interpreter does not remove inert=false. Omit the
        // attribute completely when enabled, including after account recovery.
        div { id: "session-retained-page", hidden: blocked, inert: blocked.then_some(true), tabindex: "-1", {children} }
    }
}
