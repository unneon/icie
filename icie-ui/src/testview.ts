import * as native from "./native";
import * as vscode from 'vscode';
import * as path from 'path';

export interface InputRR {
	tag: "trigger_rr";
	in_path: string;
}
export interface InputNewTest {
	tag: "new_test";
	input: string;
	desired: string;
}
export type Input = InputRR | InputNewTest;

export class Panel {
	private panel: vscode.WebviewPanel | null;
	private extensionPath: string;
	private callback: (input: Input) => void;
	public constructor(extensionPath: string, callback: (input: Input) => void) {
		this.panel = null;
		this.extensionPath = extensionPath;
		this.callback = callback;
	}
	public focus(): void {
		this.get().reveal();
	}
	public start_new_test(): void {
		this.focus();
		this.get().webview.postMessage({ 'tag': 'new_start' });
	}
	public is_created(): boolean {
		return this.panel !== null;
	}
	public update(tree: native.TestviewTree): void {
		this.get().webview.html = this.view(tree);
	}
	private get(): vscode.WebviewPanel {
		return this.panel || this.create();
	}
	private create(): vscode.WebviewPanel {
		this.panel = vscode.window.createWebviewPanel(
			'icie test view',
			'ICIE Test View',
			vscode.ViewColumn.One,
			{
				enableScripts: true
			}
		);
		this.panel.webview.onDidReceiveMessage(msg => {
			console.log(`<%    ${JSON.stringify(msg)}`);
			this.callback(msg);
		});
		this.panel.onDidDispose(() => this.panel = null);
		return this.panel;
	}
	private view(tree: native.TestviewTree): string {
		return `
			<html>
				<head>
					<link rel="stylesheet" href="${this.asset('web', 'testview.css')}">
					<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">
					<script src="${this.asset('web', 'testview.js')}"></script>
				</head>
				<body>
					<table class="test">
						${this.viewTree(tree)}
					</table>
					<a id="new-start" class="material-icons new button" onclick="new_start()">add</a>
					<a id="new-confirm" class="material-icons new button" onclick="new_confirm()">done</a>
					<textarea class="new" id="new-input"></textarea>
					<textarea class="new" id="new-desired"></textarea>
				</body>
			</html>
		`;
	}
	private viewTree(tree: native.TestviewTree): string {
		if (native.isTest(tree)) {
			let rows = Math.max(...[tree.input, tree.output].map(lines));
			if (tree.desired !== null) {
				rows = Math.max(rows, lines(tree.desired));
			}
			let good = tree.output.trim() === (tree.desired || "").trim();
			return `
				<tr data-in_path="${tree.in_path}">
					<td class="data">
						<div class="actions">
							<i class="action material-icons" title=${tree.name}>info</i>
						</div>
						${tree.input.replace(/\n/g, '<br/>')}
					</td>
					<td class="data ${good ? "out-good" : "out-bad"}">
						<div class="actions">
							<a class="action material-icons" onclick="trigger_rr()">fast_rewind</a>
						</div>
						${tree.output.replace(/\n/g, '<br/>')}
					</td>
					<td class="data">${(tree.desired || "").replace(/\n/g, '<br/>')}</td>
				</tr>
			`;
		} else {
			return `
				${tree.map(tree2 => this.viewTree(tree2)).join('\n')}
			`;
		}
	}
	private asset(...parts: string[]): vscode.Uri {
		return vscode.Uri.file(path.join(this.extensionPath, 'assets', ...parts)).with({ scheme: 'vscode-resource' });
	}
}

function lines(text: string): number {
	return text.split('\n').length;
}