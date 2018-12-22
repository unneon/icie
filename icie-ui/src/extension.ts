'use strict';
import * as vscode from 'vscode';
import * as native from './native';

export function activate(context: vscode.ExtensionContext) {

    let disposable = vscode.commands.registerCommand('icie.hello', () => {
        native.send({ tag: "ping" });
    });
    let disposable2 = vscode.commands.registerCommand('icie.build', () => {
        native.send({ tag: "trigger_build" });
    });
    let disposable3 = vscode.commands.registerCommand('icie.test', () => {
        native.send({ tag: "trigger_test" });
    });
    let disposable4 = vscode.commands.registerCommand('icie.init', () => {
        native.send({ tag: "trigger_init" });
    });

    let status = vscode.window.createStatusBarItem();

    let callback = (err: any, reaction: native.Reaction) => {
        if (reaction.tag === "status") {
            if (reaction.message !== undefined) {
                status.text = `${reaction.message}`;
                status.show();
            } else {
                status.hide();
            }
        } else if (reaction.tag === "info_message") {
            vscode.window.showInformationMessage(reaction.message);
        } else if (reaction.tag === "error_message") {
            vscode.window.showErrorMessage(reaction.message);
        } else if (reaction.tag === "quick_pick") {
            vscode.window.showQuickPick(reaction.items).then(item => {
                if (item !== undefined) {
                    native.send({ tag: "quick_pick", response: item.id });
                } else {
                    native.send({ tag: "quick_pick", response: null });
                }

            });
        } else if (reaction.tag === "input_box") {
            vscode.window.showInputBox(reaction).then(value => {
                if (value !== undefined) {
                    native.send({ tag: "input_box", response: value });
                } else {
                    native.send({ tag: "input_box", response: null });
                }
            });
        } else if (reaction.tag === "console_log") {
            console.log(reaction.message);
        } else if (reaction.tag === "save_all") {
            vscode.workspace.saveAll(false).then(something => {
                native.send({ tag: "saved_all" });
            });
        } else if (reaction.tag === "open_folder") {
            vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(reaction.path), reaction.in_new_window);
        }
        native.recv(callback);
    };
    native.recv(callback);

    native.send({
        tag: "workspace_info",
        root_path: vscode.workspace.rootPath || null,
    });

    context.subscriptions.push(disposable);
    context.subscriptions.push(disposable2);
    context.subscriptions.push(disposable3);
    context.subscriptions.push(disposable4);
}

export function deactivate() {
}