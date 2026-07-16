//! Fixture-backed compatibility tests for the supported OpenAI client.

mod support;

#[path = "openai_clients/openai_client_catalog.rs"]
mod catalog;
#[path = "openai_clients/openai_client_chat.rs"]
mod chat;
#[path = "openai_clients/openai_client_completions.rs"]
mod completions;
