use crate::{
	telemetry::TELEMETRY, util::{fs, path::Path, workspace_root}
};
use evscode::{error::Severity, R};
use futures::StreamExt;
use js_sys::Math::random;
use serde::Deserialize;

pub async fn report_html(message: &str, crashy_html: &str) -> R<()> {
	let id = random_id();
	let page_path = save_crashy_html(&id, crashy_html).await?;
	let title = "ICIE Report 'website could not be analyzed'";
	let webview_meta = evscode::Webview::new("icie.report.html", title, 1).enable_scripts().create();
	let webview = webview_meta.webview;
	webview.set_html(&render_report_html(message, &id));
	let mut stream = evscode::error::cancel_on(webview_meta.listener, webview_meta.disposer);
	TELEMETRY.report.spark();
	while let Some(note) = stream.next().await {
		let note: Note = note?.into_serde().unwrap();
		match note {
			Note::Tutorial => {
				TELEMETRY.report_tutorial.spark();
				// JS already opens the link.
			},
			Note::OpenPage => {
				TELEMETRY.report_page.spark();
				if let Err(e) = evscode::open_external(&format!("file://{}", page_path)).await {
					crate::logger::on_error(e.severity(Severity::Warning)).await;
				}
			},
		}
	}
	Ok(())
}

fn random_id() -> String {
	let random_part: String = random().to_string().chars().filter(|c| c.is_digit(10)).collect();
	format!("WCNBA-{}", random_part)
}

async fn save_crashy_html(id: &str, crashy_html: &str) -> R<Path> {
	let dir = workspace_root()?.join(id);
	let path = dir.join("page.html");
	fs::create_dir_all(&dir).await?;
	fs::write(&path, crashy_html).await?;
	Ok(path)
}

fn render_report_html(message: &str, id: &str) -> String {
	format!(
		r##"
		<html>
			<head>
				<script>{js}</script>
			</head>
			<body>
				<h1>Bug report {id}, 'website could not be analyzed'</h1>
				<h2>
					Read
					<a
						href="https://github.com/pustaczek/icie/blob/master/docs/WEBSITE_COULD_NOT_BE_ANALYZED.md"
						onclick="action_tutorial()"
					>
						how to report 'website could not be analyzed' errors tutorial
					</a>
					.
				</h2>
				Open the <a href="#" onclick="action_open_page()">page where the extension failed</a>. (read the tutorial to see what to do after that)<br/>
				Your error message: {message}
			</body>
		</html>
	"##,
		js = include_str!("report/script.js"),
		id = id,
		message = message,
	)
}

#[derive(Deserialize)]
#[serde(tag = "tag")]
enum Note {
	#[serde(rename = "report_tutorial")]
	Tutorial,
	#[serde(rename = "report_open_page")]
	OpenPage,
}
