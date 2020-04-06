use crate::util::path::Path;
use itertools::Itertools;

#[derive(Eq, Ord, PartialEq, PartialOrd)]
enum OrderedWord {
	Text(String),
	Number(i64),
}

pub async fn scan_for_tests(test_dir: &str) -> Vec<Path> {
	let mut tests = scan_unordered(test_dir).await;
	tests.sort_by_key(compare_test_path);
	tests
}

async fn scan_unordered(test_dir: &str) -> Vec<Path> {
	vscode_sys::workspace::find_files(&format!("{}/**/*.in", test_dir))
		.await
		.into_iter()
		.map(|uri| Path::from_native(uri.fs_path()))
		.collect()
}

fn compare_test_path(path: &Path) -> Vec<OrderedWord> {
	path.as_str()
		.chars()
		.group_by(|c| c.is_numeric())
		.into_iter()
		.map(|(is_digit, group): (bool, _)| {
			let text = group.collect::<String>();
			let number = if is_digit { text.parse::<i64>().ok() } else { None };
			number.map(OrderedWord::Number).unwrap_or(OrderedWord::Text(text))
		})
		.collect()
}
