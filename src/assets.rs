use crate::util::OS;
use evscode::webview::WebviewRef;
pub fn html_js_dynamic(webview:WebviewRef,source: &str) -> String {
    let path=webview.as_webview_uri(&evscode::asset(&("assets/".to_owned()+source)));
	format!("<script type=\"application/javascript\" src=\"{}\"></script>", path)
}

pub fn html_css_dynamic(source: &str) -> String {
	format!("<style>{}</style>", source)
}

pub fn html_material_icons(webview:WebviewRef) -> String {
	let mut woff2_asset = webview.as_webview_uri(&evscode::asset("assets/material-icons.woff2"));
	/*if let Ok(OS::Windows) = OS::query() {
		woff2_asset = evscode::asset("assets/material-icons.woff2");
		//woff2_asset = woff2_asset.replace('\\', "\\\\");
	}*/
	html_css_dynamic(&format!(
		r#"
			@font-face {{
				font-family: 'Material Icons';
				font-style: normal;
				font-weight: 400;
				src: url("{woff2_asset}") format('woff2');
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
		woff2_asset = woff2_asset
	))
}
