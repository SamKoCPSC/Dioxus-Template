use crate::{
    api::workspace::{workspace_overview, MAX_QUERY_CHARS},
    auth::context::use_auth,
    components::{AuthRequired, NoteCards, PageHeading},
    Route,
};
use dioxus::prelude::*;

#[component]
pub fn Home() -> Element {
    let auth = use_auth();
    let name = auth
        .current_user
        .map(|p| {
            if p.display_name.trim().is_empty() {
                p.username
            } else {
                p.display_name
            }
        })
        .unwrap_or_else(|| "there".into());
    let mut overview = use_resource(workspace_overview);
    rsx! {
        AuthRequired {
            PageHeading { title: "Home", description: "Good to see you, {name}.",
                Link { to: Route::NewNote {}, class: "btn btn-primary", "New note" }
            }
            section { class: "quick-links", aria_label: "Quick links",
                Link { to: Route::Dashboard {}, class: "card quick-link", span { class: "eyebrow", "At a glance" } h2 { "Your dashboard" } p { "See your workspace and recent updates." } }
                Link { to: Route::Search {}, class: "card quick-link", span { class: "eyebrow", "Find an idea" } h2 { "Search your notes" } p { "Find something by its title or content." } }
                Link { to: Route::Account {}, class: "card quick-link", span { class: "eyebrow", "Make it yours" } h2 { "Your account" } p { "Review your profile and sign-in settings." } }
            }
            section { class: "page-section", aria_label: "Recent notes",
                div { class: "section-heading", h2 { "Pick up where you left off" } Link { to: Route::Notes {}, "View all notes" } }
                match overview.read().as_ref() {
                    None => rsx! { p { role: "status", "Loading your workspace…" } },
                    Some(Err(_)) => rsx! { p { role: "alert", "We couldn’t load your recent notes." } button { class: "btn btn-secondary", onclick: move |_| overview.restart(), "Retry" } },
                    Some(Ok(data)) if data.recent_notes.is_empty() => rsx! { div { class: "card empty-state", h2 { "Start with one small idea" } p { "Your recent notes will appear here. Create your first note whenever you’re ready." } Link { to: Route::NewNote {}, class: "btn btn-primary", "Create your first note" } } },
                    Some(Ok(data)) => rsx! { NoteCards { notes: data.recent_notes.iter().take(3).cloned().collect() } },
                }
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    let mut overview = use_resource(workspace_overview);
    rsx! {
        AuthRequired {
            PageHeading { title: "Dashboard", description: "A snapshot of your notes and recent updates.",
                button { r#type: "button", class: "btn btn-secondary", onclick: move |_| overview.restart(), "Refresh" }
                Link { to: Route::NewNote {}, class: "btn btn-primary", "New note" }
            }
            match overview.read().as_ref() {
                None => rsx! { p { role: "status", "Loading your dashboard…" } },
                Some(Err(_)) => rsx! { div { class: "card", p { role: "alert", "We couldn’t load your dashboard. Please try again." } button { class: "btn btn-secondary", onclick: move |_| overview.restart(), "Retry" } } },
                Some(Ok(data)) => rsx! {
                    dl { class: "stats-grid", aria_label: "Workspace statistics",
                        div { class: "card stat-card", dt { "Total notes" } dd { "data-testid": "total-notes", "{data.note_count}" } }
                        div { class: "card stat-card", dt { "Updated in the last 7 days" } dd { "{data.updated_last_seven_days}" } }
                        div { class: "card stat-card", dt { "Last saved" } dd { class: "stat-date",
                            if let Some(updated) = data.last_updated_at { time { datetime: updated.to_rfc3339(), {updated.format("%d %b %Y").to_string()} } } else { "No notes yet" }
                        } }
                    }
                    section { class: "card dashboard-search", aria_label: "Find a note", h2 { "Looking for something?" } SearchForm { query: String::new() } }
                    section { class: "page-section", aria_label: "Recently updated notes",
                        div { class: "section-heading", h2 { "Recently updated" } Link { to: Route::Notes {}, "View all notes" } }
                        if data.recent_notes.is_empty() {
                            div { class: "card empty-state", h2 { "Your workspace is ready" } p { "Create a note to see your first update here." } Link { to: Route::NewNote {}, class: "btn btn-primary", "Create a note" } }
                        } else { NoteCards { notes: data.recent_notes.clone() } }
                    }
                },
            }
        }
    }
}

#[component]
fn SearchForm(query: String) -> Element {
    let initial = query.clone();
    let mut draft = use_signal(move || initial);
    use_effect(use_reactive(&query, move |query| draft.set(query)));
    // Native GET navigation preserves a shareable/back-button-friendly query URL.
    rsx! {
        form { method: "get", action: "/search", class: "search-form",
            div { class: "search-field",
                label { r#for: "note-search", "Search notes" }
                input { id: "note-search", name: "q", r#type: "search", value: draft(), maxlength: MAX_QUERY_CHARS as i64,
                    placeholder: "Words from a title or note", oninput: move |event| draft.set(event.value()) }
            }
            button { r#type: "submit", class: "btn btn-primary", "Search notes" }
        }
    }
}

#[component]
pub fn Search() -> Element {
    // Dioxus 0.7.9 decodes a typed query before splitting its fields, losing
    // literal ampersands. Read the raw history URL and decode each field once.
    let query = crate::utils::query::value(&router().full_route_string(), "q").unwrap_or_default();
    let mut results = use_resource(use_reactive(&query, |query| async move {
        if query.trim().is_empty() {
            Ok(crate::types::NoteSearchResults::default())
        } else {
            crate::api::workspace::search_notes(query).await
        }
    }));
    rsx! {
        AuthRequired {
            PageHeading { title: "Search", description: "Find a note by its title or content.",
                Link { to: Route::NewNote {}, class: "btn btn-secondary", "New note" }
            }
            section { class: "card search-panel", SearchForm { query: query.clone() }
                p { class: "muted search-help", "Search is private to your account. Use up to {MAX_QUERY_CHARS} characters." }
            }
            section { class: "page-section", aria_label: "Search results", aria_live: "polite",
                if query.trim().is_empty() {
                    div { class: "card empty-state", h2 { "What would you like to find?" } p { "Enter words from a note’s title or content to get started." } }
                } else {
                    div { class: "section-heading", h2 { "Results" } Link { to: Route::Search {}, "Clear search" } }
                    match results.read().as_ref() {
                        None => rsx! { p { role: "status", "Searching your notes…" } },
                        Some(Err(_)) => rsx! { div { class: "card", p { role: "alert", "We couldn’t search your notes. Use up to 120 characters, check your sign-in, and try again." } button { class: "btn btn-secondary", onclick: move |_| results.restart(), "Retry" } } },
                        Some(Ok(data)) if data.notes.is_empty() => rsx! { div { class: "card empty-state", h2 { "No notes found" } p { "Try different words or browse all your notes." } Link { to: Route::Notes {}, class: "btn btn-secondary", "Browse notes" } } },
                        Some(Ok(data)) => rsx! {
                            p { class: "muted", if data.has_more { "Showing the first 50 matches. Refine your search to find more." } else if data.notes.len() == 1 { "1 result" } else { "{data.notes.len()} results" } }
                            NoteCards { notes: data.notes.clone() }
                        },
                    }
                }
            }
        }
    }
}
