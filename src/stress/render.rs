pub fn render() -> String {
	format!(
		r#"
		<html>
			<head>
				<style>{css}</style>
				{material_icons}
				<script>{js}</script>
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
		css = include_str!("./style.css"),
		material_icons = crate::util::html_material_icons(),
		js = include_str!("./script.js"),
	)
}
