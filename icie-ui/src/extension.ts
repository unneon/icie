'use strict';
import * as vscode from 'vscode';
import * as native from './native';
import { ReactionProgressUpdate, ReactionProgressEnd } from 'icie-wrap';

interface ProgressUpdate {
    reaction: ReactionProgressUpdate | ReactionProgressEnd,
    recv: Promise<ProgressUpdate>,
}

export function activate(context: vscode.ExtensionContext) {
    register_trigger('icie.build', { tag: "trigger_build" }, context);
    register_trigger('icie.test', { tag: "trigger_test" }, context);
    register_trigger('icie.init', { tag: "trigger_init" }, context);
    register_trigger('icie.submit', { tag: "trigger_submit" }, context);
    register_trigger('icie.manual.submit', { tag: "trigger_manual_submit" }, context);
    register_trigger('icie.template.instantiate', { tag: "trigger_template_instantiate" }, context);

    let status = vscode.window.createStatusBarItem();
    let progressRegister: ChannelRegister<ProgressUpdate> = {};

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
        } else if (reaction.tag === "console_error") {
            console.error(reaction.message);
        } else if (reaction.tag === "open_editor") {
            vscode.workspace.openTextDocument(reaction.path)
                .then(source => vscode.window.showTextDocument(source))
                .then(editor => {
                    let oldPosition = editor.selection.active;
                    let newPosition = oldPosition.with(reaction.row-1, reaction.column-1);
                    let newSelection = new vscode.Selection(newPosition, newPosition);
                    editor.selection = newSelection;
                });
        } else if (reaction.tag === "progress_start") {
            let c0 = channel<ProgressUpdate>();
            progressRegister[reaction.id] = c0.send;
            let recv = c0.recv;
            vscode.window.withProgress({
                cancellable: false,
                location: vscode.ProgressLocation.Notification,
                title: reaction.title
            }, async progress => {
                while (true) {
                    let event = await recv;
                    recv = event.recv;
                    if (event.reaction.tag === "progress_update") {
                        progress.report({
                            message: event.reaction.message,
                            increment: event.reaction.increment
                        });
                    } else if (event.reaction.tag === "progress_end") {
                        break;
                    }
                }
            });
        } else if (reaction.tag === "progress_update" || reaction.tag === "progress_end") {
            let c2 = channel<ProgressUpdate>();
            let send = progressRegister[reaction.id];
            delete progressRegister[reaction.id];
            progressRegister[reaction.id] = c2.send;
            send({ reaction: reaction, recv: c2.recv });
        }
        native.recv(callback);
    };
    native.recv(callback);

    native.send({
        tag: "workspace_info",
        root_path: vscode.workspace.rootPath || null,
    });
}

export function deactivate() {
}

function register_trigger(command: string, impulse: native.Impulse, context: vscode.ExtensionContext): void {
    context.subscriptions.push(vscode.commands.registerCommand(command, () => native.send(impulse)));
}

interface Channel<T> {
    recv: Promise<T>;
    send: (x: T) => void;
}
function channel<T>(): Channel<T> {
    let send: ((x: T) => void) | null = null;
    let recv: Promise<T> = new Promise(resolve => {
        send = resolve;
    });
    if (send === null) {
        throw new Error("@channel.#send === null");
    }
    return { recv, send };
}

interface ChannelRegister<T> {
    [id: string]: (x: T) => void;
}
