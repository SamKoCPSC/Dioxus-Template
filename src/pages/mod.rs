use crate::{auth::context::use_auth, Route};
use dioxus::prelude::*;
mod notes;
mod settings;
pub(crate) mod showcase;
mod workspace;
pub use notes::{EditNote, NewNote, Notes};
pub use settings::{Account, Accounts, Profile, Settings};
pub use showcase::ComponentShowcase;
pub use workspace::{Dashboard, Home, Search};
#[component]
pub fn Landing() -> Element {
    let auth = use_auth();
    rsx! {section{class:"hero",p{class:"eyebrow","A place to begin"}h1{"Make room for your next idea."}p{class:"lead","A simple home for your private notes. Save an idea, come back to it, and make it your own."}
     if auth.is_authenticated{Link{to:Route::Home{},class:"btn btn-primary","Go to my home"}}
     else{Link{to:Route::Login {},class:"btn btn-primary","Get started"}}
     Link { to: Route::About {}, class: "btn btn-secondary", "About {app::NAME}" }
    }div{class:"feature-grid",article{class:"card",h2{"Your space"}p{"Your notes belong to your account."}}article{class:"card",h2{"Pick up where you left off"}p{"Save and revisit your ideas whenever you need them."}}article{class:"card",h2{"Comfortable anywhere"}p{"A clear workspace on your phone or computer."}}}}
}
#[component]
pub fn Login() -> Element {
    let path = router().full_route_string();
    let redirect_uri = crate::utils::query::value(&path, "redirect_uri");
    let error = crate::utils::query::value(&path, "error");
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    let target = redirect_uri
        .as_deref()
        .and_then(crate::auth::redirect::normalize_same_origin_redirect)
        .unwrap_or_else(|| "/home".into());
    let encoded =
        percent_encoding::utf8_percent_encode(&target, percent_encoding::NON_ALPHANUMERIC)
            .to_string();
    let google = canonical.on_canonical_origin(&format!(
        "/auth/google/start?purpose=login&redirect_uri={encoded}"
    ));
    let github = canonical.on_canonical_origin(&format!(
        "/auth/github/start?purpose=login&redirect_uri={encoded}"
    ));
    rsx! {section{class:"card login-card",h1{"Welcome back"}p{"Choose how you’d like to sign in."}
     if error.is_some(){p{role:"alert",class:"error","Sign-in could not complete. If this email already belongs to an account, sign in with its existing provider and link the other provider in Accounts."}}
     div{class:"stack",a{href:google,class:"btn btn-primary","Continue with Google"}a{href:github,class:"btn btn-secondary","Continue with GitHub"}}
    }}
}
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    let _ = segments;
    rsx! {section{class:"card",h1{"Page not found"}p{"That page isn’t available."}Link{to:Route::Home{},class:"btn btn-primary","Go home"}}}
}

#[component]
pub fn About() -> Element {
    let auth = use_auth();
    rsx! {
        section { class: "hero about-hero",
            p { class: "eyebrow", "About {app::NAME}" }
            h1 { "A little space for what matters." }
            p { class: "lead", "Capture a thought, find it again, and pick up where you left off. A comfortable workspace for your everyday ideas." }
            div { class: "actions",
                if auth.is_authenticated {
                    Link { to: Route::Home {}, class: "btn btn-primary", "Open your workspace" }
                } else {
                    Link { to: Route::Login {}, class: "btn btn-primary", "Get started" }
                }
                Link { to: Route::Landing {}, class: "btn btn-secondary", "Back to the landing page" }
            }
        }
        section { class: "feature-grid", aria_label: "What you can do",
            article { class: "card", h2 { "Keep your ideas together" } p { "Write a note, save a useful detail, or make space for a new idea. Your workspace keeps it all in one place." } }
            article { class: "card", h2 { "Find your way back" } p { "Browse recent notes, search their titles and content, and reopen the one you need." } }
            article { class: "card", h2 { "Feel at home" } p { "Choose a light or dark appearance and manage your profile and sign-in methods from Settings." } }
        }
        section { class: "card about-privacy",
            h2 { "A private workspace" }
            p { "Your notes are visible to your account. Sign out when you finish on a shared device." }
        }
    }
}
