import * as vscode from 'vscode';
import * as path from 'path';

export abstract class Panel<Food, Notes, Model> {
	private panel: vscode.WebviewPanel | null;
	private extensionPath: string;
	private view_type: string;
	private title: string;
	private retain_context_when_hidden: boolean;
	private callback: (notes: Notes) => void;
	public constructor(view_type: string, title: string, retain_context_when_hidden: boolean, extensionPath: string, callback: (notes: Notes) => void) {
		this.panel = null;
		this.extensionPath = extensionPath;
		this.view_type = view_type;
		this.title = title;
		this.retain_context_when_hidden = retain_context_when_hidden;
		this.callback = callback;
	}
	public focus(): void {
		this.get().reveal();
	}
	public is_created(): boolean {
		return this.panel !== null;
	}
	public update(model: Model): void {
		let html = this.view(model);
		this.get().webview.html = html;
	}
	protected abstract view(model: Model): string;
	protected feed(food: Food): void {
		this.get().webview.postMessage(food);
	}
	protected asset(...parts: string[]): vscode.Uri {
		return vscode.Uri.file(path.join(this.extensionPath, 'assets', ...parts)).with({ scheme: 'vscode-resource' });
	}
	private get(): vscode.WebviewPanel {
		return this.panel || this.create();
	}
	private create(): vscode.WebviewPanel {
		this.panel = vscode.window.createWebviewPanel(
			this.view_type,
			this.title,
			{
				viewColumn: vscode.ViewColumn.One,
				preserveFocus: true
			},
			{
				enableScripts: true,
				retainContextWhenHidden: this.retain_context_when_hidden
			}
		);
		this.panel.webview.onDidReceiveMessage(msg => {
			console.log(`<%    ${JSON.stringify(msg)}`);
			this.callback(msg);
		});
		this.panel.onDidDispose(() => this.panel = null);
		return this.panel;
	}
}
