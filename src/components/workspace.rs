use crate::{types::NotePreview, Route};
use dioxus::prelude::*;

#[component]
pub fn PageHeading(title: String, description: String, children: Element) -> Element {
    rsx! {
        div { class: "page-heading",
            div {
                p { class: "eyebrow", "Your workspace" }
                h1 { "{title}" }
                p { class: "page-description", "{description}" }
            }
            div { class: "actions", {children} }
        }
    }
}

#[component]
pub fn NoteCards(notes: Vec<NotePreview>) -> Element {
    rsx! {
        div { class: "notes-grid",
            for note in notes {
                Link { key: "{note.id}", to: Route::EditNote { id: note.id }, class: "card note-card",
                    h2 { "{note.title}" }
                    p { "{note.excerpt}" }
                    small { {format!("Updated {}", note.updated_at.format("%Y-%m-%d %H:%M UTC"))} }
                }
            }
        }
    }
}

#[component]
pub fn SettingsNav() -> Element {
    let current = use_route::<Route>();
    rsx! {
        nav { class: "settings-nav", aria_label: "Settings navigation",
            for (to, label) in [
                (Route::Settings {}, "Overview"),
                (Route::Account {}, "Account"),
                (Route::Profile {}, "Profile"),
                (Route::Accounts {}, "Sign-in methods"),
            ] {
                Link { key: "{label}", to: to.clone(), aria_current: (current == to).then_some("page"), "{label}" }
            }
        }
    }
}
