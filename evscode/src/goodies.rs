//! Set of common utilities that are not present in original VS Code API.

pub mod stacked_status;
pub mod webview_resultmap;
pub mod webview_singleton;

/// Handle to a webview that exists in some sort of collection and may be used by multiple threads.
pub type WebviewHandle = std::sync::Arc<std::sync::Mutex<crate::Webview>>;

pub use stacked_status::StackedStatus;
pub use webview_resultmap::WebviewResultmap;
pub use webview_singleton::WebviewSingleton;
