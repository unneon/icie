const vscode = acquireVsCodeApi();

let newing = false;

function trigger_rr() {
	let el = event.srcElement;
	vscode.postMessage({
		tag: "trigger_rr",
		in_path: el.parentElement.parentElement.parentElement.dataset.in_path
	});
}

function new_start() {
	console.log(`new_start()`);
	if (!newing) {
		for (let el of document.getElementsByClassName('new')) {
			el.classList.add("new-newing");
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
		el.classList.remove("new-newing");
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

window.addEventListener('message', event => {
	let message = event.data;
	if (message.tag === 'new_start') {
		if (!newing) {
			new_start();
		} else {
			new_confirm();
		}
	}
});