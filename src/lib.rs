//! App identity is independent of the stable Rust crate/binary name `app`.
pub const NAME: &str = env!("APP_NAME");
pub const SLUG: &str = env!("APP_SLUG");
