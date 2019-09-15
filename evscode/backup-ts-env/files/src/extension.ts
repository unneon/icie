'use strict';
import * as vscode from 'vscode';
import * as child_process from "child_process";
import * as fs from 'fs';
import * as os from 'os';
import TelemetryReporter from 'vscode-extension-telemetry';

export function activate(ctx: vscode.ExtensionContext) {
    let meta: {
        id: string;
        name: string;
        repository: string;
        commands: string[];
        telemetry: {
            extension_id: string;
            extension_version: string;
            instrumentation_key: string;
        };
    } = JSON.parse(fs.readFileSync(`${ctx.extensionPath}/data/meta.json`).toString());
    let telemetry = new TelemetryReporter(meta.telemetry.extension_id, meta.telemetry.extension_version, meta.telemetry.instrumentation_key);
    let crit = new critical.Critical(meta.name, meta.repository, telemetry);
    let logic: native.Logic;
    try {
        logic = new native.Logic(ctx.extensionPath, vscode.workspace.rootPath === undefined ? null : vscode.workspace.rootPath, crit);
    } catch {
        for (let command_id of meta.commands) {
            ctx.subscriptions.push(vscode.commands.registerCommand(command_id, () => {}));
        }
        return;
    }
    ctx.subscriptions.push({ dispose: () => logic.send({ tag: 'dispose' }) });
    let status = vscode.window.createStatusBarItem();
    let progresses = new progress.Register(logic);
    let webviews = new register.Register<webview.Panel>();
    let terminals = new register.Register<vscode.Terminal>();

    logic.send({
        tag: 'config',
        tree: vscode.workspace.getConfiguration(meta.id),
    });
    logic.send({
        tag: 'meta',
        extension: ctx.extensionPath,
        workspace: vscode.workspace.rootPath === undefined ? null : vscode.workspace.rootPath,
    });
    vscode.workspace.onDidChangeConfiguration(ev => {
        logic.send({
            tag: 'config',
            tree: vscode.workspace.getConfiguration(meta.id),
        });
    });

    for (let command_id of meta.commands) {
        ctx.subscriptions.push(vscode.commands.registerCommand(command_id, () => logic.send({ tag: 'trigger', command_id: command_id })));
    }

    let answer = function (aid: number, value: any): void {
        logic.send({
            tag: 'async',
            aid,
            value
        });
    };

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
                modal: reaction.modal
            };
            let libf = reaction.kind === 'info' ? vscode.window.showInformationMessage : reaction.kind === 'warning' ? vscode.window.showWarningMessage : vscode.window.showErrorMessage;
            libf(message, options, ...reaction.items).then(response => {
                logic.send({ tag: 'async', aid: reaction.aid, value: response === undefined ? null : response.id });
            });
        } else if (reaction.tag === "quick_pick") {
            vscode.window.showQuickPick(reaction.items.map(item => {
                return {
                    label: item.label,
                    description: item.description === null ? undefined : item.description,
                    detail: item.detail === null ? undefined : item.detail,
                    alwaysShow: item.alwaysShow,
                    id: item.id
                };
            }), {
                matchOnDescription: reaction.matchOnDescription,
                matchOnDetail: reaction.matchOnDetail,
                ignoreFocusOut: reaction.ignoreFocusOut,
                placeHolder: nullmap(reaction.placeholder)
            }).then(selected => {
                logic.send({ tag: 'async', aid: reaction.aid, value: selected === undefined ? null : selected.id });
            });
        } else if (reaction.tag === "input_box") {
            vscode.window.showInputBox({
                ignoreFocusOut: reaction.ignoreFocusOut,
                password: reaction.password,
                placeHolder: reaction.placeHolder || undefined,
                prompt: reaction.prompt || undefined,
                value: reaction.value || undefined,
                valueSelection: reaction.valueSelection || undefined
            }).then(value => {
                logic.send({ tag: 'async', aid: reaction.aid, value: value !== undefined ? value : null });
            });
        } else if (reaction.tag === "console") {
            if (reaction.level === 'debug') {
                console.debug(reaction.message);
            } else if (reaction.level === 'log') {
                console.log(reaction.message);
            } else if (reaction.level === 'info') {
                console.info(reaction.message);
            } else if (reaction.level === 'warn') {
                console.warn(reaction.message);
            } else if (reaction.level === 'error') {
                console.error(reaction.message);
            }
        } else if (reaction.tag === 'console_group') {
            (reaction.collapsed ? console.groupCollapsed : console.group)(nullmap(reaction.label));
        } else if (reaction.tag === 'console_group_end') {
            console.groupEnd();
        } else if (reaction.tag === "save_all") {
            vscode.workspace.saveAll(false).then(ret => {
                logic.send({ tag: 'async', aid: reaction.aid, value: ret });
            });
        } else if (reaction.tag === "open_folder") {
            vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(reaction.path), reaction.in_new_window);
        } else if (reaction.tag === "open_editor") {
            (async () => {
                let editor = null;
                if (!reaction.force_new) {
                    let old_editor = vscode.window.visibleTextEditors.find(ed => ed.document.fileName === reaction.path);
                    if (old_editor !== undefined) {
                        editor = old_editor;
                    }
                }
                if (editor === null) {
                    let source = await vscode.workspace.openTextDocument(reaction.path);
                    editor = await vscode.window.showTextDocument(source, {
                        preserveFocus: nullmap(reaction.preserve_focus),
                        preview: nullmap(reaction.preview),
                        selection: reaction.selection !== null ? convrange(reaction.selection) : undefined,
                        viewColumn: reaction.view_column !== null ? webview.convert_view_column(reaction.view_column) : undefined,
                    });
                }
                if (reaction.cursor !== null) {
                    let newPosition = convpos(reaction.cursor)
                    editor.selection = new vscode.Selection(newPosition, newPosition);
                    editor.revealRange(new vscode.Range(newPosition, newPosition), vscode.TextEditorRevealType.InCenter);
                }
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            })();
        } else if (reaction.tag === "progress_start") {
            progresses.start(reaction.hid, {
                title: reaction.title,
                location: reaction.location,
                cancellable: reaction.cancellable
            });
        } else if (reaction.tag === "progress_update") {
            progresses.update(reaction.hid, {
                increment: reaction.increment,
                message: reaction.message
            });
        } else if (reaction.tag === "progress_end") {
            progresses.finish(reaction.hid);
        } else if (reaction.tag === 'progress_register_cancel') {
            progresses.register_cancel(reaction.hid, reaction.aid);
        } else if (reaction.tag === "query_document_text") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                let contents = doc.getText(undefined);
                logic.send({ tag: 'async', aid: reaction.aid, value: contents });
            });
        } else if (reaction.tag === "edit_paste") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                vscode.window.showTextDocument(doc, undefined, true).then(edi => {
                    edi.edit(edit_builder => {
                        edit_builder.insert(new vscode.Position(reaction.position.line, reaction.position.character), reaction.text);
                    }, undefined).then(_ => {
                        logic.send({ tag: 'async', aid: reaction.aid, value: null });
                    });
                });
            });
        } else if (reaction.tag === 'webview_create') {
            webviews.create(reaction.hid, new webview.Panel(reaction, logic));
        } else if (reaction.tag === 'webview_set_html') {
            webviews.run(reaction.hid, w => w.set_html(reaction.html));
        } else if (reaction.tag === 'webview_post_message') {
            webviews.run(reaction.hid, w => w.post_message(reaction.message));
        } else if (reaction.tag === 'webview_register_listener') {
            webviews.run(reaction.hid, w => w.register_listener(reaction.aid));
        } else if (reaction.tag === 'webview_register_disposer') {
            webviews.run(reaction.hid, w => w.register_disposer(reaction.aid));
        } else if (reaction.tag === 'webview_was_disposed') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.was_disposed(), true));
        } else if (reaction.tag === 'webview_reveal') {
            webviews.run(reaction.hid, w => w.reveal(reaction.view_column, reaction.preserve_focus));
        } else if (reaction.tag === 'webview_dispose') {
            webviews.run(reaction.hid, w => w.dispose());
        } else if (reaction.tag === 'webview_is_visible') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.is_visible(), false));
        } else if (reaction.tag === 'webview_is_active') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.is_active(), false));
        } else if (reaction.tag === 'reaction_memento_get') {
            let memento = reaction.dst === 'workspace' ? ctx.workspaceState : ctx.globalState;
            let value = memento.get<any>(reaction.key);
            let found = value !== undefined;
            if (value === undefined) {
                value = null;
            }
            logic.send({ tag: 'async', aid: reaction.aid, value: { value, found }});
        } else if (reaction.tag === 'reaction_memento_set') {
            let memento = reaction.dst === 'workspace' ? ctx.workspaceState : ctx.globalState;
            memento.update(reaction.key, reaction.val);
        } else if (reaction.tag === 'active_editor_file') {
            let editor = vscode.window.activeTextEditor;
            if (editor !== undefined) {
                logic.send({ tag: 'async', aid: reaction.aid, value: editor.document.fileName });
            } else {
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            }
        } else if (reaction.tag === 'clipboard_write') {
            vscode.env.clipboard.writeText(reaction.val).then(() => {
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            });
        } else if (reaction.tag === 'terminal_create') {
            terminals.create(reaction.hid, vscode.window.createTerminal({
                cwd: nullmap(reaction.cwd),
                env: nullmap(reaction.env),
                name: nullmap(reaction.name),
                shellArgs: nullmap(reaction.shellArgs),
                shellPath: nullmap(reaction.shellPath)
            }));
        } else if (reaction.tag === 'terminal_write') {
            terminals.run(reaction.hid, t => t.sendText(reaction.text, reaction.addNewLine));
        } else if (reaction.tag === 'terminal_show') {
            terminals.run(reaction.hid, t => t.show(reaction.preserveFocus));
        } else if (reaction.tag === 'open_dialog') {
            let options: vscode.OpenDialogOptions = {
                canSelectFiles: reaction.canSelectFiles,
                canSelectFolders: reaction.canSelectFolders,
                canSelectMany: reaction.canSelectMany,
                defaultUri: reaction.defaultFile !== null ? vscode.Uri.file(reaction.defaultFile) : undefined,
                filters: nullmap(reaction.filters),
                openLabel: nullmap(reaction.openLabel),
            };
            vscode.window.showOpenDialog(options).then(uris => {
                if (uris !== undefined) {
                    logic.send({ tag: 'async', aid: reaction.aid, value: uris.map(uri => uri.path) });
                } else {
                    logic.send({ tag: 'async', aid: reaction.aid, value: null });
                }
            });
        } else if (reaction.tag === 'open_external') {
            vscode.env.openExternal(vscode.Uri.parse(reaction.url)).then(success => {
                logic.send({ tag: 'async', aid: reaction.aid, value: success });
            })
        } else if (reaction.tag === 'telemetry_event') {
            telemetry.sendTelemetryEvent(reaction.event_name, reaction.properties, reaction.measurements);
        } else if (reaction.tag === 'kill') {
            telemetry.dispose();
            logic.kill();
        }
    };
    logic.recv(callback);
}

