/// LSP (Language Server Protocol) module for hornet2
///
/// This module provides LSP server functionality for Arazzo specification files,
/// enabling real-time validation, hover information, go-to-definition, and code completion
/// in code editors.
pub mod completion;
pub mod definition;
pub mod diagnostic;
pub mod document;
pub mod hover;
pub mod server;
pub mod workspace;

// Re-export public API
pub use server::ArazzoLanguageServer;
