//! Set of common utilities that are not present in original VS Code API.

pub mod dev_tools_logger;
pub mod multistatus;
pub mod webview_collection;

pub use dev_tools_logger::DevToolsLogger;
pub use multistatus::MultiStatus;
pub use webview_collection::Collection;
