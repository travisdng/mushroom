//! Search: indexing and retrieval over the notes.
//!
//! The index is derived from Markdown and rebuildable. Retrieval sits behind
//! a trait so semantic and hybrid search can be added later without touching
//! callers.

pub mod extract;
