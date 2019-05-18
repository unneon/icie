const vscode = acquireVsCodeApi();

function button_start() {
	vscode.postMessage({
		tag: 'discovery_start'
	});
}

function button_pause() {
	vscode.postMessage({
		tag: 'discovery_pause'
	});
}

function button_clear() {
	vscode.postMessage({
		tag: 'discovery_reset'
	});
}

function action_save() {
	vscode.postMessage({
		tag: 'discovery_save',
		input: document.getElementById('best-test').dataset.input
	});
}

window.addEventListener('message', event => {
	let message = event.data;
	if (message.tag === 'discovery_state') {
		let actions = document.getElementsByClassName('control-button');
		let best_test = document.getElementById('best-test');
		let current = document.getElementById('current');
		let normal_tests = document.getElementsByClassName('normal-test');
		if (message.running === true) {
			for (let action of actions) {
				action.classList.add('running');
			}
			current.classList.add('running');
		} else {
			for (let action of actions) {
				action.classList.remove('running');
			}
			current.classList.remove('running');
		}
		if (message.reset === true) {
			best_test.textContent = '';
			while (normal_tests.length > 0) {
				normal_tests[0].remove();
			}
			current.children[0].textContent = 1;
		}

	} else if (message.tag === 'discovery_row') {
		let current = document.getElementById('current');
		let log_body = document.getElementById('log-body');
		current.children[0].textContent = message.number + 1;
		if (message.input !== null) {
			let tr = document.createElement('tr');
			tr.classList.add('normal-test');
			let td1 = document.createElement('td');
			td1.textContent = message.number;
			let td2 = document.createElement('td');
			td2.classList.add(`outcome-${message.outcome}`);
			td2.textContent = pretty_outcome(message.outcome);
			let td3 = document.createElement('td');
			td3.textContent = message.fitness;
			tr.appendChild(td1);
			tr.appendChild(td2);
			tr.appendChild(td3);
			if (log_body.children.length > 1) {
				log_body.insertBefore(tr, current.nextSibling);
			} else {
				log_body.appendChild(tr);
			}
			let best_test = document.getElementById('best-test');
			best_test.innerHTML = message.input.replace(/\n/g, '<br/>');
			best_test.dataset.input = message.input;
		}
	}
});

function pretty_outcome(outcome) {
	if (outcome === 'accept') {
		return 'Accept';
	} else if (outcome === 'wrong_answer') {
		return 'Wrong answer';
	} else if (outcome === 'runtime_error') {
		return 'Runtime error';
	} else if (outcome === 'time_limit_exceeded') {
		return 'Time limit exceeded';
	} else if (outcome === 'ignored_no_out') {
		return 'Ignored because of no out';
	} else {
		throw new Error('unrecognized outcome');
	}
}