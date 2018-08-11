import * as afs from './afs';

export interface Manifest {
    task_url: string;
}

export async function load(path: string): Promise<Manifest> {
    let data = await afs.read(path, 'utf8');
    let json = JSON.parse(data.toString());
    return json;
}

export async function save(path: string, manifest: Manifest): Promise<void> {
    return afs.write(path, JSON.stringify(manifest));
}