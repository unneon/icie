use crate::util::{fs, path::Path, OS};

/// Internal path for dynamically loading assets during development. Set to ICIE source directory if you are developing
/// ICIE to enable hot reload.
#[evscode::config]
static DYNAMIC_ASSET_PATH: evscode::Config<Option<String>> = None;

pub async fn html_js(path: &str) -> String {
	match dynamic_asset(path).await {
		Some(asset) => html_js_dynamic(&asset),
		None => html_js_static(path),
	}
}

pub fn html_js_static(path: &str) -> String {
	format!("<script src=\"{}\"></script>", evscode::asset(path))
}

fn html_js_dynamic(source: &str) -> String {
	format!("<script>{}</script>", source)
}

pub async fn html_css(path: &str) -> String {
	match dynamic_asset(path).await {
		Some(asset) => html_css_dynamic(&asset),
		None => html_css_static(path),
	}
}

fn html_css_static(path: &str) -> String {
	html_css_static_raw(&evscode::asset(path))
}

fn html_css_static_raw(url: &str) -> String {
	format!("<link rel=\"stylesheet\" href=\"{}\">", url)
}

fn html_css_dynamic(source: &str) -> String {
	format!("<style>{}</style>", source)
}

pub fn html_material_icons() -> String {
	match OS::query() {
		// For whatever reason, bundled icons do not display on Windows.
		// I made sure the paths are correct and fully-backslashed, but to no avail.
		Ok(OS::Windows) => material_icons_cloud(),
		_ => material_icons_bundled(),
	}
}

fn material_icons_cloud() -> String {
	html_css_static_raw("https://fonts.googleapis.com/icon?family=Material+Icons")
}

fn material_icons_bundled() -> String {
	html_css_dynamic(&format!(
		r#"
			@font-face {{
				font-family: 'Material Icons';
				font-style: normal;
				font-weight: 400;
				src: url({woff2_asset}) format('woff2');
			}}

			.material-icons {{
				font-family: 'Material Icons';
				font-weight: normal;
				font-style: normal;
				font-size: 24px;
				line-height: 1;
				letter-spacing: normal;
				text-transform: none;
				display: inline-block;
				white-space: nowrap;
				word-wrap: normal;
				direction: ltr;
				-webkit-font-feature-settings: 'liga';
				-webkit-font-smoothing: antialiased;
			}}
	"#,
		woff2_asset = evscode::asset("assets/material-icons.woff2")
	))
}

async fn dynamic_asset(path: &str) -> Option<String> {
	let assets = Path::from_native(DYNAMIC_ASSET_PATH.get()?);
	let source = fs::read_to_string(&assets.join(path)).await.ok()?;
	Some(source)
}
