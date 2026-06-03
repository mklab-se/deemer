//! Core types and configuration for `deemer`.
//!
//! This crate holds the framework-agnostic pieces of the tool — configuration
//! and error types — so they can be reused by the CLI binary, tests, or any
//! future crate (e.g. a `-client`) without pulling in `clap`, `tokio`, or other
//! CLI-only dependencies.
//!
//! deemer's domain logic — test-run models, AI judgement, and verdicts — lives here.

pub mod check;
pub mod config;
pub mod error;
pub mod expr;
pub mod judge;
pub mod results;
pub mod suite;
pub mod template;
pub mod value;

pub use error::{Error, Result};
