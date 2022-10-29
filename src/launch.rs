use crate::{
	assets, dir, logger, manifest::Manifest, net::{interpret_url, require_task,Session}, open, util::{self, fs, workspace_root,sleep,set_workspace_root}
};
use futures::FutureExt;
use evscode::{error::ResultExt, quick_pick, webview::WebviewMeta, QuickPick, E, R};
use vscode_sys::{window};
use futures::StreamExt;
use serde::Serialize;
use std::cmp::min;
use crate::util::time_now;
use unijudge::{Backend, Resource, Statement,
	boxed::{BoxedContest, BoxedTask},ErrorCode
};
use std::convert::TryInto;
//use wasm_timer;
//use wasm_timer::Instant;
use core::time::Duration;

pub async fn activate() -> R<()> {
	let _status = crate::STATUS.push("Launching");
	logger::initialize()?;
	evscode::spawn(crate::newsletter::check());
	if open::contest::is_contest().await?==true{
		let _status = crate::STATUS.push("Launching from contest");
		let sub_dirs=fs::find_a_dir(&workspace_root()?).await?;
		set_workspace_root(sub_dirs.as_str());
		util::listener(); 
    }
    layout_setup().await?;
	open::contest::check_for_manifest().await?;
	Ok(())
}

pub async fn deactivate() -> R<()> {
	// Deactivate does not really seem to work at all? Issue [47881][1] suggest that also most JS
	// APIs break when this happens.
	// [1]: https://github.com/Microsoft/vscode/issues/47881
	Ok(())
}
pub fn format_sec(secs:i64)->String{
	let hrs=secs/3600;
	let min=secs%3600/60;
	let sec=secs%60;
	if hrs>0
	{format!("Remaining {} : {:0>2} : {:0>2} s",hrs,min,sec)}
	else if min>0
	{format!("Remaining {} : {:0>2} s",min,sec)}
	else 
	{format!("Remaining {} s",sec)}
}
pub async fn layout_setup() -> R<()> {
    let _status = crate::STATUS.push("Opening");
	if let Ok(manifest) = Manifest::load().await {
        place_cursor_in_code().await;
		if manifest.statement.is_some() {
			statement().await?;
		}
		// Refocus the cursor, because apparently preserve_focus is useless.
		place_cursor_in_code().await;
        	
		let url = manifest.req_task_url()?;
        let (url, backend) = interpret_url(url)?;
		let url = require_task::<BoxedContest, BoxedTask>(url)?;
        let Resource::Task(task) = url.resource;
		let sess = Session::connect(&url.domain, backend).await?;
        
		let rem_t= sess.run(|backend, sess| backend.remain_time(sess, &task)).await;
        
		//let to_end:String;
		let statusBarItem=window::create_status_bar_item();
        
        statusBarItem.show();
		match rem_t {
			Ok(to_end) =>{
				let deadline = time_now() + Duration::from_secs(to_end.try_into().unwrap());
				
                statusBarItem.set_text(&format_sec(to_end));
				evscode::spawn(async move {
					while let Ok(left) = deadline.duration_since(time_now()) {
						
						let delay = min(left, Duration::from_secs(1));
						sleep(delay).await;
						statusBarItem.set_text(&format_sec(left.as_secs().try_into().unwrap()));
						
					}
					statusBarItem.set_text("Contest Ended");
					return Ok(());
				});
				
			},
			Err(err) =>{
                //match err.0 {
                //     ErrorCode::NotYetStarted =>  {statusBarItem.set_text("Contest Not Started");},
                     //ErrorCode::Ended_Already =>  {
                         //statusBarItem.set_text("Contest Ended");
                        //},
                //     _                          => {statusBarItem.set_text("Contest Fetch Error");},
                //}
            }
		}			

	}
	
	Ok(())
}

async fn place_cursor_in_code() {
	if let Ok(solution) = dir::solution() {
		let _ = util::open_source(&solution).await;
	}
}

async fn display_pdf(mut webview: WebviewMeta, pdf: &[u8]) {
	let _status = crate::STATUS.push("Rendering PDF");
	webview.webview.set_html(&format!(
		"<html><head>{}{}</head><body id=\"body\" style=\"padding: 0;\"></body></html>",
		assets::html_js_dynamic("pdf-2.2.228.min.js"),
		assets::html_js_dynamic("pdf.js"),
	));
	// This webview script sends a message indicating that it is ready to receive messages. See
	// [`evscode::Webview::post_message`] docs.
	webview.listener.next().await;
	let _ = webview.webview.post_message(StatementData { pdf_data_base64: pdf }).await;
}

#[derive(Serialize)]
struct StatementData<'a> {
	pdf_data_base64: &'a [u8],
}

#[evscode::command(title = "ICIE Statement", key = "alt+8")]
async fn statement() -> R<()> {
	let manifest = Manifest::load().await?;
	let statement = manifest.req_statement()?;
	let webview = evscode::Webview::new("icie.statement", "ICIE Statement", 2)
		.enable_scripts()
		.enable_find_widget()
		.retain_context_when_hidden()
		.preserve_focus()
		.create();
	match statement {
		Statement::HTML { html } => webview.webview.set_html(html),
		Statement::PDF { pdf } => {
			let pdf = pdf.clone();
			evscode::spawn(async move {
				display_pdf(webview, &pdf).await;
				Ok(())
			})
		},
	}
	Ok(())
}

#[evscode::command(title = "ICIE Launch nearby", key = "alt+backspace")]
async fn nearby() -> R<()> {
	let parent = workspace_root()?.parent();
	let mut nearby =
		fs::read_dir(&parent).await?.into_iter().map(|path| (path.fmt_relative(&parent), path)).collect::<Vec<_>>();
	nearby.sort_by_key(|nearby| nearby.0.clone());
	let path = QuickPick::new()
		.items(nearby.into_iter().map(|nearby| quick_pick::Item::new(nearby.1, nearby.0)))
		.show()
		.await
		.ok_or_else(E::cancel)?;
	evscode::open_folder(path.as_str(), false).await;
	Ok(())
}

#[evscode::command(title = "ICIE Web Task")]
async fn web_task() -> R<()> {
	let manifest = Manifest::load().await?;
	evscode::open_external(manifest.req_task_url()?).await?;
	Ok(())
}

#[evscode::command(title = "ICIE Web Contest")]
async fn web_contest() -> R<()> {
	let manifest = Manifest::load().await?;
	let (url, backend) = interpret_url(manifest.req_task_url()?)?;
	let Resource::Task(task) = require_task(url)?.resource;
	let contest = backend.backend.task_contest(&task).wrap("task is not attached to any contest")?;
	let url = backend.backend.contest_url(&contest);
	evscode::open_external(&url).await?;
	Ok(())
}
