use evscode::{stdlib::state::Scope, State};

pub struct Skill {
	use_count: State<u64>,
	proficiency_threshold: u64,
}
impl Skill {
	pub const fn new(state_entry_name: &'static str, proficiency_threshold: u64) -> Skill {
		Skill { use_count: State::new(state_entry_name, Scope::Global), proficiency_threshold }
	}

	pub async fn is_proficient(&'static self) -> bool {
		self.use_count.get().unwrap().unwrap_or(0) >= self.proficiency_threshold
	}

	pub async fn add_use(&'static self) {
		let new_uses = self.use_count.get().unwrap().unwrap_or(0) + 1;
		self.use_count.set(&new_uses).await; // race condition, yay
	}
}