function nullmap<T>(x: T | null): T | undefined {
    return x === null ? undefined : x;
}
function convpos(x: native.Position2): vscode.Position {
    return new vscode.Position(x.line, x.column);
}
function convrange(x: native.Range2): vscode.Range {
    return new vscode.Range(convpos(x.start), convpos(x.end));
}

export function deactivate() {
}

namespace channel {

    export class Channel<T> {
        private tx: null | (() => void);
        private buffer: T[];
        public constructor() {
            this.tx = null;
            this.buffer = [];
        }
        public wake(): Promise<void> {
            if (this.buffer.length > 0) {
                return Promise.resolve();
            } else {
                return this.reset();
            }
        }
        public recv(): T {
            let value = this.buffer.shift();
            if (value === undefined) {
                unreachable('Channel.recv.(value === undefined)');
                throw new Error;
            } else {
                return value;
            }
        }
        public send(x: T): void {
            this.buffer.push(x);
            if (this.tx !== null) {
                let tx = this.tx;
                this.tx = null;
                tx();
            }
        }
        private reset(): Promise<void> {
            return new Promise<void>(resolve => {
                this.tx = resolve;
            });
        }
    }

    function unreachable(site: string): never {
        console.error(`UNREACHABLE ${site}`);
        throw new Error;
    }

}

