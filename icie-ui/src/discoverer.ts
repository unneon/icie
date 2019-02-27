import * as native from './native';
import * as panel from './panel';

interface FoodRow {
	tag: 'discovery_row';
	number: number;
	outcome: native.Outcome;
	fitness: number;
	input: string | null;
}
interface FoodState {
	tag: 'discovery_state';
	running: boolean;
	reset: boolean;
}
type Food = FoodRow | FoodState;
interface NotesStart {
	tag: 'discovery_start';
}
interface NotesPause {
	tag: 'discovery_pause';
}
interface NotesReset {
	tag: 'discovery_reset';
}
interface NotesSave {
	tag: 'discovery_save';
	input: string;
}
type Notes = NotesStart | NotesPause | NotesReset | NotesSave;
type Model = {};

export class Panel extends panel.Panel<Food, Notes, {}> {
	public constructor(extension_path: string, callback: (notes: Notes) => void) {
		super('icie webview discoverer', 'ICIE Discoverer', true, extension_path, callback);
	}
	public react(food: Food): void {
		this.feed(food);
	}
	protected view(model: Model): string {
		return `
			<html>
				<head>
					<link rel="stylesheet" href="${this.asset('web', 'discoverer.css')}">
					<link href="https://fonts.googleapis.com/icon?family=Material+Icons" rel="stylesheet">
					<script src="${this.asset('web', 'discoverer.js')}"></script>
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
		`;
	}
}

