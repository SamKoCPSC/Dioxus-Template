//! Reusable primitives; the sample need not render every exported component.
#![allow(dead_code)]
pub use action_menu::*;
pub use button::*;
pub use card::*;
pub use empty_state::*;
pub use feedback::*;
pub use input::*;
pub use loading_spinner::*;
pub use shared_drawer::*;
pub use shared_modal::*;
mod action_menu;
mod button;
mod card;
mod empty_state;
mod feedback;
mod input;
mod loading_spinner;
#[cfg(any(target_arch = "wasm32", test))]
mod modal_lifecycle;
mod shared_drawer;
mod shared_modal;
#[cfg(target_arch = "wasm32")]
pub(crate) use modal_lifecycle::browser::set_form_modals_suspended;
#[cfg(target_arch = "wasm32")]
pub(crate) mod account_menu;
#[cfg(test)]
mod modal_lifecycle_tests;
#[cfg(target_arch = "wasm32")]
mod overlay_dom;
#[cfg(any(target_arch = "wasm32", test))]
mod overlay_logic;
#[cfg(test)]
mod overlay_logic_tests;
#[cfg(target_arch = "wasm32")]
mod tooltip_lifecycle;

pub(crate) mod autocomplete_popup;
