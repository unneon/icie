import * as native from "./native";
import * as panel from './panel';

interface FoodNewStart {
	tag: "new_start";
}
type Food = FoodNewStart;
interface NotesRR {
	tag: "trigger_rr";
	in_path: string;
}
interface NotesNewTest {
	tag: "new_test";
	input: string;
	desired: string;
}
type Notes = NotesRR | NotesNewTest;
type Model = native.TestviewTree;

export class Panel extends panel.Panel<Food, Notes, native.TestviewTree> {
	public constructor(extension_path: string, callback: (notes: Notes) => void) {
		super('icie webview test', 'ICIE Test View', false, extension_path, callback);
	}
	public start_new_test(): void {
		this.focus();
		this.feed({ tag: 'new_start' });
	}
	protected view(tree: Model): string {
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
					<br/>
					<div id="new-container" class="new">
						<textarea id="new-input" class="new"></textarea>
						<textarea id="new-desired" class="new"></textarea>
						<div id="new-start" class="material-icons button new" onclick="new_start()">add</div>
						<div id="new-confirm" class="material-icons button new" onclick="new_confirm()">done</div>
					</div>
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
			let good = tree.outcome === 'accept';
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
}

function lines(text: string): number {
	return text.split('\n').length;
}