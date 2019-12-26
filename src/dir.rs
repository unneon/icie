use crate::util::path::Path;

/// File stem of the mail source file. This is the optimal solution, which should be tested, sent to
/// judging systems for scoring etc. For example, if this is set to "main", the source will be
/// called "main.cpp"(assuming icie.dir.cppExtension is set to "cpp").
#[evscode::config]
pub static SOLUTION_STEM: evscode::Config<String> = "main";

/// File stem of the brut source file. This is the a slow solution, which should not be sent to
/// judging systems, but can be used for checking outputs during discovery(stress testing). For
/// example, if this is set to "brut", the source will be called "brut.cpp"(assuming
/// icie.dir.cppExtension is set to "cpp").
#[evscode::config]
static BRUT_STEM: evscode::Config<String> = "brut";

/// File stem of the test generator source file. This is a program that will generate a random test
/// input and write it to stdout. Remember to initialize the random number generator with a
/// subsecond-precision clock, such as clock(3) or std::chrono::high_resolution_clock. For example,
/// if this is set to "gen", the source will be called "gen.cpp"(assuming icie.dir.cppExtension is
/// set to "cpp").
#[evscode::config]
static GEN_STEM: evscode::Config<String> = "gen";

/// File stem of the task checker source file. For tasks where there exist multiple correct answers,
/// this is the program which will be called to check if a given answer is correct. If the source
/// exists, the program will be called; otherwise, the answers will be checked for text equality. To
/// read the test case as well as your and a correct answer, you should declare main in a different
/// way than usual - `int main(int, char* argv[])` and open the test case files `ifstream
/// in(argv[1]), my(argv[2]), out(argv[3])`. After that, use `in`, `my` and `out` in the same way as
/// `cin`. If the answer is correct, the program should return a 0 exit code(e.g. normal return from
/// main). If the answer is not, is should return a non-zero exit code, e.g. by using `exit(1)`. A
/// good way to do so is with assertions, like `assert(index[i] <= n);`.
#[evscode::config]
static CHECKER_STEM: evscode::Config<String> = "checker";

/// The file extension used for sources written in the C++ language.
#[evscode::config]
pub static CPP_EXTENSION: evscode::Config<String> = "cpp";

/// The directory used for storing test cases. Usually, the directory will contain other
/// subdirectories with files called ID.in or ID.out. For example, if this is set to "tests", test
/// paths may look like tests/example/1.in or tests/user/3.out.
#[evscode::config]
pub static TESTS_DIRECTORY: evscode::Config<String> = "tests";

/// The subdirectory used for storing test cases entered by the user. See icie.dir.testsDirectory
/// configuration entry for details.
#[evscode::config]
static CUSTOM_TESTS_SUBDIRECTORY: evscode::Config<String> = "user";

/// The directory where new projects will be created by default. For example, with this set to
/// ~/Competitive, using Alt+F11 may create a ~/Competitive/rainbow-squirrel directory for the
/// project.
#[evscode::config]
pub static PROJECT_DIRECTORY: evscode::Config<Path> = "~";

pub fn solution() -> evscode::R<Path> {
	Ok(Path::from_native(evscode::workspace_root()?)
		.join(&*SOLUTION_STEM.get())
		.with_extension(&*CPP_EXTENSION.get()))
}

pub fn brut() -> evscode::R<Path> {
	Ok(Path::from_native(evscode::workspace_root()?)
		.join(&*BRUT_STEM.get())
		.with_extension(&*CPP_EXTENSION.get()))
}

pub fn gen() -> evscode::R<Path> {
	Ok(Path::from_native(evscode::workspace_root()?)
		.join(&*GEN_STEM.get())
		.with_extension(&*CPP_EXTENSION.get()))
}

pub fn checker() -> evscode::R<Path> {
	Ok(Path::from_native(evscode::workspace_root()?)
		.join(&*CHECKER_STEM.get())
		.with_extension(&*CPP_EXTENSION.get()))
}

pub fn tests() -> evscode::R<Path> {
	Ok(Path::from_native(evscode::workspace_root()?).join(&*TESTS_DIRECTORY.get()))
}

pub fn custom_tests() -> evscode::R<Path> {
	Ok(tests()?.join(&*CUSTOM_TESTS_SUBDIRECTORY.get()))
}
