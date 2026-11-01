use crate::{auth::context::use_auth, Route};
use dioxus::prelude::*;
mod notes;
mod settings;
pub use notes::{EditNote, NewNote, Notes};
pub use settings::{Accounts, Profile};
#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    rsx! {section{class:"hero",p{class:"eyebrow","A place to begin"}h1{"Make room for your next idea."}p{class:"lead","A simple home for your private notes. Save an idea, come back to it, and make it your own."}
     if auth.is_authenticated{Link{to:Route::Notes{},class:"btn btn-primary","Open my notes"}}
     else{Link{to:Route::Login{redirect_uri:None,error:None},class:"btn btn-primary","Get started"}}
    }div{class:"feature-grid",article{class:"card",h2{"Your space"}p{"Your notes belong to your account."}}article{class:"card",h2{"Pick up where you left off"}p{"Save and revisit your ideas whenever you need them."}}article{class:"card",h2{"Comfortable anywhere"}p{"A clear workspace on your phone or computer."}}}}
}
#[component]
pub fn Login(redirect_uri: Option<String>, error: Option<String>) -> Element {
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    let target = redirect_uri
        .as_deref()
        .and_then(crate::auth::redirect::normalize_same_origin_redirect)
        .unwrap_or_else(|| "/notes".into());
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
