use crate::util::path::Path;
use itertools::Itertools;
use std::cmp::Ordering;

pub async fn scan_and_order(test_dir: &str) -> Vec<Path> {
	let mut tests = scan(test_dir).await;
	order(&mut tests);
	tests
}

async fn scan(test_dir: &str) -> Vec<Path> {
	vscode_sys::workspace::find_files(&format!("{}/**/*.in", test_dir))
		.await
		.into_iter()
		.map(|uri| Path::from_native(uri.fs_path()))
		.collect()
}

fn order(tests: &mut Vec<Path>) {
	tests.sort_by(comp_by_test_number);
}

fn comp_by_test_number(lhs: &Path, rhs: &Path) -> Ordering {
	let lgroups = lhs.to_str().unwrap().chars().group_by(|c| c.is_numeric());
	let rgroups = rhs.to_str().unwrap().chars().group_by(|c| c.is_numeric());
	for ((isdig, lgrp), (_, rgrp)) in lgroups.into_iter().zip(rgroups.into_iter()) {
		let grp_compr = if isdig {
			let lnum: i64 = lgrp.collect::<String>().parse().unwrap();
			let rnum: i64 = rgrp.collect::<String>().parse().unwrap();
			lnum.cmp(&rnum)
		} else {
			lgrp.cmp(rgrp)
		};
		if grp_compr != Ordering::Equal {
			return grp_compr;
		}
	}
	lhs.to_str().unwrap().len().cmp(&rhs.to_str().unwrap().len())
}
