//! Reusable side panel extracted from Noms' navigation drawer. Mount while open.
use super::SharedModal;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Default)]
pub enum DrawerSide {
    #[default]
    Start,
    End,
}

/// The caller owns open state and content. The shared native-dialog lifecycle
/// traps focus, blocks the background, locks scrolling and restores the trigger.
/// Close, Escape and backdrop invoke on_dismiss; clicking content does not.
#[component]
pub fn SharedDrawer(
    title: String,
    #[props(default)] side: DrawerSide,
    on_dismiss: EventHandler<()>,
    children: Element,
) -> Element {
    let id = use_hook(|| format!("drawer-{}", uuid::Uuid::new_v4()));
    let title_id = format!("{id}-title");
    let close_id = format!("{id}-close");
    let side_class = match side {
        DrawerSide::Start => "drawer-overlay--start",
        DrawerSide::End => "drawer-overlay--end",
    };
    rsx! {
        SharedModal {
            aria_labelledby: title_id.clone(),
            focus_selector: format!("#{close_id}"),
            class: "drawer-overlay {side_class}",
            on_dismiss,
            section { class: "drawer-panel", onclick: move |event| event.stop_propagation(),
                div { class: "drawer-header",
                    h2 { id: title_id, "{title}" }
                    button { id: close_id, r#type: "button", class: "btn btn-secondary drawer-close",
                        aria_label: "Close drawer", onclick: move |_| on_dismiss.call(()),
                        span { aria_hidden: "true", "✕" }
                    }
                }
                div { class: "drawer-content", {children} }
            }
        }
    }
}
