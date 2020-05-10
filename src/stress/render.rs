use crate::assets;

pub async fn render() -> String {
	format!(
		r#"
		<html>
			<head>
				{css}
				{material_icons}
				{js}
			</head>
			<body>
				<div class="container">
					<table class="log">
						<thead>
							<tr>
								<th>Test</th>
								<th>Verdict</th>
								<th>Fitness</th>
							</tr>
						</thead>
						<tbody id="log-body">
							<tr id="current">
								<td>1</td>
								<td></td>
								<td></td>
							</tr>
						</tbody>
					</table>
				</div>
				<br/>
				<div id="best-test-container" class="data">
					<div class="actions">
						<a class="action material-icons" onclick="action_save()">add</a>
					</div>
					<div id="best-test">
					</div>
				</div>
			</body>
		</html>
	"#,
		css = assets::html_css("src/stress/style.css").await,
		material_icons = assets::html_material_icons(),
		js = assets::html_js("src/stress/script.js").await,
	)
}
