#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize)]
pub enum Codegen {
	Debug,
	Release,
	Profile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, evscode::Configurable)]
pub enum Standard {
	#[evscode(name = "C++03")]
	Cpp03,
	#[evscode(name = "C++11")]
	Cpp11,
	#[evscode(name = "C++14")]
	Cpp14,
	#[evscode(name = "C++17")]
	Cpp17,
	#[evscode(name = "C++20")]
	FutureCpp20,
}

impl Codegen {
	pub const LIST: &'static [Codegen] = &[Codegen::Debug, Codegen::Release, Codegen::Profile];

	pub fn flags_clang(self) -> &'static [&'static str] {
		match &self {
			Codegen::Debug => &["-g", "-D_GLIBCXX_DEBUG", "-fno-sanitize-recover=undefined", "-fsanitize=undefined"],
			Codegen::Release => &["-Ofast"],
			Codegen::Profile => &["-g", "-O2", "-fno-inline-functions"],
		}
	}
}

impl Standard {
	pub fn flag_clang(self) -> &'static str {
		match self {
			Standard::Cpp03 => "-std=c++03",
			Standard::Cpp11 => "-std=c++11",
			Standard::Cpp14 => "-std=c++14",
			Standard::Cpp17 => "-std=c++17",
			Standard::FutureCpp20 => "-std=c++2a",
		}
	}
}
