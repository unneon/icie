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
					<div class="controls">
						<a id="start" class="material-icons control-button" onclick="button_start()">play_arrow</a>
						<a id="pause" class="material-icons control-button" onclick="button_pause()">pause</a>
						<br/>
						<a id="reset" class="material-icons control-button" onclick="button_clear()">clear</a>
					</div>
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
