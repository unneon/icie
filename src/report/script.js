const vscode = acquireVsCodeApi();

function action_open_page() {
	vscode.postMessage({
		tag: 'report_open_page',
	});
}
