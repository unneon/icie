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
		super('icie webview test', 'ICIE Test View', true, extension_path, callback);
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
					<table class="test-table">
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
			return `
				<tr class="test-row" data-in_path="${tree.in_path}">
					<td style="height: ${1.1*lines(tree.input.trim())}em; line-height: 1.1em;" class="test-cell">
						<div class="test-actions">
							<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
							<div class="test-action material-icons" title=${tree.name}>info</div>
						</div>
						<div class="test-data">
							${tree.input.replace(/\n/g, '<br/>')}
						</div>
					</td>
					<td class="test-cell ${test_outcome_class(tree.outcome)}">
						<div class="test-actions">
							<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
							<div class="test-action material-icons" onclick="trigger_rr()">fast_rewind</div>
						</div>
						<div class="test-data">
							${tree.output.replace(/\n/g, '<br/>')}
						</div>
						${view_out_note(tree.outcome)}
					</td>
					<td class="test-cell">
						${tree.desired !== null ? `
							<div class="test-actions">
								<div class="test-action material-icons" onclick="clipcopy()">file_copy</div>
							</div>
							<div class="test-data">
								${tree.desired.replace(/\n/g, '<br/>')}
							</div>
						` : `
							<div class="test-note">
								File does not exist
							</div>
						`}
					</td>
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
function test_outcome_class(outcome: native.Outcome): string {
	if (outcome === 'accept') {
		return 'test-good';
	} else if (outcome === 'wrong_answer' || outcome === 'runtime_error' || outcome == 'time_limit_exceeded') {
		return 'test-bad';
	} else if (outcome === 'ignored_no_out') {
		return 'test-warn';
	} else {
		throw new Error(`unrecognized outcome ${outcome}`);
	}
}
function view_out_note(outcome: native.Outcome): string {
	if (outcome !== 'accept' && outcome !== 'wrong_answer') {
		return `
			<div class="test-note">
				${pretty_outcome(outcome)}
			</div>
		`;
	} else {
		return ``;
	}
}
function pretty_outcome(outcome: native.Outcome): string {
	if (outcome === 'accept') {
		return 'Accept';
	} else if (outcome === 'wrong_answer') {
		return 'Wrong Answer';
	} else if (outcome === 'runtime_error') {
		return 'Runtime Error';
	} else if (outcome === 'time_limit_exceeded') {
		return 'Time Limit Exceeded';
	} else if (outcome === 'ignored_no_out') {
		return 'Ignored';
	} else {
		throw new Error(`unrecognized outcome ${outcome}`);
	}
}