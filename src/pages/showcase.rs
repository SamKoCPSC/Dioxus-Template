use dioxus::prelude::*;

pub(crate) fn available(local: bool) -> bool {
    cfg!(debug_assertions) && local
}

#[component]
pub fn ComponentShowcase() -> Element {
    let local = use_context::<crate::auth::local_origin::LocalCanonicalOrigin>();
    if !available(local.0.is_some()) {
        return rsx! { super::NotFound { segments: vec![] } };
    }
    #[cfg(debug_assertions)]
    {
        rsx! { ShowcaseContents {} }
    }
    #[cfg(not(debug_assertions))]
    {
        rsx! { super::NotFound { segments: vec![] } }
    }
}

#[component]
pub fn ShowcaseLink() -> Element {
    let local = use_context::<crate::auth::local_origin::LocalCanonicalOrigin>();
    if available(local.0.is_some()) {
        rsx! { Link { to: crate::Route::ComponentShowcase {}, "Components" } }
    } else {
        rsx! {}
    }
}

#[cfg(debug_assertions)]
#[component]
fn ShowcaseContents() -> Element {
    use crate::components::base::*;
    let theme = use_context::<crate::utils::theme::UseTheme>();
    let mut name = use_signal(|| "Project notes".to_owned());
    let mut message = use_signal(|| None::<Result<String, String>>);
    let mut generation = use_signal(|| 0_u64);
    let mut dialog = use_signal(|| false);
    let mut drawer = use_signal(|| None::<DrawerSide>);
    let mut notify = move |text: &str| {
        generation += 1;
        message.set(Some(Ok(text.to_owned())));
    };
    rsx! {
        section { class: "showcase-intro",
            p { class: "eyebrow", "Local development" }
            h1 { "Component showcase" }
            p { class: "lead", "Explore the building blocks and try their interactions. These examples stay in this page and do not save account data." }
            button { r#type: "button", class: "btn btn-secondary", onclick: move |_| theme.toggle(),
                if theme.is_dark() { "Preview light theme" } else { "Preview dark theme" }
            }
        }
        nav { class: "showcase-index", aria_label: "Component sections",
            a { href: "#foundations", "Foundations" } a { href: "#buttons", "Buttons" }
            a { href: "#inputs", "Inputs & cards" } a { href: "#states", "States" } a { href: "#overlays", "Dialogs & drawers" }
        }
        section { id: "foundations", class: "page-section", aria_label: "Foundations",
            h2 { "Foundations" }
            p { class: "muted", "Colors follow the same theme tokens as the app." }
            div { class: "showcase-swatches",
                for (label, token) in [("Background", "--bg"), ("Surface", "--surface"), ("Accent", "--accent"), ("Text", "--text"), ("Danger", "--danger")] {
                    div { class: "card showcase-swatch", span { class: "swatch-color", style: "background:var({token})", aria_hidden: "true" }
                        strong { "{label}" } code { "{token}" }
                    }
                }
            }
        }
        section { id: "buttons", class: "page-section", aria_label: "Buttons",
            h2 { "Buttons" }
            Card { div { class: "actions",
                Button { onclick: move |_| notify("Primary example selected."), "Primary action" }
                Button { variant: ButtonVariant::Secondary, onclick: move |_| notify("Secondary example selected."), "Secondary action" }
                Button { variant: ButtonVariant::Ghost, onclick: move |_| notify("Ghost example selected."), "Ghost action" }
                Button { variant: ButtonVariant::Danger, onclick: move |_| notify("Danger example selected. No data was deleted."), "Danger action" }
                Button { disabled: true, onclick: move |_| {}, "Disabled action" }
            } }
        }
        section { id: "inputs", class: "page-section", aria_label: "Inputs and cards",
            h2 { "Inputs & cards" }
            div { class: "showcase-columns",
                Card { h3 { "Controlled input" } p { class: "muted", "Change the title to update the preview." }
                    Input { accessible_name: "Example project name", value: name(), placeholder: "Project name", oninput: move |value| name.set(value) }
                }
                Card { h3 { "{name}" } p { "A reusable card with the app’s spacing, border and surface colors." } }
            }
        }
        section { id: "states", class: "page-section", aria_label: "Loading, empty and feedback states",
            h2 { "Loading, empty & feedback" }
            div { class: "showcase-columns",
                Card { h3 { "Loading" } div { class: "showcase-loading", LoadingSpinner {} span { "Loading your workspace…" } }
                    p { class: "muted", "A static preview of the loading state." }
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| { generation += 1; message.set(Some(Err("Example error: try that action again.".to_owned()))); }, "Show error notification" }
                }
                Card { EmptyState { icon: rsx! { span { aria_hidden: "true", "✎" } }, title: "No notes yet", description: "An empty state can explain what comes next.",
                    action: rsx! { Button { onclick: move |_| notify("Empty-state action selected."), "Try empty-state action" } }
                } }
            }
        }
        section { id: "overlays", class: "page-section", aria_label: "Dialogs and drawers",
            h2 { "Dialogs & drawers" }
            Card { p { "Try close controls, Escape, backdrop clicks and keyboard navigation. Focus returns to the button that opened the panel." }
                div { class: "actions",
                    Button { onclick: move |_| dialog.set(true), "Open example dialog" }
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| drawer.set(Some(DrawerSide::Start)), "Open start drawer" }
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| drawer.set(Some(DrawerSide::End)), "Open end drawer" }
                }
            }
        }
        FeedbackToast { message: message(), generation: generation(), on_dismiss: move |_| message.set(None) }
        if dialog() {
            SharedModal { aria_labelledby: "showcase-dialog-title", focus_selector: "#showcase-dialog-cancel", on_dismiss: move |_| dialog.set(false),
                div { class: "modal-card", onclick: move |event| event.stop_propagation(),
                    h2 { id: "showcase-dialog-title", "Example dialog" }
                    p { "A small confirmation using the shared modal component." }
                    div { class: "actions",
                        button { id: "showcase-dialog-cancel", class: "btn btn-secondary", onclick: move |_| dialog.set(false), "Cancel example" }
                        Button { onclick: move |_| { dialog.set(false); notify("Example confirmed."); }, "Confirm example" }
                    }
                }
            }
        }
        if let Some(side) = drawer() {
            SharedDrawer { title: "Example drawer", side, on_dismiss: move |_| drawer.set(None),
                p { "A side panel with independently scrolling content." }
                Input { accessible_name: "Drawer example input", value: name(), oninput: move |value| name.set(value) }
                p { class: "muted page-section", "This input shares the card preview’s state. Closing the panel preserves the value." }
            }
        }
    }
}
