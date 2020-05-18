use crate::terminal::Terminal;
use async_trait::async_trait;
use evscode::R;

pub struct Fallback<A, B>(pub A, pub B);

#[async_trait(?Send)]
impl<A: Terminal, B: Terminal> Terminal for Fallback<A, B> {
	async fn spawn(&self, title: &str, command: &[&str]) -> R<()> {
		match self.0.spawn(title, command).await {
			Ok(()) => Ok(()),
			Err(_) => self.1.spawn(title, command).await,
		}
	}
}
