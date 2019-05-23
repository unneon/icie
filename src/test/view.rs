pub mod manage;
pub mod render;

#[evscode::config(description = "Auto-scroll to first failed test")]
static SCROLL_TO_FIRST_FAILED: evscode::Config<bool> = true;
