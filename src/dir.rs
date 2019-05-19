use std::path::PathBuf;

#[evscode::config(description = "Solution file stem")]
pub static SOLUTION_STEM: evscode::Config<String> = "main";

#[evscode::config(description = "Brut file stem")]
static BRUT_STEM: evscode::Config<String> = "brut";

#[evscode::config(description = "Test generator file stem")]
static GEN_STEM: evscode::Config<String> = "gen";

#[evscode::config(description = "C++ source extension")]
pub static CPP_EXTENSION: evscode::Config<String> = "cpp";

#[evscode::config(description = "Tests directory name")]
static TESTS_DIRECTORY: evscode::Config<String> = "tests";

#[evscode::config(description = "Custom test subdirectory")]
static CUSTOM_TESTS_SUBDIRECTORY: evscode::Config<String> = "user";

#[evscode::config(description = "Project directory")]
pub static PROJECT_DIRECTORY: evscode::Config<PathBuf> = "~";

pub fn solution() -> PathBuf {
	evscode::workspace_root().join(&*SOLUTION_STEM.get()).with_extension(&*CPP_EXTENSION.get())
}

pub fn brut() -> PathBuf {
	evscode::workspace_root().join(&*BRUT_STEM.get()).with_extension(&*CPP_EXTENSION.get())
}

pub fn gen() -> PathBuf {
	evscode::workspace_root().join(&*GEN_STEM.get()).with_extension(&*CPP_EXTENSION.get())
}

pub fn tests() -> PathBuf {
	evscode::workspace_root().join(&*TESTS_DIRECTORY.get())
}

pub fn custom_tests() -> PathBuf {
	tests().join(&*CUSTOM_TESTS_SUBDIRECTORY.get())
}

pub fn random_codename() -> String {
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
	static ANIMALS: &[&str] = &[
		"capybara", "squirrel", "spider", "anteater", "hamster", "whale", "eagle", "zebra", "dolphin", "hedgehog", "penguin", "wombat", "ladybug", "platypus", "squid", "koala",
		"panda",
	];
	format!("{}-{}", ADJECTIVES.choose(&mut rng).unwrap(), ANIMALS.choose(&mut rng).unwrap())
}
