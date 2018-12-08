#[derive(Clone, Debug)]
pub struct QuickPickItem {
	pub id: String,
	pub label: String,
	pub description: Option<String>,
	pub detail: Option<String>,
}
#[derive(Clone, Debug)]
pub struct InputBoxOptions {
	pub prompt: Option<String>,
	pub placeholder: Option<String>,
	pub password: bool,
	pub ignore_focus_out: bool,
}