namespace native {

    export interface QuickPickItem {
        label: string;
        description: string | null;
        detail: string | null;
        id: string;
        alwaysShow: boolean;
    }
    export type MessageKind = 'info' | 'warning' | 'error';
    export interface MessageItem {
        title: string;
        isCloseAffordance: boolean;
        id: string;
    }
    export interface Position {
        line: number;
        character: number;
    }
    export interface Position2 {
        column: number;
        line: number;
    }
    export interface Range2 {
        start: Position2;
        end: Position2;
    }

    export interface ImpulseTrigger {
        tag: "trigger";
        command_id: string;
    }
    export interface ImpulseAsync {
        tag: "async";
        aid: number;
        value: any;
    }
    export interface ImpulseConfig {
        tag: "config";
        tree: any;
    }
    export interface ImpulseMeta {
        tag: "meta";
        workspace: string | null;
        extension: string;
    }
    export interface ImpulseDispose {
        tag: "dispose";
    }

    export interface ReactionStatus {
        tag: "status";
        message: string | null;
    }
    export interface ReactionMessage {
        tag: "message";
        message: string;
        kind: MessageKind;
        items: MessageItem[];
        modal: boolean;
        aid: number;
    }
    export interface ReactionQuickPick {
        tag: "quick_pick";
        items: QuickPickItem[];
        matchOnDescription: boolean;
        matchOnDetail: boolean;
        ignoreFocusOut: boolean;
        placeholder: string | null;
        aid: number;
    }
    export interface ReactionInputBox {
        tag: "input_box";
        prompt: string | null;
        placeHolder: string | null;
        password: boolean;
        ignoreFocusOut: boolean;
        value: string | null;
        valueSelection: [number, number] | null;
        aid: number;
    }
    export interface ReactionSaveAll {
        tag: "save_all";
        aid: number;
    }
    export interface ReactionOpenFolder {
        tag: "open_folder";
        path: string;
        in_new_window: boolean;
    }
    export interface ReactionConsole {
        tag: "console";
        level: "debug" | "log" | "info" | "warn" | "error";
        message: string;
    }
    export interface ReactionOpenEditor {
        tag: "open_editor";
        path: string;
        cursor: Position2 | null,
        preserve_focus: boolean | null;
        preview: boolean | null;
        selection: Range2 | null;
        view_column: WebviewViewColumn | null;
        force_new: boolean;
        aid: number;
    }
    export interface ReactionProgressStart {
        tag: "progress_start";
        hid: string;
        title: string | null;
        location: progress.Location;
        cancellable: boolean;
    }
    export interface ReactionProgressUpdate {
        tag: "progress_update";
        hid: string;
        increment: number | null;
        message: string | null;
    }
    export interface ReactionProgressRegisterCancel {
        tag: "progress_register_cancel";
        hid: string;
        aid: number;
    }
    export interface ReactionProgressEnd {
        tag: "progress_end";
        hid: string;
    }
    export interface ReactionQueryDocumentText {
        tag: 'query_document_text';
        path: string;
        aid: number;
    }
    export interface ReactionPasteEdit {
        tag: 'edit_paste';
        position: Position;
        text: string;
        path: string;
        aid: number;
    }
    export type WebviewViewColumn = 'active' | 'beside' | 'eight' | 'five' | 'four' | 'nine' | 'one' | 'seven' | 'six' | 'three' | 'two';
    export interface ReactionWebviewCreate {
        tag: 'webview_create';
        view_type: string;
        title: string;
        view_column: WebviewViewColumn;
        preserve_focus: boolean;
        enable_command_uris: boolean;
        enable_scripts: boolean;
        local_resource_roots: string[] | null;
        enable_find_widget: boolean;
        retain_context_when_hidden: boolean;
        hid: number;
    }
    export interface ReactionWebviewSetHTML {
        tag: 'webview_set_html';
        hid: number;
        html: string;
    }
    export interface ReactionWebviewPostMessage {
        tag: 'webview_post_message';
        hid: number;
        message: any;
    }
    export interface ReactionWebviewRegisterListener {
        tag: 'webview_register_listener';
        hid: number;
        aid: number;
    }
    export interface ReactionWebviewRegisterDisposer {
        tag: 'webview_register_disposer';
        hid: number;
        aid: number;
    }
    export interface ReactionWebviewWasDisposed {
        tag: 'webview_was_disposed';
        hid: number;
        aid: number;
    }
    export interface ReactionWebviewReveal {
        tag: 'webview_reveal';
        hid: number;
        view_column: WebviewViewColumn;
        preserve_focus: boolean;
    }
    export interface ReactionWebviewDispose {
        tag: 'webview_dispose';
        hid: number;
    }
    export interface ReactionWebviewIsVisible {
        tag: 'webview_is_visible';
        hid: number;
        aid: number;
    }
    export type MementoDest = 'workspace' | 'global';
    export interface ReactionMementoGet {
        tag: 'reaction_memento_get';
        aid: number;
        key: string;
        dst: MementoDest;
    }
    export interface ReactionMementoSet {
        tag: 'reaction_memento_set';
        key: string;
        val: any;
        dst: MementoDest;
    }
    export interface ReactionActiveEditorFile {
        tag: 'active_editor_file';
        aid: number;
    }
    export interface ReactionWebviewIsActive {
        tag: 'webview_is_active';
        hid: number;
        aid: number;
    }
    export interface ReactionClipboardWrite {
        tag: 'clipboard_write';
        aid: number;
        val: string;
    }
    export interface ReactionTerminalCreate {
        tag: 'terminal_create';
        hid: number;
        cwd: string | null;
        env: any | null;
        name: string | null;
        shellArgs: string[] | null;
        shellPath: string | null;
    }
    export interface ReactionTerminalWrite {
        tag: 'terminal_write';
        hid: number;
        text: string;
        addNewLine: boolean;
    }
    export interface ReactionTerminalShow {
        tag: 'terminal_show';
        hid: number;
        preserveFocus: boolean;
    }
    export interface ReactionOpenDialog {
        tag: 'open_dialog';
        canSelectFiles: boolean;
        canSelectFolders: boolean;
        canSelectMany: boolean;
        defaultFile: string | null;
        filters: { [name: string]: (string[]) };
        openLabel: string | null;
        aid: number;
    }
    export interface ReactionConsoleGroup {
        tag: 'console_group';
        collapsed: boolean;
        label: string | null;
    }
    export interface ReactionConsoleGroupEnd {
        tag: 'console_group_end';
    }
    export interface ReactionOpenExternal {
        tag: 'open_external';
        url: string;
        aid: number;
    }
    export interface ReactionTelemetryEvent {
        tag: 'telemetry_event';
        event_name: string;
        properties: { [key: string]: string };
        measurements: { [key: string]: number };
    }
    export interface ReactionKill {
        tag: 'kill';
    }

