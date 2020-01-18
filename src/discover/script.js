const vscode = acquireVsCodeApi();

function action_save() {
	vscode.postMessage({
		tag: 'discovery_save',
	});
}

window.addEventListener('message', event => {
	let message = event.data;
	if (message.tag === 'row') {
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
