const vscode = acquireVsCodeApi();

let newing = false;

function make_action(callback) {
	return function() {
		let el = event.target;
		let row = el.parentElement.parentElement.parentElement;
		let in_path = row.dataset.in_path;
		let target_cell = el.parentElement.parentElement;
		return callback({
			row: row,
			in_path: in_path,
			target_cell: target_cell,
		});
	};
}

trigger_copy = make_action(ev => {
	let data_node = Array.from(ev.target_cell.children).find(el => el.classList.contains('test-data'));
	let selection = window.getSelection();
	let range = document.createRange();
	range.selectNodeContents(data_node);
	selection.removeAllRanges();
	selection.addRange(range);
	document.execCommand('Copy');
	selection.removeAllRanges();
});
trigger_rr = make_action(ev => vscode.postMessage({ tag: "trigger_rr", in_path: ev.in_path }));
trigger_gdb = make_action(ev => vscode.postMessage({ tag: "trigger_gdb", in_path: ev.in_path }));
trigger_set_alt = make_action(ev => vscode.postMessage({ tag: "set_alt", in_path: ev.in_path, out: ev.row.dataset.out_raw }));
trigger_del_alt = make_action(ev => vscode.postMessage({ tag: "del_alt", in_path: ev.in_path }));
trigger_edit = make_action(ev => {
	let cell_node = ev.target_cell;
	let path = ev.in_path;
	if (cell_node.classList.contains("test-desired")) {
		path = path.replace(/\.in$/, '.out');
	}
	vscode.postMessage({ tag: "edit", path: path });
});

function new_start() {
	console.log(`new_start()`);
	if (!newing) {
		for (let el of document.getElementsByClassName('new')) {
			el.classList.add("new-active");
		}
		newing = true;
	}
	document.getElementById('new-input').focus();
}

function new_confirm() {
	console.log(`new_confirm()`);
	if (!newing) {
		throw new Error('confirmed the test even though creation has not been started');
	}
	for (let el of document.getElementsByClassName('new')) {
		el.classList.remove("new-active");
	}
	let input = document.getElementById('new-input').value;
	let desired = document.getElementById('new-desired').value;
	document.getElementById('new-input').value = '';
	document.getElementById('new-desired').value = '';
	vscode.postMessage({
		tag: "new_test",
		input: input,
		desired: desired
	});
	newing = false;
}

function scroll_to_wa() {
	let failed = document.getElementsByClassName('test-row-failed');
	if (failed.length > 0) {
		failed[0].scrollIntoView();
	}
}

window.addEventListener('message', event => {
	let message = event.data;
	if (message.tag === 'new_start') {
		if (!newing) {
			new_start();
		} else {
			new_confirm();
		}
	} else if (message.tag === 'scroll_to_wa') {
		scroll_to_wa();
	}
});

window.addEventListener('load', () => {
	let update = function () {
		this.style.height = 'auto';
		this.style.height = `${Math.max(86, this.scrollHeight)}px`;
	};
	for (let tx of document.getElementsByTagName('textarea')) {
		tx.setAttribute('style', `height: ${Math.max(86, tx.scrollHeight)}px; overflow-y: hidden;`);
		tx.addEventListener('input', update, false);
	}
}, false);
