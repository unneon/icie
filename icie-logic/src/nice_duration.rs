use std::{fmt, time::Duration};

pub fn serialize<S: serde::Serializer>(t: &Option<Duration>, s: S) -> Result<S::Ok, S::Error> {
	if let Some(t) = t {
		if t.subsec_nanos() == 0 {
			s.serialize_str(&format!("{}s", t.as_secs()))
		} else if t.subsec_nanos() == t.subsec_millis() * 1000000 {
			s.serialize_str(&format!("{}ms", t.as_millis()))
		} else {
			s.serialize_str(&format!("{}ns", t.as_nanos()))
		}
	} else {
		s.serialize_none()
	}
}

pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Duration>, D::Error> {
	d.deserialize_option(OptDurDesVis)
}

struct OptDurDesVis;
impl<'de> serde::de::Visitor<'de> for OptDurDesVis {
	type Value = Option<Duration>;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("\"2s\" or \"750ms\"")
	}

	fn visit_none<E>(self) -> Result<Self::Value, E> {
		Ok(None)
	}

	fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Ok(Some(deserializer.deserialize_str(DurDesVis)?))
	}
}

struct DurDesVis;
impl<'de> serde::de::Visitor<'de> for DurDesVis {
	type Value = Duration;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("\"2s\" or \"750ms\"")
	}

	fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<Self::Value, E> {
		let make_e = || E::invalid_value(serde::de::Unexpected::Str(s), &self);
		let sufstart = s.find(|c: char| !c.is_digit(10)).ok_or_else(make_e)?;
		let n = s[..sufstart].parse().map_err(|_| make_e())?;
		let suf = &s[sufstart..];
		Ok(match suf {
			"h" => Duration::from_secs(n * 60 * 60),
			"min" => Duration::from_secs(n * 60),
			"s" => Duration::from_secs(n),
			"ms" => Duration::from_millis(n),
			"ns" => Duration::from_nanos(n),
			_ => Err(make_e())?,
		})
	}
}
