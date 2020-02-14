//! Set of common utilities that are not present in original VS Code API.

pub mod logger;
pub mod multistatus;
pub mod webview_collection;

pub use logger::DevToolsLogger;
pub use multistatus::MultiStatus;
pub use webview_collection::Collection;
