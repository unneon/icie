use std::path::PathBuf;

#[evscode::config(description = "Solution file stem")]
static SOLUTION_STEM: evscode::Config<String> = "main";

#[evscode::config(description = "Brut file stem")]
static BRUT_STEM: evscode::Config<String> = "brut";

#[evscode::config(description = "Test generator file stem")]
static GEN_STEM: evscode::Config<String> = "gen";

#[evscode::config(description = "C++ source extension")]
static CPP_EXTENSION: evscode::Config<String> = "cpp";

#[evscode::config(description = "Tests directory name")]
static TESTS_DIRECTORY: evscode::Config<String> = "tests";

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
	tests().join("user")
}
