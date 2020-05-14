use crate::util::path::Path;

#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Word<'a> {
	Text(&'a str),
	Number(i64),
}

pub async fn scan_for_tests(test_dir: &str) -> Vec<Path> {
	let mut tests = scan_unordered(test_dir).await;
	tests.sort_by(|a, b| compare_test_path(a).cmp(&compare_test_path(b)));
	tests
}

async fn scan_unordered(test_dir: &str) -> Vec<Path> {
	vscode_sys::workspace::find_files(&format!("{}/**/*.in", test_dir))
		.await
		.into_iter()
		.map(|uri| Path::from_native(uri.fs_path()))
		.collect()
}

fn compare_test_path(raw_path: &Path) -> Vec<Word> {
	let mut path = raw_path.as_str();
	let mut words = Vec::new();
	while !path.is_empty() {
		let end_of_number = path.find(|c: char| !c.is_digit(10)).unwrap_or_else(|| path.len());
		let end_of_word = path.find(|c: char| c.is_digit(10)).unwrap_or_else(|| path.len());
		let word = match path[..end_of_number].parse() {
			Ok(number) => Word::Number(number),
			Err(_) => Word::Text(&path[..end_of_number.max(end_of_word)]),
		};
		path = &path[end_of_number.max(end_of_word)..];
		words.push(word);
	}
	words
}
