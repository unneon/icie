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
#[derive(Clone, Debug)]
pub enum MessageKind {
	Info,
	Warning,
	Error,
}
#[derive(Debug)]
pub struct MessageItem {
	pub title: String,
	pub is_close_affordance: Option<bool>,
	pub id: String,
}
#[derive(Debug)]
pub struct MessageItems {
	pub id: String,
	pub list: Vec<MessageItem>,
}
#[derive(Debug)]
pub struct Position {
	pub line: i64,
	pub character: i64,
}
