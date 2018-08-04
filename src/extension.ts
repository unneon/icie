'use strict';
import * as vscode from 'vscode';
import * as cp from 'child_process';
import { homedir } from 'os';
import * as fs from 'fs';

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {
    console.log('Congratulations, your extension "icie" is now active!');

    let icie = new ICIE();

    // The command has been defined in the package.json file
    // Now provide the implementation of the command with  registerCommand
    // The commandId parameter must match the command field in package.json
    let disposable = vscode.commands.registerCommand('icie.build', () => {
        icie.triggerBuild();
    });
    let disposable2 = vscode.commands.registerCommand('icie.test', () => {
        icie.triggerTest();
    });
    let disposable3 = vscode.commands.registerCommand('icie.init', () => {
        icie.triggerInit();
    });
    let disposable4 = vscode.commands.registerCommand('icie.submit', () => {
        icie.triggerSubmit();
    });

    context.subscriptions.push(icie);
    context.subscriptions.push(disposable);
    context.subscriptions.push(disposable2);
    context.subscriptions.push(disposable3);
    context.subscriptions.push(disposable4);
}

// this method is called when your extension is deactivated
export function deactivate() {
}

class ICIE {

    public async triggerBuild(): Promise<void> {
        let source = this.getMainSource();
        await this.assureAllSaved();
        try {
            await exec(this.getCiPath(), ['build', source], {});
            vscode.window.showInformationMessage('ICIE Build finished');
        } catch (err) {
            vscode.window.showErrorMessage('ICIE Build failed');
            return Promise.reject('ICIE build failed: ' + err);
        }
    }
    public async triggerTest(): Promise<boolean> {
        await this.assureCompiled();
        let executable = this.getMainExecutable();
        let testdir = this.getTestDirectory();
        console.log(`[ICIE.triggerTest] Checking ${executable} agains ${testdir}`);
        try {
            await exec(this.getCiPath(), ['test', executable, testdir], {});
            vscode.window.showInformationMessage('ICIE Test: all tests passed');
            return true;
        } catch (err) {
            vscode.window.showErrorMessage('ICIE Test: some tests failed');
            return false;
        }
    }

    public async triggerInit(): Promise<void> {
        let task_url = await this.askDescriptionURL();
        let project_name = await this.randomProjectName(5);
        let project_dir = homedir() + '/' + project_name;
        await mkdir(project_dir);
        await execInteractive(this.getCiPath(), ['--format', 'json', 'init', task_url], {cwd: project_dir}, kid => this.respondToAuthreq(kid));
        let manifest = new ICIEManifest(task_url);
        await manifest.save(project_dir + '/.icie');
        await exec('cp', [this.getTemplateMainPath(), project_dir + '/' + this.getPreferredMainSource()], {});
        await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(project_dir), false);
    }
    public async triggerSubmit(): Promise<void> {
        console.log('ICIE Submit triggered');
        let tests_succeded = await this.triggerTest();
        if (tests_succeded) {
            let manifest = await ICIEManifest.load();
            await exec(this.getCiPath(), ['submit', this.getMainSource(), manifest.task_url], {});
        }
    }
    private respondToAuthreq(kid: cp.ChildProcess) {
        // TODO this is wrong, but node sucks and I can't be bothered
        kid.stdout.on('data', chunk => {
            console.log(`ICIE.@respondToAuthreq.kid.stdout.#data.chunk = ${chunk}`);
            this.respondToAuthreqLine(kid, chunk.toString());
        });
    }
    private async respondToAuthreqLine(kid: cp.ChildProcess, line: string): Promise<void> {
        let authreq = JSON.parse(line);
        let domain = authreq.domain;
        console.log('ICIE.@respondToAuthreqLine.domain:', domain);
        let username = await inputbox({ prompt: `Username at ${domain}` });
        console.log('username:', username);
        let password = await inputbox({ prompt: `Password for ${username} at ${domain}` });
        let authresp = JSON.stringify({
            'username': username,
            'password': password
        });
        kid.stdin.write(authresp, 'utf8', () => {
            kid.stdin.end();
        });
    }

    public dispose() {

    }

    private async requiresCompilation(): Promise<boolean> {
        let src = this.getMainSource();
        console.log(`[ICIE.assureCompiled] Checking whether ${src} is compiled`);
        let exe = this.getMainExecutable();
        await this.assureAllSaved();
        let statsrc = await file_stat(src);
        try {
            var statexe = await file_stat(exe);
        } catch (err) {
            return true;
        }
        if (statsrc.mtime <= statexe.mtime) {
            console.log(`[ICIE.assureCompiled] ${src} was compiled already`);
            return false;
        } else {
            console.log(`[ICIE.assureCompiled] ${src} needs compiling`);
            return true;
        }
    }

    private async assureCompiled(): Promise<void> {
        if (await this.requiresCompilation()) {
            await this.triggerBuild();
        }
    }
    private async assureAllSaved(): Promise<void> {
        return vscode.workspace.saveAll(false).then(something => {});
    }

    private askDescriptionURL(): Promise<string> {
        let options: vscode.InputBoxOptions = {
            prompt: 'Task description URL: '
        };
        return new Promise((resolve, reject) => {
            vscode.window.showInputBox(options).then(value => {
                if (value) {
                    resolve(value);
                } else {
                    reject('problem description url not entered');
                }
            });
        });
    }

    private async randomProjectName(tries: number): Promise<string> {
        for (; tries>0; --tries) {
            let name = this.randomName();
            if (!await file_exists(homedir() + '/' + name)) {
                return name;
            }
        }
        return Promise.reject('failed to find free project name');
    }
    private randomName(): string {
        let adjectives = [
            "playful",
            "shining",
            "sparkling",
            "rainbow",
            "kawaii",
            "superb",
            "amazing",
            "glowing",
            "blessed",
            "smiling",
        ];
        let animals = [
            "capybara",
            "squirrel",
            "spider",
            "anteater",
            "hamster",
            "whale",
            "eagle",
            "zebra",
            "dolphin",
            "hedgehog",
        ];
        return choice(adjectives) + '-' + choice(animals);
    }

    private getMainSource(): string {
        let editor = vscode.window.activeTextEditor;
        if (!editor) {
            throw "ICIE Build: editor not found";
        }
        let doc = editor.document;
        return doc.fileName.toString();
    }
    private getMainExecutable(): string {
        let source = this.getMainSource();
        return source.substr(0, source.length - 4) + '.e';
    }
    private getTestDirectory(): string {
        let path = vscode.workspace.rootPath;
        console.log('ICIE Build: getTestDirectory.path = ' + path);
        if (!path) {
            throw 'ICIE Build: path not found';
        }
        return path;
    }
    private getCiPath(): string {
        return homedir() + '/.cargo/bin/ci';
    }
    private getTemplateMainPath(): string {
        return homedir() + '/.config/icie/template-main.cpp'
    }
    private getPreferredMainSource(): string {
        return "main.cpp";
    }

}

