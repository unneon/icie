class PageViewer {
	constructor(page, canvas) {
		this.canvas = canvas;
		this.context = this.canvas.getContext('2d');
		this.original_width = page.getViewport({ scale: 1 }).width;
		this.page = page;
		this.update();
		this.update(); // update twice to account for the scrollbar
	}
	update() {
		let viewport = this.page.getViewport({
			scale: document.body.clientWidth / this.original_width
		});
		this.canvas.width = viewport.width;
		this.canvas.height = viewport.height;
		this.page.render({
			canvasContext: this.context,
			viewport: viewport
		});
	}
}

let page_viewers = [];
receive_message().then(preps => {
	let pdf_data = Uint8Array.from(event.data.pdf_data_base64);
	pdfjsLib.getDocument(pdf_data).promise.then(doc => {
		let body = document.getElementById('body');
		for (let i=1; i<=doc.numPages; ++i) {
			let canvas = document.createElement('canvas');
			body.appendChild(canvas);
			doc.getPage(i).then(page => {
				page_viewers.push(new PageViewer(page, canvas));
			});
		}
	});
});
window.onresize = () => {
	for (let page_viewer of page_viewers) {
		page_viewer.update();
	}
};

function receive_message() {
	return new Promise(resolve => {
		window.addEventListener('message', event => {
			resolve(event);
		});
		let vscode = acquireVsCodeApi();
		vscode.postMessage({ tag: "notify-live" });
	});
}