    export type Impulse = ImpulseTrigger | ImpulseAsync | ImpulseConfig | ImpulseMeta | ImpulseDispose;
    export type Reaction = ReactionStatus | ReactionMessage | ReactionQuickPick | ReactionInputBox | ReactionConsole | ReactionSaveAll | ReactionOpenFolder | ReactionOpenEditor | ReactionProgressStart | ReactionProgressUpdate | ReactionProgressRegisterCancel | ReactionProgressEnd | ReactionQueryDocumentText | ReactionPasteEdit | ReactionWebviewCreate | ReactionWebviewSetHTML | ReactionWebviewPostMessage | ReactionWebviewRegisterListener | ReactionWebviewRegisterDisposer | ReactionWebviewWasDisposed | ReactionWebviewReveal | ReactionWebviewDispose | ReactionWebviewIsVisible | ReactionMementoSet | ReactionMementoGet | ReactionActiveEditorFile | ReactionWebviewIsActive | ReactionClipboardWrite | ReactionTerminalCreate | ReactionTerminalWrite | ReactionTerminalShow | ReactionOpenDialog | ReactionConsoleGroup | ReactionConsoleGroupEnd | ReactionOpenExternal | ReactionTelemetryEvent | ReactionKill;

    export class Logic {
        path: string;
        kid: child_process.ChildProcess;
        parser: multijson.Parser<Reaction>;
        crit: critical.Critical;
        killed: boolean;
        constructor(extensionPath: string, workspacePath: string | null, crit: critical.Critical) {
            this.crit = crit;
            this.killed = false;
            this.parser = new multijson.Parser<Reaction>();
            process.env.RUST_BACKTRACE = '1';
            if (os.platform() === 'linux') {
                this.path = `${extensionPath}/data/bin/linux`;
            } else {
                throw this.crit.os_support();
            }
            this.kid = child_process.spawn(this.path, ['--extension'], {
                cwd: workspacePath !== null ? workspacePath : extensionPath
            });
            let stderr_buf = '';
            if (this.kid.stderr !== null) {
                this.kid.stderr.on('data', chunk => {
                    if (typeof chunk === 'object') {
                        chunk = chunk.toString();
                    }
                    stderr_buf += chunk;
                });
            }
            this.kid.on('exit', (code, signal) => {
                if (!this.killed) {
                    throw this.crit.error(`the extension process crashed with exit code ${code}`, stderr_buf);
                }
            });
        }
        send(impulse: Impulse) {
            // console.log(`  --> ${JSON.stringify(impulse)}`);
            if (this.kid.stdin === null) {
                throw this.crit.error('an unexpected internal error has occured', 'this.kid.stdin === null');
            }
            this.kid.stdin.write(`${JSON.stringify(impulse)}\n`);
        }
        recv(callback: (reaction: Reaction) => void) {
            if (this.kid.stdout === null) {
                throw this.crit.error('an unexpected internal error has occured', 'this.kid.stdout === null');
            }
            this.kid.stdout.on('data', chunk => {
                if (typeof chunk === 'string') {
                    chunk = Buffer.from(chunk);
                }
                this.parser.write(chunk);
                for (let reaction of this.parser.read()) {
                    // console.log(`<--   ${JSON.stringify(reaction)}`);
                    callback(reaction);
                }
            });
        }
        kill() {
            this.killed = true;
            this.kid.kill('SIGKILL');
        }
    }

}

