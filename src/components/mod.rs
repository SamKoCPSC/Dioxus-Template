#![allow(unused_imports)]
mod auth_required;
pub mod base;
use crate::auth::context::use_auth;
pub use auth_required::AuthRequired;
use dioxus::prelude::*;
#[component]
pub fn AppLayout() -> Element {
    #[cfg(target_arch = "wasm32")]
    let _viewport = use_hook(|| std::rc::Rc::new(crate::utils::mobile_viewport::mount()));
    let auth = use_auth();
    let theme = crate::utils::theme::use_theme();
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    rsx! {
     a { href:"#main",class:"skip-link","Skip to content" }
     header { class:"site-header",nav { class:"container nav",aria_label:"Primary navigation",
      Link { to:crate::Route::Home{},class:"brand","{app::NAME}" }
      div {class:"nav-links",
       if auth.is_authenticated {
        Link { to:crate::Route::Notes{},"Notes" }
        Link { to:crate::Route::Profile{},"Profile" }
        Link { to:crate::Route::Accounts{},"Accounts" }
       }
      }
      div {class:"nav-actions",
       button { r#type:"button",class:"btn btn-secondary",aria_label:"Toggle theme",onclick:move|_|theme.toggle(),if theme.is_dark(){"Light"}else{"Dark"} }
       if auth.is_authenticated {
        form { method:"post",action:canonical.on_canonical_origin("/auth/logout?redirect_uri=/"),button {r#type:"submit",class:"btn btn-secondary","Sign out"} }
       } else {Link{to:crate::Route::Login{redirect_uri:None,error:None},class:"btn btn-primary","Sign in"}}
      }
     }}
     main {id:"main",class:"container main-content",crate::auth::refresh::AuthRefresh{},crate::auth::session_page::SessionPage{} }
     footer {class:"container footer",aria_label:"Footer navigation", "{app::NAME}" }
    }
}

/// Client-interactive forms must not perform a native GET before hydration.
/// Keep native OAuth/logout forms separate: those deliberately work without WASM.
#[component]
pub fn ClientForm(on_submit: EventHandler<FormEvent>, children: Element) -> Element {
    #[allow(unused_mut)]
    let mut ready = use_signal(|| false);
    #[cfg(target_arch = "wasm32")]
    use_effect(move || ready.set(true));
    rsx! {form{onsubmit:move|event|{event.prevent_default();if ready(){on_submit.call(event);}},fieldset{disabled:!ready(),{children}}}}
}
#[cfg(test)]
mod tests {
    use super::*;
    #[component]
    fn TestForm() -> Element {
        rsx! {ClientForm{on_submit:move|_|{},input{r#type:"text"}button{r#type:"submit","Save"}}}
    }
    #[test]
    fn server_rendered_client_form_is_inert_until_hydration() {
        let mut dom = VirtualDom::new(TestForm);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("<form"));
        assert!(html.contains("<fieldset disabled"), "{html}");
    }
}
