use crate::ci;
use json::JsonValue;

pub enum Note {
	Start,
	Pause,
	Reset,
	Save { input: String },
}

impl From<JsonValue> for Note {
	fn from(val: JsonValue) -> Note {
		match val["tag"].as_str().unwrap() {
			"discovery_start" => Note::Start,
			"discovery_pause" => Note::Pause,
			"discovery_reset" => Note::Reset,
			"discovery_save" => Note::Save { input: String::from(val["input"].as_str().unwrap()) },
			_ => panic!("unrecognized discover::comms::Note .tag"),
		}
	}
}

pub enum Food {
	State { running: bool, reset: bool },
	Row { number: usize, outcome: ci::test::Verdict, fitness: i64, input: Option<String> },
}

impl From<Food> for JsonValue {
	fn from(food: Food) -> JsonValue {
		match food {
			Food::State { running, reset } => json::object! {
				"tag" => "discovery_state",
				"running" => running,
				"reset" => reset
			},
			Food::Row { number, outcome, fitness, input } => json::object! {
				"tag" => "discovery_row",
				"number" => number,
				"outcome" => match outcome {
					ci::test::Verdict::Accepted { .. } => "accept",
					ci::test::Verdict::WrongAnswer => "wrong_answer",
					ci::test::Verdict::RuntimeError => "runtime_error",
					ci::test::Verdict::TimeLimitExceeded => "time_limit_exceeded",
					ci::test::Verdict::IgnoredNoOut => "ignored_no_out",
				},
				"fitness" => fitness,
				"input" => input,
			},
		}
	}
}
