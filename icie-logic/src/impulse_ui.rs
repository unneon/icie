use crate::Impulse;
use ci::{
	self, commands::{
		list_resources::Resource, multitest::{TestRow, TestRowSkipped}
	}, error::Error, strres::StrRes, testing::Outcome
};
use std::{
	path::{Path, PathBuf}, sync::mpsc::{self, Receiver, Sender, TryRecvError}, time::Duration
};

pub struct ImpulseCiUi {
	pub impulse: Sender<Impulse>,
	pub pause: Receiver<()>,
}

impl ci::ui::Ui for ImpulseCiUi {
	fn read_auth(&mut self, domain: &str) -> (String, String) {
		let (tx, rx) = mpsc::channel();
		self.impulse
			.send(Impulse::CiAuthRequest {
				domain: domain.to_owned(),
				channel: tx,
			})
			.unwrap();
		rx.recv().unwrap().unwrap()
	}

	fn track_progress(&mut self, verdict: &unijudge::Verdict, finish: bool) {
		self.impulse.send(Impulse::CiTrack { verdict: verdict.clone(), finish }).unwrap();
	}

	fn submit_success(&mut self, id: String) {
		self.impulse.send(Impulse::CiSubmitSuccess { id }).unwrap();
	}

	fn test_list(&mut self, paths: &[PathBuf]) {
		self.impulse
			.send(Impulse::CiTestList {
				paths: paths.iter().cloned().collect(),
			})
			.unwrap();
	}

	fn print_resource_list(&mut self, _resources: &[Resource]) {
		unimplemented!()
	}

	fn print_resource(&mut self, _data: &[u8]) {
		unimplemented!()
	}

	fn print_test(&mut self, outcome: &Outcome, timing: Option<Duration>, in_path: &Path, output: Option<StrRes>) {
		self.impulse
			.send(Impulse::CiTestSingle {
				outcome: outcome.clone(),
				timing,
				in_path: in_path.to_owned(),
				output: output.map(|sr| sr.get_string().expect("internal conversion StrRes -> String failed")),
			})
			.unwrap();
	}

	fn print_finish_test(&mut self, success: bool) {
		self.impulse.send(Impulse::CiTestFinish { success }).unwrap();
	}

	fn print_finish_init(&mut self) {
		self.impulse.send(Impulse::CiInitFinish).unwrap();
	}

	fn print_transpiled(&mut self, _compiled: &str) {
		unimplemented!()
	}

	fn print_found_test(&mut self, _test_str: &str) {
		unimplemented!()
	}

	fn print_error(&mut self, _error: Error) {
		unimplemented!()
	}

	fn multitest_row_skipped(&mut self, _number: TestRowSkipped) {
		unimplemented!()
	}

	fn multitest_row(&mut self, row: TestRow) {
		self.impulse
			.send(Impulse::CiMultitestRow {
				number: row.number,
				input: row.input.get_string().unwrap(),
				brut_measure: (*row.brut_measure).clone(),
				measures: row.measures.iter().cloned().collect(),
				fitness: row.fitness,
			})
			.unwrap();
		loop {
			match self.pause.try_recv() {
				Ok(()) => self.pause.recv().unwrap(),
				Err(TryRecvError::Empty) => break,
				Err(e) => Err(e).expect("discovery failed when still running tests"),
			}
		}
	}

	fn multitest_finish(&mut self, _input: Option<String>) {
		self.impulse.send(Impulse::CiMultitestFinish).unwrap();
	}

	fn warn(&mut self, _message: &str) {
		unimplemented!()
	}

	fn notice(&mut self, _message: &str) {}
}

pub struct PausableUi {
	pub ui: ImpulseCiUi,
	pub pause: Sender<()>,
}
