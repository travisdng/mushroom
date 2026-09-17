//! Notes: the Markdown files that are the application's reason to exist.
//!
//! Markdown on disk is the source of truth. Nothing here may lose or rewrite
//! what the user typed — see `.kiro/steering/data-integrity.md`.

pub mod cache;
pub mod frontmatter;
pub mod model;
pub mod paths;
pub mod selfwrites;
pub mod service;
pub mod store;
pub mod watcher;
