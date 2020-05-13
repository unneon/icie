use crate::util::SourceTarget;
use std::time::Duration;

pub fn time(t: &Duration) -> String {
	let s = t.as_secs();
	let ms = t.as_millis() % 1000;
	format!("{}.{:03}s", s, ms)
}

pub fn time_left(mut t: Duration) -> String {
	let mut s = {
		let x = t.as_secs() % 60;
		t -= Duration::from_secs(x);
		format!("{} left", plural(x as usize, "second", "seconds"))
	};
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 % 60;
		t -= Duration::from_secs(x * 60);
		s = format!("{}, {}", plural(x as usize, "minute", "minutes"), s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 % 24;
		t -= Duration::from_secs(x * 60 * 60);
		s = format!("{}, {}", plural(x as usize, "hour", "hours"), s);
	}
	if t.as_secs() > 0 {
		let x = t.as_secs() / 60 / 60 / 24;
		t -= Duration::from_secs(x * 60 * 60 * 24);
		s = format!("{}, {}", plural(x as usize, "day", "days"), s)
	}
	s
}

pub fn verb_on_source(verb: &'static str, source: &SourceTarget) -> String {
	match source {
		SourceTarget::Custom(source) => format!("{} {}", verb, source.fmt_workspace()),
		SourceTarget::Main => verb.to_owned(),
		SourceTarget::BruteForce => format!("{} brute force", verb),
		SourceTarget::TestGenerator => format!("{} test generator", verb),
	}
}

pub fn plural(x: usize, singular: &str, plural: &str) -> String {
	format!("{} {}", x, if x == 1 { singular } else { plural })
}

pub fn list(xs: &[&str]) -> String {
	match xs {
		[] => "...".to_owned(),
		[only] => (*only).to_owned(),
		[head @ .., tail] => format!("{} and {}", head.join(", "), tail),
	}
}
