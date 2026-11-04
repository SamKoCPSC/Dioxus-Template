#![allow(unused_imports)]
mod auth_required;
pub mod base;
mod navigation;
mod workspace;
use crate::auth::context::use_auth;
pub use auth_required::AuthRequired;
use dioxus::prelude::*;
pub use workspace::{NoteCards, PageHeading, SettingsNav};
#[component]
pub fn AppLayout() -> Element {
    #[cfg(target_arch = "wasm32")]
    let _viewport = use_hook(|| std::rc::Rc::new(crate::utils::mobile_viewport::mount()));
    let theme = crate::utils::theme::use_theme();
    provide_context(theme);
    rsx! {
     a { href:"#main",class:"skip-link","Skip to content" }
     navigation::AppNavigation {}
     main {id:"main",class:"container main-content",crate::auth::refresh::AuthRefresh{},crate::auth::session_page::SessionPage{} }
     footer {class:"container footer",aria_label:"Footer navigation", span { "{app::NAME}" } Link { to: crate::Route::About {}, "About" } crate::pages::showcase::ShowcaseLink {} }
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
