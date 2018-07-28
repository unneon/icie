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
        icie.triggerBuild(() => {});
    });
    let disposable2 = vscode.commands.registerCommand('icie.test', () => {
        icie.triggerTest((success) => {});
    });
    let disposable3 = vscode.commands.registerCommand('icie.init', () => {
        icie.triggerInit();
    });
    let disposable4 = vscode.commands.registerCommand('icie.submit', () => {
        icie.triggerSubmit(() => {});
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

    public triggerBuild(callback: () => void) {
        vscode.window.withProgress({
            location: vscode.ProgressLocation.Notification,
            title: "ICIE Build",
            cancellable: false
        }, (progress, token) => {
            progress.report({ increment: 0, message: 'Saving changes' });
            let source = this.getMainSource();
            console.log(`ICIE.triggerBuild,source = ${source}`);
            return new Promise(resolve => {
                this.assureAllSaved().then(() => {
                    progress.report({ increment: 50, message: 'Compiling' });
                    cp.execFile(this.getCiPath(), ['build', source], (err, stdout, stderr) => {
                        if (!err) {
                            progress.report({ increment: 50, message: 'Finished' });
                            vscode.window.showInformationMessage('ICIE Build finished');
                            resolve(true);
                        } else {
                            progress.report({ increment: 50, message: 'Failed' });
                            console.log(`ICIE.triggerBuild,err = ${err}`);
                            vscode.window.showErrorMessage('ICIE Build failed');
                            resolve(false);
                        }
                    });
                });
            }).then(good => {
                if (good) {
                    callback();
                }
            });
        });
    }
    public triggerTest(callback: (success: boolean) => void) {
        this.assureCompiled().then(() => {
            let executable = this.getMainExecutable();
            let testdir = this.getTestDirectory();
            console.log(`[ICIE.triggerTest] Checking ${executable} agains ${testdir}`);
            cp.execFile(this.getCiPath(), ['test', executable, testdir], (err, stdout, stderr) => {
                console.log(stdout);
                console.log(stderr);
                console.log('triggerTest.err = ' + err);
                if (err) {
                    vscode.window.showErrorMessage('ICIE Test: some tests failed');
                    callback(false);
                } else {
                    vscode.window.showInformationMessage('ICIE Test: all tests passed');
                    callback(true);
                }
            });
        })
    }

    public async triggerInit(): Promise<void> {
        let task_url = await this.askDescriptionURL();
        let project_name = await this.randomProjectName(5);
        let project_dir = homedir() + '/' + project_name;
        await mkdir(project_dir);
        await exec(this.getCiPath(), ['init', task_url], {cwd: project_dir});
        let manifest = new ICIEManifest(task_url);
        await manifest.save(project_dir + '/.icie');
        await exec('cp', [this.getTemplateMainPath(), project_dir + '/' + this.getPreferredMainSource()], {});
        await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(project_dir), false);
    }
    public triggerSubmit(callback: () => void) {
        console.log('ICIE Submit triggered');
        this.triggerTest(tests_succeded => {
            if (tests_succeded) {
                ICIEManifest.load(manifest => {
                    cp.execFile(this.getCiPath(), ['submit', this.getMainSource(), manifest.task_url], (err1, stdout, stderr) => {
                        if (err1) {
                            vscode.window.showErrorMessage('ICIE Submit failed to submit solution');
                            throw 'ICIE Submit failed to submit solution';
                        }
                        callback();
                    });
                });
            }
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
            await new Promise(resolve => this.triggerBuild(resolve));
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

    static load(callback: (manifest: ICIEManifest) => void) {
        fs.readFile(vscode.workspace.rootPath + "/.icie", (err1, data) => {
            // console.log('ICIEManifest.load,err1 = ' + err1);
            if (err1) {
                vscode.window.showErrorMessage('ICIE Submit has not found .icie file, use ICIE Init first');
                throw 'ICIE Submit has not found .icie file, use ICIE Init first';
            }
            let json = JSON.parse(data.toString());
            callback(new ICIEManifest(json.task_url));
        });
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