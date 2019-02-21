'use strict';
import * as vscode from 'vscode';
import * as channel from './channel';
import * as native from './native';
import * as testview from './testview';

interface ProgressUpdate {
    reaction: native.ReactionProgressUpdate | native.ReactionProgressEnd;
    recv: Promise<ProgressUpdate>;
}

export function activate(context: vscode.ExtensionContext) {
    let logic = new native.Logic(context.extensionPath);
    let status = vscode.window.createStatusBarItem();
    let progressRegister: ChannelRegister<ProgressUpdate> = {};
    let testview_panel = new testview.Panel(context.extensionPath, input => {
        if (input.tag === 'trigger_rr') {
            logic.send(input);
        }
    });

    register_trigger('icie.build', { tag: "trigger_build" }, logic, context);
    register_trigger('icie.test', { tag: "trigger_test" }, logic, context);
    register_trigger('icie.init', { tag: "trigger_init" }, logic, context);
    register_trigger('icie.submit', { tag: "trigger_submit" }, logic, context);
    register_trigger('icie.manual.submit', { tag: "trigger_manual_submit" }, logic, context);
    register_trigger('icie.template.instantiate', { tag: "trigger_template_instantiate" }, logic, context);
    register_trigger('icie.test.view', { tag: "trigger_testview" }, logic, context);

    let callback = (reaction: native.Reaction) => {
        if (reaction.tag === "status") {
            if (reaction.message !== null) {
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
            vscode.window.showQuickPick(reaction.items.map(item => {
                return {
                    description: item.description || undefined,
                    detail: item.detail || undefined,
                    label: item.label,
                    id: item.id
                };
            })).then(item => {
                if (item !== undefined) {
                    logic.send({ tag: "quick_pick", response: item.id });
                } else {
                    logic.send({ tag: "quick_pick", response: null });
                }

            });
        } else if (reaction.tag === "input_box") {
            vscode.window.showInputBox({
                ignoreFocusOut: reaction.ignoreFocusOut,
                password: reaction.password,
                placeHolder: reaction.placeholder || undefined,
                prompt: reaction.prompt || undefined
            }).then(value => {
                if (value !== undefined) {
                    logic.send({ tag: "input_box", response: value });
                } else {
                    logic.send({ tag: "input_box", response: null });
                }
            });
        } else if (reaction.tag === "console_log") {
            console.log(reaction.message);
        } else if (reaction.tag === "save_all") {
            vscode.workspace.saveAll(false).then(something => {
                logic.send({ tag: "saved_all" });
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
            let c0 = channel.channel<ProgressUpdate>();
            progressRegister[reaction.id] = c0.send;
            let recv = c0.recv;
            vscode.window.withProgress({
                cancellable: false,
                location: vscode.ProgressLocation.Notification,
                title: reaction.title || undefined
            }, async progress => {
                while (true) {
                    let event = await recv;
                    recv = event.recv;
                    if (event.reaction.tag === "progress_update") {
                        progress.report({
                            message: event.reaction.message || undefined,
                            increment: event.reaction.increment || undefined
                        });
                    } else if (event.reaction.tag === "progress_end") {
                        break;
                    }
                }
            });
        } else if (reaction.tag === "progress_update" || reaction.tag === "progress_end") {
            let c2 = channel.channel<ProgressUpdate>();
            let send = progressRegister[reaction.id];
            delete progressRegister[reaction.id];
            progressRegister[reaction.id] = c2.send;
            send({ reaction: reaction, recv: c2.recv });
        } else if (reaction.tag === "testview_focus") {
            testview_panel.focus();
        } else if (reaction.tag === "testview_update") {
            testview_panel.update(reaction.tree);
        }
    };
    logic.recv(callback);

    logic.send({
        tag: "workspace_info",
        root_path: vscode.workspace.rootPath || null,
    });
}

export function deactivate() {
}

function register_trigger(command: string, impulse: native.Impulse, logic: native.Logic, context: vscode.ExtensionContext): void {
    context.subscriptions.push(vscode.commands.registerCommand(command, () => logic.send(impulse)));
}

interface ChannelRegister<T> {
    [id: string]: (x: T) => void;
}