namespace critical {

    export class Critical {
        name: string;
        repository: string;
        telemetry: TelemetryReporter;
        public constructor(name: string, repository: string, telemetry: TelemetryReporter) {
            this.name = name;
            this.repository = repository;
            this.telemetry = telemetry;
        }
        public error(message: string, extended_log: string): Error {
            this.telemetry.sendTelemetryException(new Error(`${this.name} critical error, ${message}, ${extended_log}`), {}, {});
            this.telemetry.dispose();
            let repo_uri = vscode.Uri.parse(this.repository);
            let issues_uri = vscode.Uri.parse(`${this.repository}/issues`);
            let fmt = `${this.name} has encountered a critical error: ${message}. Please report this issue at [${repo_uri.authority}${repo_uri.path}](${issues_uri}), the logs are at Help > Toggle Developer Tools (Ctrl+Shift+I) > Console.`;
            let fmt_long = `${fmt}\n\n${extended_log}`;
            vscode.window.showErrorMessage(fmt, 'Copy logs and open issue tracker').then(response => {
                if (response !== undefined) {
                    vscode.env.clipboard.writeText(fmt_long).then(_ => {
                        vscode.env.openExternal(issues_uri);
                    });
                }
            });
            return new Error(fmt_long);
        }
        public os_support(): Error {
            let platform = os.platform();
            let short_msg = `OS ${JSON.stringify(platform)} is not supported on <0.7`;
            let user_os = platform === 'win32' ? 'Windows' : platform === 'darwin' ? 'MacOS' : 'Windows/MacOS/...';
            let user_msg = `Sorry :(, ${user_os} support will come in the 0.7 release, likely in October 2019. Please check out ICIE 0.7 once it comes out, or try it out on Linux now!`;
            this.telemetry.sendTelemetryException(new Error(short_msg), {}, {});
            this.telemetry.dispose();
            vscode.window.showErrorMessage(user_msg);
            return new Error(short_msg);
        }
    }

}