class ICIEManifest {

    public task_url: string;

    public constructor(task_url: string) {
        this.task_url = task_url;
    }

    static async load(): Promise<ICIEManifest> {
        let data = await readFile(vscode.workspace.rootPath + '/.icie', 'utf8');
        let json = JSON.parse(data.toString());
        return new ICIEManifest(json.task_url);
    }
    public save(path: string): Promise<void> {
        return new Promise((resolve, reject) => fs.writeFile(path, JSON.stringify(this), err1 => {
            if (err1) {
                reject(err1);
            } else {
                resolve();
            }
        }));
    }

}

function choice<T>(xs: T[]): T {
    return xs[Math.floor(Math.random() * xs.length)];
}
function file_exists(path: string): Promise<boolean> {
    return new Promise(resolve => {
        fs.exists(path, resolve);
    });
}
function file_stat(path: string): Promise<fs.Stats> {
    return new Promise((resolve, reject) => {
        fs.stat(path, (err, stats) => {
            if (err) {
                reject(err);
            } else {
                resolve(stats);
            }
        })
    });
}
function mkdir(path: string): Promise<void> {
    return new Promise((resolve, reject) => {
        fs.mkdir(path, err => {
            if (err) {
                reject(err);
            } else {
                resolve();
            }
        });
    });
}
interface ExecOutput {
    stdout: string,
    stderr: string,
}
function exec(file: string, args: string[], options: cp.ExecFileOptions): Promise<ExecOutput> {
    return new Promise((resolve, reject) => cp.execFile(file, args, options, (err, stdout, stderr) => {
        if (err) {
            reject(err);
        } else {
            resolve({ stdout: stdout, stderr: stderr });
        }
    }));
}
function execInteractive(file: string, args: string[], options: cp.ExecFileOptions, while_running: (kid: cp.ChildProcess) => void): Promise<void> {
    return new Promise((resolve, reject) => {
        let kid = cp.spawn(file, args, options);
        kid.on('exit', (code, signal) => {
            console.log('sigh exit');
            resolve();
        });
        while_running(kid);
    });
}
function readFile(filename: string, encoding: string): Promise<string> {
    return new Promise((resolve, reject) => {
        fs.readFile(filename, encoding, (err, data) => {
            if (err) {
                reject(err);
            } else {
                resolve(data);
            }
        })
    });
}
function then2promise<T>(t: Thenable<T>): Promise<T> {
    return new Promise((resolve, reject) => {
        t.then(val => resolve(val), reason => reject(reason));
    });
}
function inputbox(options: vscode.InputBoxOptions): Promise<string | undefined> {
    return then2promise(vscode.window.showInputBox(options));
}