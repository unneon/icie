'use strict';
import * as vscode from 'vscode';
import * as channel from './channel';
import * as native from './native';
import * as testview from './testview';
import * as discoverer from './discoverer';

interface ProgressUpdate {
    reaction: native.ReactionProgressUpdate | native.ReactionProgressEnd;
    recv: Promise<ProgressUpdate>;
}

export function activate(context: vscode.ExtensionContext) {
    let logic = new native.Logic(context.extensionPath);
    context.subscriptions.push(new vscode.Disposable(() => logic.kill()));
    let status = vscode.window.createStatusBarItem();
    let progressRegister: ChannelRegister<ProgressUpdate> = {};
    let testview_panel = new testview.Panel(context.extensionPath, notes => logic.send(notes));
    let discoverer_panel = new discoverer.Panel(context.extensionPath, notes => logic.send(notes));

    register_trigger('icie.build', { tag: "trigger_build" }, logic, context);
    register_trigger('icie.test', { tag: "trigger_test" }, logic, context);
    register_trigger('icie.init', { tag: "trigger_init" }, logic, context);
    register_trigger('icie.submit', { tag: "trigger_submit" }, logic, context);
    register_trigger('icie.manual.submit', { tag: "trigger_manual_submit" }, logic, context);
    register_trigger('icie.template.instantiate', { tag: "trigger_template_instantiate" }, logic, context);
    register_trigger('icie.test.view', { tag: "trigger_testview" }, logic, context);
    register_trigger('icie.test.discoverer', { tag: "trigger_multitest_view" }, logic, context);
    register_trigger('icie.paste.pick', { tag: "trigger_paste_pick" }, logic, context);
    register_trigger('icie.external.terminal', { tag: "trigger_terminal" }, logic, context);
    register_trigger('icie.init.existing', { tag: "trigger_init_existing" }, logic, context);
    register_trigger('icie.paste.qistruct', { tag: "trigger_qistruct" }, logic, context);
    // context.subscriptions.push(vscode.commands.registerCommand('icie.paste.pick', async () => {
    //     console.log(`Hello!`);
    //     let doc = await vscode.workspace.openTextDocument(vscode.Uri.file(`${vscode.workspace.rootPath}/main.cpp`));
    //     let text = doc.getText(undefined);
    //     console.log(text);
    // }));
    context.subscriptions.push(vscode.commands.registerCommand('icie.test.new', () => {
        // TODO make it work even if the panel is not open
        if (testview_panel.is_created()) {
            testview_panel.start_new_test();
        }
    }));

    let callback = (reaction: native.Reaction) => {
        if (reaction.tag === "status") {
            if (reaction.message !== null) {
                status.text = `${reaction.message}`;
                status.show();
            } else {
                status.hide();
            }
        } else if (reaction.tag === "message") {
            let message = reaction.message;
            let options = {
                modal: reaction.modal === null ? undefined : reaction.modal
            };
            if (reaction.items === null) {
                if (reaction.kind === 'info') {
                    vscode.window.showInformationMessage(message, options);
                } else if (reaction.kind === 'warning') {
                    vscode.window.showWarningMessage(message, options);
                } else if (reaction.kind === 'error') {
                    vscode.window.showErrorMessage(message, options);
                }
            } else {
                let question_id = reaction.items.id;
                let items = reaction.items.list.map(mi => { return {
                    title: mi.title,
                    isCloseAffordance: mi.is_close_affordance === null ? undefined : mi.is_close_affordance,
                    id: mi.id
                }; });
                let fut = null;
                if (reaction.kind === 'info') {
                    fut = vscode.window.showInformationMessage(message, options, ...items);
                } else if (reaction.kind === 'warning') {
                    fut = vscode.window.showWarningMessage(message, options, ...items);
                } else if (reaction.kind === 'error') {
                    fut = vscode.window.showErrorMessage(message, options, ...items);
                } else {
                    throw new Error("uknown message kind");
                }
                fut.then(item => {
                    if (item !== undefined) {
                        logic.send({ tag: 'message_response', id: question_id, response: item.id });
                    } else {
                        logic.send({ tag: 'message_response', id: question_id, response: null });
                    }
                });
            }
        } else if (reaction.tag === "quick_pick") {
            vscode.window.showQuickPick(reaction.items.map(item => {
                return {
                    description: item.description || undefined,
                    detail: item.detail || undefined,
                    label: item.label,
                    id: item.id
                };
            }), { matchOnDescription: reaction.matchOnDescription, matchOnDetail: reaction.matchOnDetail }).then(item => {
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
                prompt: reaction.prompt || undefined,
                value: reaction.value || undefined,
                valueSelection: reaction.valueSelection || undefined
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
                .then(source => {
                    console.log(`source = ${JSON.stringify(source)}`);
                    return vscode.window.showTextDocument(source);
                })
                .then(editor => {
                    let oldPosition = editor.selection.active;
                    let newPosition = oldPosition.with(reaction.row-1, reaction.column-1);
                    let newSelection = new vscode.Selection(newPosition, newPosition);
                    editor.selection = newSelection;
                    editor.revealRange(new vscode.Range(newPosition, newPosition), vscode.TextEditorRevealType.InCenter);
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
        } else if (reaction.tag === "multitest_view_focus") {
            discoverer_panel.update({});
            discoverer_panel.focus();
        } else if (reaction.tag === "discovery_row" || reaction.tag === "discovery_state") {
            discoverer_panel.react(reaction);
        } else if (reaction.tag === "query_document_text") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                let contents = doc.getText(undefined);
                logic.send({ tag: 'document_text', contents });
            });
        } else if (reaction.tag === "edit_paste") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                vscode.window.showTextDocument(doc, undefined, true).then(edi => {
                    edi.edit(edit_builder => {
                        edit_builder.insert(new vscode.Position(reaction.position.line, reaction.position.character), reaction.text);
                    }, undefined).then(_ => {
                        logic.send({ tag: 'acknowledge_edit' });
                    });
                });
            });
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
