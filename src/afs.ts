import * as fs from 'fs';
import * as fs2 from 'fs-extra';

export function exists(path: string): Promise<boolean> {
    return new Promise(resolve => fs.exists(path, resolve));
}
export function stat(path: string): Promise<fs.Stats> {
    return new Promise((resolve, reject) => fs.stat(path, (err, stats) => err ? reject(err) : resolve(stats)));
}
export function mkdir(path: string): Promise<void> {
    return new Promise((resolve, reject) => fs.mkdir(path, err => err ? reject(err) : resolve()));
}
export function read(filename: string, encoding: string): Promise<string> {
    return new Promise((resolve, reject) => fs.readFile(filename, encoding, (err, data) => err ? reject(err) : resolve(data)));
}
export function write(filename: string, content: string | Buffer): Promise<void> {
    return new Promise((resolve, reject) => fs.writeFile(filename, content, err => err ? reject(err) : resolve()));
}
export function copy(from: string, to: string): Promise<void> {
    return fs2.copy(from, to);
}
export async function assureDir(path: string): Promise<void> {
    if (!await exists(path)) {
        await mkdir(path);
    }
}
export function chmod(filename: string, mode: number): Promise<void> {
    return new Promise((resolve, reject) => fs.chmod(filename, mode, err => err ? reject(err) : resolve()));
}