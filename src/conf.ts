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
    if (!await afs.exists(configPath())) {
        await createDefaultConfig();
    }
    let data = await afs.read(configPath(), 'utf8');
    let json = JSON.parse(data.toString());
    return json;
}

// TODO use xdg env vars
async function createDefaultConfig() {
    await afs.assureDir(os.homedir() + '/.config');
    await afs.assureDir(os.homedir() + '/.config/icie');
    if (!await afs.exists(os.homedir() + '/.config/icie/config.json')) {
        await afs.write(os.homedir() + '/.config/icie/config.json', JSON.stringify({
            template: {
                path: os.homedir() + '/.config/icie/template-main.cpp',
                start: {
                    row: 8,
                    column: 5
                }
            }
        }));
        if (!await afs.exists(os.homedir() + '/.config/icie/template-main.cpp')) {
            await afs.write(os.homedir() + '/.config/icie/template-main.cpp', `#include <bits/stdc++.h>
using namespace std;
// Edit your config and template at ${os.homedir()}/.config/icie 😄 💖

int main() {
    ios::sync_with_stdio(false);
    cin.tie(nullptr);
    
}
`);
        }
    }
}


function configPath(): string {
    return os.homedir() + '/.config/icie/config.json';
}