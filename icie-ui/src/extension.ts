'use strict';
// The module 'vscode' contains the VS Code extensibility API
// Import the module and reference it with the alias vscode in your code below
import * as vscode from 'vscode';
import * as icie_wrap from 'icie-wrap';

function icie_send(impulse: icie_wrap.Impulse) {
    console.log(`   ~> ${JSON.stringify(impulse)}`);
    let ermsg = icie_wrap.message_send(impulse);
    if (!ermsg.startsWith("Message sent successfully")) {
        console.error(`   #> ${ermsg}`);
    }
}

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {

    // Use the console to output diagnostic information (console.log) and errors (console.error)
    // This line of code will only be executed once when your extension is activated
    console.log('Congratulations, your extension "icie" is now active!');

    // The command has been defined in the package.json file
    // Now provide the implementation of the command with  registerCommand
    // The commandId parameter must match the command field in package.json
    let disposable = vscode.commands.registerCommand('icie.hello', () => {
        // The code you place here will be executed every time your command is executed
        icie_send({ tag: "ping" });
    });
    let disposable2 = vscode.commands.registerCommand('icie.build', () => {
        icie_send({ tag: "trigger_build" });
    });

    let status = vscode.window.createStatusBarItem();

    let callback = (err: any, reaction: icie_wrap.Reaction) => {
        console.log(`<~    ${JSON.stringify(reaction)}`);
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
                    icie_send({ tag: "quick_pick", response: item.id });
                } else {
                    icie_send({ tag: "quick_pick", response: null });
                }
            });
        } else if (reaction.tag === "input_box") {
            vscode.window.showInputBox(reaction).then(value => {
                if (value !== undefined) {
                    icie_send({ tag: "input_box", response: value });
                } else {
                    icie_send({ tag: "input_box", response: null });
                }
            });
        } else if (reaction.tag === "console_log") {
            console.log(reaction.message);
        }
        icie_wrap.message_recv(callback);
    };
    icie_wrap.message_recv(callback);

    icie_send({
        tag: "workspace_info",
        root_path: vscode.workspace.rootPath || null,
    });

    context.subscriptions.push(disposable);
    context.subscriptions.push(disposable2);
}

// this method is called when your extension is deactivated
export function deactivate() {
}