namespace multijson {

    export class Parser<T> {
        buffer: Buffer;
        constructor() {
            this.buffer = Buffer.alloc(0);
        }
        write(chunk: Buffer) {
            this.buffer = Buffer.concat([this.buffer, chunk]);
        }
        read(): T[] {
            let objs: T[] = [];
            let last = 0;
            while (true) {
                let pos = this.buffer.indexOf('\n', last);
                if (pos === -1) {
                    break;
                }
                let sub = this.buffer.slice(last, pos);
                last = pos + 1;
                let obj = JSON.parse(sub.toString());
                objs.push(obj);
            }
            this.buffer = this.buffer.slice(last);
            return objs;
        }
    }

}

namespace progress {

    export type HID = string;
    export type AID = number;
    export type Location = 'notification' | 'source_control' | 'window';
    export interface Options {
        title: string | null;
        location: Location;
        cancellable: boolean;
    }
    export interface Update {
        increment: number | null;
        message: string | null;
    }

    export class Register {
        private logic: native.Logic;
        private cancel_aids: Map<HID, AID | 'premature'>;
        private channels: Map<HID, channel.Channel<Update | 'finish' | 'cancel'>>;
        public constructor(logic: native.Logic) {
            this.logic = logic;
            this.cancel_aids = new Map;
            this.channels = new Map;
        }
        public start(hid: HID, options: Options): void {
            let comms = new channel.Channel<Update | 'finish' | 'cancel'>();
            this.channels.set(hid, comms);
            vscode.window.withProgress({
                cancellable: options.cancellable,
                location: this.convert_location(options.location),
                title: options.title !== null ? options.title : undefined,
            }, async (progress_handle, cancel_token) => {
                cancel_token.onCancellationRequested(_ => {
                    comms.send('cancel');
                });
                while (true) {
                    await comms.wake();
                    let event = comms.recv();
                    if (event === 'cancel') {
                        let aid = this.cancel_aids.get(hid);
                        if (aid === 'premature') {
                            // unreachable
                        } else if (aid !== undefined) {
                            this.logic.send({ tag: 'async', aid, value: null });
                        } else {
                            this.cancel_aids.set(hid, 'premature');
                        }
                        break;
                    } else if (event === 'finish') {
                        break;
                    } else {
                        progress_handle.report({
                            increment: event.increment !== null ? event.increment : undefined,
                            message: event.message !== null ? event.message : undefined
                        });
                    }
                }
                if (this.cancel_aids.get(hid) !== 'premature') {
                    this.cancel_aids.delete(hid);
                }
                this.channels.delete(hid);
            });
        }
        public update(hid: HID, update: Update): void {
            let comms = this.channels.get(hid);
            if (comms !== undefined) {
                comms.send(update);
            }
        }
        public finish(hid: HID): void {
            let comms = this.channels.get(hid);
            if (comms !== undefined) {
                comms.send('finish');
            }
        }
        public register_cancel(hid: HID, aid: AID): void {
            if (this.channels.has(hid)) {
                this.cancel_aids.set(hid, aid);
            } else if (this.cancel_aids.get(hid) === 'premature') {
                this.logic.send({ tag: 'async', aid, value: null });
                this.cancel_aids.delete(hid);
            }
        }
        private convert_location(location: Location): vscode.ProgressLocation {
            if (location === 'notification') {
                return vscode.ProgressLocation.Notification;
            } else if (location === 'source_control') {
                return vscode.ProgressLocation.SourceControl;
            } else if (location === 'window') {
                return vscode.ProgressLocation.Window;
            } else {
                throw new Error(`unrecognized progress location ${JSON.stringify(location)}`);
            }
        }
    }

}

