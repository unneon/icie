const vscode = acquireVsCodeApi();

function action_save() {
	// TODO: How does this work if Rust code does not contain this string?
	vscode.postMessage({
		tag: 'stress_save',
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
			td2.classList.add(`verdict-${message.verdict}`);
			td2.textContent = pretty_verdict(message.verdict);
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

function pretty_verdict(verdict) {
	if (verdict === 'accept') {
		return 'Accept';
	} else if (verdict === 'wrong_answer') {
		return 'Wrong answer';
	} else if (verdict === 'runtime_error') {
		return 'Runtime error';
	} else if (verdict === 'time_limit_exceeded') {
		return 'Time limit exceeded';
	} else if (verdict === 'ignored_no_out') {
		return 'Ignored because of no out';
	} else {
		throw new Error('unrecognized verdict');
	}
}
