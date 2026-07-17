//! # wren-core
//!
//! Wren core (hexagonal architecture — see `docs/architecture/overview.md`):
//! domain, ports and use cases. **Dependency rule:** this crate does not
//! know about Groq, OpenAI, Tauri, cpal or the operating system. Everything
//! external implements the ports in `ports`.

pub mod composite;
pub mod domain;
pub mod ports;
pub mod usecase;

pub use composite::CompositeFeedback;
pub use domain::*;
pub use ports::*;
pub use usecase::{partial_transcript, DictationService, ToggleOutcome};
