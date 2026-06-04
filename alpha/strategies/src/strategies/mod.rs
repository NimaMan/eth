//! Concrete strategies.
//!
//! Each strategy is a named config preset (plus a factory producing complete
//! resolved specs) composing the [`crate::core`] engine. Strategies add no
//! trait-level logic; they only choose engine config.

pub mod alpha11;
pub mod diagnostics;
