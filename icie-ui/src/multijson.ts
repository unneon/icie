export class Parser<T> {
	buffer: Buffer;
	constructor() {
		this.buffer = Buffer.alloc(0);
	}
	write(chunk: Buffer) {
		this.buffer = Buffer.concat([this.buffer, chunk])
	}
	read(): T[] {
		let objs: T[] = [];
		let last = 0;
		while (true) {
			let pos = this.buffer.indexOf('\n', last);
			if (pos === -1) {
				break;
			}
			let sub = this.buffer.slice(last, pos);
			last = pos+1;
			let obj = JSON.parse(sub.toString());
			objs.push(obj);
		}
		this.buffer = this.buffer.slice(last);
		return objs;
	}
}
