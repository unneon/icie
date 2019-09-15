use json::JsonValue;

pub enum Note {
	Save,
}

impl From<JsonValue> for Note {
	fn from(val: JsonValue) -> Note {
		match val["tag"].as_str().unwrap() {
			"discovery_save" => Note::Save,
			_ => panic!("unrecognized discover::comms::Note .tag"),
		}
	}
}
