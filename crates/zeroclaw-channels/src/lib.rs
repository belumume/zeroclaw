//! Channel implementations for messaging platform integrations.

pub mod util;

// Always-compiled channels and utilities
pub mod bluesky;
pub mod clawdtalk;
pub mod dingtalk;
pub mod discord;
pub mod discord_history;
pub mod imessage;
pub mod irc;
pub mod link_enricher;
pub mod linq;
pub mod mattermost;
pub mod mochat;
pub mod nextcloud_talk;
pub mod notion;
pub mod qq;
pub mod reddit;
pub mod signal;
pub mod slack;
pub mod transcription;
pub mod tts;
pub mod twitter;
pub mod voice_call;
pub mod wati;
pub mod webhook;
pub mod wecom;
pub mod whatsapp;

// Feature-gated channels
#[cfg(feature = "channel-email")]
pub mod email_channel;
#[cfg(feature = "channel-email")]
pub mod gmail_push;
#[cfg(feature = "channel-lark")]
pub mod lark;
#[cfg(feature = "channel-nostr")]
pub mod nostr;
#[cfg(feature = "channel-telegram")]
// telegram.rs stays in root (depends on crate::security::pairing)

#[cfg(feature = "voice-wake")]
pub mod voice_wake;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_storage;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_web;
