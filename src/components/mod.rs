#![allow(unused_imports)]
mod auth_required;
pub mod base;
mod workspace;
use crate::auth::context::use_auth;
pub use auth_required::AuthRequired;
use dioxus::prelude::*;
pub use workspace::{NoteCards, PageHeading, SettingsNav};
#[component]
pub fn AppLayout() -> Element {
    #[cfg(target_arch = "wasm32")]
    let _viewport = use_hook(|| std::rc::Rc::new(crate::utils::mobile_viewport::mount()));
    let auth = use_auth();
    let theme = crate::utils::theme::use_theme();
    provide_context(theme);
    let canonical =
        try_use_context::<crate::auth::local_origin::LocalCanonicalOrigin>().unwrap_or_default();
    rsx! {
     a { href:"#main",class:"skip-link","Skip to content" }
     header { class:"site-header",nav { class:"container nav",aria_label:"Primary navigation",
      Link { to:if auth.is_authenticated {crate::Route::Home{}} else {crate::Route::Landing{}},class:"brand","{app::NAME}" }
      div {class:"nav-links",
       if auth.is_authenticated {
        NavigationLink { to: crate::Route::Home {}, label: "Home" }
        NavigationLink { to: crate::Route::Dashboard {}, label: "Dashboard" }
        NavigationLink { to: crate::Route::Notes {}, label: "Notes" }
        NavigationLink { to: crate::Route::Search {}, label: "Search" }
        NavigationLink { to: crate::Route::Settings {}, label: "Settings" }
       }
      }
      div {class:"nav-actions",
       button { r#type:"button",class:"btn btn-secondary",aria_label:"Toggle theme",onclick:move|_|theme.toggle(),if theme.is_dark(){"Light"}else{"Dark"} }
       if auth.is_authenticated {
        form { method:"post",action:canonical.on_canonical_origin("/auth/logout?redirect_uri=/"),button {r#type:"submit",class:"btn btn-secondary","Sign out"} }
       } else {Link{to:crate::Route::Login {},class:"btn btn-primary","Sign in"}}
      }
     }}
     main {id:"main",class:"container main-content",crate::auth::refresh::AuthRefresh{},crate::auth::session_page::SessionPage{} }
     footer {class:"container footer",aria_label:"Footer navigation", span { "{app::NAME}" } Link { to: crate::Route::About {}, "About" } }
    }
}

#[component]
fn NavigationLink(to: crate::Route, label: String) -> Element {
    let current = use_route::<crate::Route>();
    let exact = current == to;
    let selected = exact
        || (matches!(&to, crate::Route::Notes {})
            && matches!(
                &current,
                crate::Route::NewNote {} | crate::Route::EditNote { .. }
            ))
        || (matches!(&to, crate::Route::Settings {})
            && matches!(
                &current,
                crate::Route::Account {} | crate::Route::Profile {} | crate::Route::Accounts {}
            ));
    rsx! { Link { to, class: if selected { "is-active" } else { "" }, aria_current: exact.then_some("page"), "{label}" } }
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
