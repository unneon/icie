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
        icie.triggerInit(() => {});
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
                this.assureAllSaved(() => {
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
        this.assureCompiled(() => {
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

    public triggerInit(callback: () => void) {
        this.askDescriptionURL(task_url => {
            this.randomProjectName(5, project_name => {
                let project_dir = homedir() + '/' + project_name;
                fs.mkdir(project_dir, err1 => {
                    cp.execFile(this.getCiPath(), ['init', task_url], {cwd: project_dir}, (err2, stdout, stderr) => {
                        new ICIEManifest(task_url).save(project_dir + '/.icie', () => {
                            cp.execFile('cp', [this.getTemplateMainPath(), project_dir + '/' + this.getPreferredMainSource()], (err3, stdout2, stderr3) => {
                                if (err3) {
                                    vscode.window.showErrorMessage('ICIE Init not found C++ template code at ~/.config/icie/template-main.cpp');
                                    throw 'ICIE Init not found C++ template code at ~/.config/icie/template-main.cpp';
                                }
                                vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(project_dir), false);
                            });
                        });
                    });
                });
            });
        });
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

    private assureCompiled(callback: () => void) {
        let src = this.getMainSource();
        console.log(`[ICIE.assureCompiled] Checking whether ${src} is compiled`);
        let exe = this.getMainExecutable();
        this.assureAllSaved(() => {
            fs.stat(src, (e1, statsrc) => {
                console.log(`[ICIE.assureCompiled] stat(src) ran, e1 = ${e1}`);
                fs.stat(exe, (e2, statexe) => {
                    console.log(`[ICIE.assureCompiled] stat(exe) ran, e2 = ${e2}`);
                    if (!e2 && statsrc.mtime <= statexe.mtime) {
                        console.log(`[ICIE.assureCompiled] ${src} was compiled already`);
                        callback();
                    } else {
                        console.log(`[ICIE.assureCompiled] ${src} needs compiling`);
                        this.triggerBuild(() => callback());
                    }
                });
            });
        });
    }
    private assureAllSaved(callback: () => void) {
        vscode.workspace.saveAll(false).then((whatisthis) => {
            callback();
        }, (whatisthis) => {
            callback();
        });
    }

    private askDescriptionURL(callback: (answer: string) => void) {
        let options: vscode.InputBoxOptions = {
            prompt: 'Task description URL: '
        };
        vscode.window.showInputBox(options).then(value => {
            if (value) {
                callback(value);
            }
        })
    }

    private randomProjectName(tries: number, callback: (project_name: string) => void) {
        if (tries === 0) {
            throw "ICIE Init: failed to find free project name";
        }
        let name = this.randomName();
        fs.exists(homedir + '/' + name, already_exists => {
            if (already_exists) {
                this.randomProjectName(tries - 1, callback);
            } else {
                callback(name);
            }
        });
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
    public save(path: string, callback: () => void) {
        fs.writeFile(path, JSON.stringify(this), err1 => {
            if (err1) {
                vscode.window.showErrorMessage('failed to save .icie file');
                throw 'failed to save .icie file';
            }
            callback();
        });
    }

}

function choice<T>(xs: T[]): T {
    return xs[Math.floor(Math.random() * xs.length)];
}