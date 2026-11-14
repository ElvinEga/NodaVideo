//! OpenCode server AgentProvider adapter for **NodaVideo**.
//!
//! Talks to a local `opencode serve` process over HTTP: model and variant
//! catalogs, session prompts, streamed events, and permission approvals.

pub mod provider;

pub use provider::OpenCodeAgentProvider;