namespace webview {

    export type HID = number;
    export type AID = number;

    export class AsyncBuffer<T> {
        private logic: native.Logic;
        private aid: AID | null;
        private buffer: T[];
        public constructor(logic: native.Logic) {
            this.logic = logic;
            this.aid = null;
            this.buffer = [];
        }
        public send(value: T): void {
            if (this.aid !== null) {
                this.logic.send({ tag: 'async', aid: this.aid, value });
            } else {
                this.buffer.push(value);
            }
        }
        public register(aid: AID): void {
            this.aid = aid;
            for (let value of this.buffer) {
                this.logic.send({ tag: 'async', aid, value });
            }
            this.buffer = [];
        }
    }

    export class Panel {
        private listener: AsyncBuffer<any>;
        private disposer: AsyncBuffer<void>;
        private panel: vscode.WebviewPanel | null;
        public constructor(options: native.ReactionWebviewCreate, logic: native.Logic) {
            this.listener = new AsyncBuffer<any>(logic);
            this.disposer = new AsyncBuffer<void>(logic);
            this.panel = vscode.window.createWebviewPanel(options.view_type, options.title, {
                preserveFocus: options.preserve_focus,
                viewColumn: convert_view_column(options.view_column),
            }, {
                    enableFindWidget: options.enable_find_widget,
                    retainContextWhenHidden: options.retain_context_when_hidden,
                    enableCommandUris: options.enable_command_uris,
                    enableScripts: options.enable_scripts,
                    localResourceRoots: options.local_resource_roots === null ? undefined : options.local_resource_roots.map(lrr => vscode.Uri.parse(lrr)),
                });
            this.panel.webview.onDidReceiveMessage(value => {
                this.listener.send(value);
            });
            this.panel.onDidDispose(_ => {
                this.disposer.send();
                this.panel = null;
            });
        }
        public set_html(html: string): void {
            if (this.panel !== null) {
                this.panel.webview.html = html;
            }
        }
        public post_message(message: any): void {
            if (this.panel !== null) {
                this.panel.webview.postMessage(message);
            }
        }
        public register_listener(aid: AID): void {
            this.listener.register(aid);
        }
        public register_disposer(aid: AID): void {
            this.disposer.register(aid);
        }
        public was_disposed(): boolean {
            return this.panel === null;
        }
        public is_visible(): boolean {
            return this.panel !== null && this.panel.visible;
        }
        public reveal(view_column: native.WebviewViewColumn, preserve_focus: boolean): void {
            if (this.panel !== null) {
                this.panel.reveal(convert_view_column(view_column), preserve_focus);
            }
        }
        public dispose(): void {
            if (this.panel !== null) {
                this.panel.dispose();
            }
        }
        public is_active(): boolean {
            return this.panel !== null && this.panel.active;
        }
    }

