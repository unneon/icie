const vscode = acquireVsCodeApi();

let newing = false;

function trigger_rr() {
	let el = event.srcElement;
	vscode.postMessage({
		tag: "trigger_rr",
		in_path: el.parentElement.parentElement.parentElement.dataset.in_path
	});
}

function trigger_gdb() {
	let el = event.srcElement;
	vscode.postMessage({
		tag: "trigger_gdb",
		in_path: el.parentElement.parentElement.parentElement.dataset.in_path
	});
}

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

function clipcopy() {
	let action = event.target;
	let cell = action.parentElement.parentElement;
	let data_node = Array.from(cell.children).find(el => el.classList.contains('test-data'));
	let selection = window.getSelection();
	let range = document.createRange();
	range.selectNodeContents(data_node);
	selection.removeAllRanges();
	selection.addRange(range);
	document.execCommand('Copy');
	selection.removeAllRanges();
	console.log(`copied text to clipboard`);
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
