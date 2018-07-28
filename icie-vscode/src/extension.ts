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
    let disposable = vscode.commands.registerCommand('extension.icieBuild', () => {
        icie.triggerBuild(() => {});
    });
    let disposable2 = vscode.commands.registerCommand('extension.icieTest', () => {
        icie.triggerTest((success) => {});
    });

    context.subscriptions.push(icie);
    context.subscriptions.push(disposable);
    context.subscriptions.push(disposable2);
}

// this method is called when your extension is deactivated
export function deactivate() {
}

class ICIE {

    public triggerBuild(callback: () => void) {
        console.log('ICIE Build triggered.');
        let source = this.getMainSource();
        console.log(`ICIE Build: source = ${source}`);
        cp.execFile(this.getCiPath(), ['build', source], (err, stdout, stderr) => {
            console.log(`ICIE Build: err = ${err}`);
            if (err) {
                throw 'ICIE Build failed.';
            } else {
                console.log('ICIE Build finished.');
                callback();
            }
        });
    }
    public triggerTest(callback: (success: boolean) => void) {
        this.assureCompiled(() => {
            let executable = this.getMainExecutable();
            let testdir = this.getTestDirectory();
            cp.execFile(this.getCiPath(), ['test', executable, testdir], (err, stdout, stderr) => {
                console.log(stdout);
                console.log(stderr);
                console.log('triggerTest.err = ' + err);
                if (err) {
                    callback(false);
                } else {
                    callback(true);
                }
            });
        })
    }

    public dispose() {

    }

    private assureCompiled(callback: () => void) {
        let src = this.getMainSource();
        let exe = this.getMainExecutable();
        fs.stat(src, (e, statsrc) => {
            fs.stat(exe, (e, statexe) => {
                if (statsrc.mtime > statexe.mtime) {
                    this.triggerBuild(() => callback());
                } else {
                    callback();
                }
            });
        });
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

}