    export function convert_view_column(col: native.WebviewViewColumn): vscode.ViewColumn {
        if (col === 'active') {
            return vscode.ViewColumn.Active;
        } else if (col === 'beside') {
            return vscode.ViewColumn.Beside;
        } else if (col === 'eight') {
            return vscode.ViewColumn.Eight;
        } else if (col === 'five') {
            return vscode.ViewColumn.Five;
        } else if (col === 'four') {
            return vscode.ViewColumn.Four;
        } else if (col === 'nine') {
            return vscode.ViewColumn.Nine;
        } else if (col === 'one') {
            return vscode.ViewColumn.One;
        } else if (col === 'seven') {
            return vscode.ViewColumn.Seven;
        } else if (col === 'six') {
            return vscode.ViewColumn.Six;
        } else if (col === 'three') {
            return vscode.ViewColumn.Three;
        } else if (col === 'two') {
            return vscode.ViewColumn.Two;
        } else {
            throw new Error('unrecognized view column');
        }
    }

}

namespace register {

    export type HID = number;

    export class Register<T> {
        private store: Map<HID, T>;
        public constructor() {
            this.store = new Map;
        }
        public create(hid: HID, obj: T): void {
            this.store.set(hid, obj);
        }
        public run<R>(hid: HID, f: (obj: T) => R): void {
            let obj = this.store.get(hid);
            if (obj !== undefined) {
                f(obj);
            }
        }
        public query<R1, R2>(hid: HID, f: (obj: T) => R1, def: R2): R1 | R2 {
            let obj = this.store.get(hid);
            if (obj !== undefined) {
                return f(obj);
            } else {
                return def;
            }
        }
    }

}
