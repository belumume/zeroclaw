//! Core trait definitions and shared message types for ZeroClaw.
//!
//! This crate contains the fundamental abstractions that all ZeroClaw subsystems
//! depend on: the `Channel`, `Provider`, `Tool` traits, along with the message
//! types they exchange (`ChannelMessage`, `SendMessage`, `ChatMessage`,
//! `ToolResult`, etc.).
//!
//! Extracting these into a leaf crate with minimal dependencies ensures that
//! changing a channel implementation, tool, or provider doesn't trigger
//! recompilation of the trait definitions themselves.

pub mod agent;
pub mod channel;
pub mod media;
pub mod provider;
pub mod schema;
pub mod tool;

tokio::task_local! {
    /// Current thread/sender ID for per-sender rate limiting.
    /// Set by the agent loop, read by SecurityPolicy.
    pub static TOOL_LOOP_THREAD_ID: Option<String>;

    /// Override for tool choice mode, set by the agent loop.
    /// Read by providers that support native tool calling.
    pub static TOOL_CHOICE_OVERRIDE: Option<String>;
}
