import * as afs from './afs';
import * as os from 'os';

interface TextFilePos {
    row: number;
    column: number;
}

interface SourceTemplate {
    path: string;
    start: TextFilePos;
}

export interface Config {
    template: SourceTemplate;
}

export async function load(): Promise<Config> {
    let data = await afs.read(configPath(), 'utf8');
    let json = JSON.parse(data.toString());
    return json;
}

function configPath(): string {
    return os.homedir() + '/.config/icie/config.json';
}