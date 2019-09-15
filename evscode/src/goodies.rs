//! Set of common utilities that are not present in original VS Code API.

pub mod multistatus;
pub mod webview_collection;

pub use multistatus::MultiStatus;
pub use webview_collection::Collection;
