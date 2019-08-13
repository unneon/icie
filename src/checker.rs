use crate::{
	build, ci::{self, task::Checker}, dir
};
use evscode::R;
use std::time::Duration;

/// The maximum time a checker executable can run before getting killed, specified in milliseconds. Killing will cause the test to be classified as failed. Leaving this empty(which denotes no limit) is not recommended, because this will cause stuck processes to run indefinitely, wasting system resources.
#[evscode::config]
static TIME_LIMIT: evscode::Config<Option<u64>> = Some(1500);

pub fn get_checker() -> R<Box<dyn Checker+Send>> {
	let checker = dir::checker()?;
	Ok(if !checker.exists() {
		Box::new(ci::task::FreeWhitespaceChecker)
	} else {
		let environment = ci::exec::Environment { time_limit: (*TIME_LIMIT.get()).map(Duration::from_millis) };
		Box::new(ci::task::ExecChecker { executable: build::build(checker, &ci::cpp::Codegen::Release, false)?, environment })
	})
}
