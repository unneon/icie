use std::path::PathBuf;

#[evscode::config(
	description = "File stem of the mail source file. This is the optimal solution, which should be tested, sent to judging systems for scoring etc. For example, if this is set \
	               to \"main\", the source will be called \"main.cpp\"(assuming icie.dir.cppExtension is set to \"cpp\")."
)]
pub static SOLUTION_STEM: evscode::Config<String> = "main";

#[evscode::config(
	description = "File stem of the brut source file. This is the a slow solution, which should not be sent to judging systems, but can be used for checking outputs during \
	               discovery(stress testing). For example, if this is set to \"brut\", the source will be called \"brut.cpp\"(assuming icie.dir.cppExtension is set to \"cpp\")."
)]
static BRUT_STEM: evscode::Config<String> = "brut";

#[evscode::config(
	description = "File stem of the test generator source file. This is a program that will generate a random test input and write it to stdout. Remember to initialize the \
	               random number generator with a subsecond-precision clock, such as clock(3) or std::chrono::high_resolution_clock. For example, if this is set to \"gen\", the \
	               source will be called \"gen.cpp\"(assuming icie.dir.cppExtension is set to \"cpp\")."
)]
static GEN_STEM: evscode::Config<String> = "gen";

#[evscode::config(description = "The file extension used for sources written in the C++ language.")]
pub static CPP_EXTENSION: evscode::Config<String> = "cpp";

#[evscode::config(
	description = "The directory used for storing test cases. Usually, the directory will contain other subdirectories with files called <test id>.in or <test id>.out. For \
	               example, if this is set to \"tests\", test paths may look like tests/example/1.in or tests/user/3.out."
)]
static TESTS_DIRECTORY: evscode::Config<String> = "tests";

#[evscode::config(
	description = "The subdirectory used for storing test cases entered by the user. See icie.dir.testsDirectory configuration entry for details."
)]
static CUSTOM_TESTS_SUBDIRECTORY: evscode::Config<String> = "user";

#[evscode::config(
	description = "The directory where new projects will be created by default. For example, with this set to ~/Competitive, using Alt+F11 may create a \
	               ~/Competitive/rainbow-squirrel directory for the project."
)]
pub static PROJECT_DIRECTORY: evscode::Config<PathBuf> = "~";

pub fn solution() -> evscode::R<PathBuf> {
	Ok(evscode::workspace_root()?.join(&*SOLUTION_STEM.get()).with_extension(&*CPP_EXTENSION.get()))
}

pub fn brut() -> evscode::R<PathBuf> {
	Ok(evscode::workspace_root()?.join(&*BRUT_STEM.get()).with_extension(&*CPP_EXTENSION.get()))
}

pub fn gen() -> evscode::R<PathBuf> {
	Ok(evscode::workspace_root()?.join(&*GEN_STEM.get()).with_extension(&*CPP_EXTENSION.get()))
}

pub fn tests() -> evscode::R<PathBuf> {
	Ok(evscode::workspace_root()?.join(&*TESTS_DIRECTORY.get()))
}

pub fn custom_tests() -> evscode::R<PathBuf> {
	Ok(tests()?.join(&*CUSTOM_TESTS_SUBDIRECTORY.get()))
}

pub fn random_adjective() -> &'static str {
	use rand::seq::SliceRandom;
	let mut rng = rand::thread_rng();
	static ADJECTIVES: &[&str] = &[
		"playful",
		"shining",
		"sparkling",
		"rainbow",
		"kawaii",
		"superb",
		"amazing",
		"glowing",
		"blessed",
		"smiling",
		"exquisite",
		"cuddly",
		"caramel",
		"serene",
		"sublime",
		"beaming",
		"graceful",
		"plushy",
		"heavenly",
		"marshmallow",
	];
	ADJECTIVES.choose(&mut rng).unwrap()
}

pub fn random_animal() -> &'static str {
	use rand::seq::SliceRandom;
	let mut rng = rand::thread_rng();
	static ANIMALS: &[&str] = &[
		"capybara", "squirrel", "spider", "anteater", "hamster", "whale", "eagle", "zebra", "dolphin", "hedgehog", "penguin", "wombat", "ladybug",
		"platypus", "squid", "koala", "panda",
	];
	ANIMALS.choose(&mut rng).unwrap()
}
