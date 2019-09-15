'use strict';
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : new P(function (resolve) { resolve(result.value); }).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
const vscode = require("vscode");
const child_process = require("child_process");
const fs = require("fs");
const os = require("os");
const vscode_extension_telemetry_1 = require("vscode-extension-telemetry");
function activate(ctx) {
    let meta = JSON.parse(fs.readFileSync(`${ctx.extensionPath}/data/meta.json`).toString());
    let telemetry = new vscode_extension_telemetry_1.default(meta.telemetry.extension_id, meta.telemetry.extension_version, meta.telemetry.instrumentation_key);
    let crit = new critical.Critical(meta.name, meta.repository, telemetry);
    let logic;
    try {
        logic = new native.Logic(ctx.extensionPath, vscode.workspace.rootPath === undefined ? null : vscode.workspace.rootPath, crit);
    }
    catch (_a) {
        for (let command_id of meta.commands) {
            ctx.subscriptions.push(vscode.commands.registerCommand(command_id, () => { }));
        }
        return;
    }
    ctx.subscriptions.push({ dispose: () => logic.send({ tag: 'dispose' }) });
    let status = vscode.window.createStatusBarItem();
    let progresses = new progress.Register(logic);
    let webviews = new register.Register();
    let terminals = new register.Register();
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
    let answer = function (aid, value) {
        logic.send({
            tag: 'async',
            aid,
            value
        });
    };
    let callback = (reaction) => {
        if (reaction.tag === "status") {
            if (reaction.message !== null) {
                status.text = `${reaction.message}`;
                status.show();
            }
            else {
                status.hide();
            }
        }
        else if (reaction.tag === "message") {
            let message = reaction.message;
            let options = {
                modal: reaction.modal
            };
            let libf = reaction.kind === 'info' ? vscode.window.showInformationMessage : reaction.kind === 'warning' ? vscode.window.showWarningMessage : vscode.window.showErrorMessage;
            libf(message, options, ...reaction.items).then(response => {
                logic.send({ tag: 'async', aid: reaction.aid, value: response === undefined ? null : response.id });
            });
        }
        else if (reaction.tag === "quick_pick") {
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
        }
        else if (reaction.tag === "input_box") {
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
        }
        else if (reaction.tag === "console") {
            if (reaction.level === 'debug') {
                console.debug(reaction.message);
            }
            else if (reaction.level === 'log') {
                console.log(reaction.message);
            }
            else if (reaction.level === 'info') {
                console.info(reaction.message);
            }
            else if (reaction.level === 'warn') {
                console.warn(reaction.message);
            }
            else if (reaction.level === 'error') {
                console.error(reaction.message);
            }
        }
        else if (reaction.tag === 'console_group') {
            (reaction.collapsed ? console.groupCollapsed : console.group)(nullmap(reaction.label));
        }
        else if (reaction.tag === 'console_group_end') {
            console.groupEnd();
        }
        else if (reaction.tag === "save_all") {
            vscode.workspace.saveAll(false).then(ret => {
                logic.send({ tag: 'async', aid: reaction.aid, value: ret });
            });
        }
        else if (reaction.tag === "open_folder") {
            vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(reaction.path), reaction.in_new_window);
        }
        else if (reaction.tag === "open_editor") {
            (() => __awaiter(this, void 0, void 0, function* () {
                let editor = null;
                if (!reaction.force_new) {
                    let old_editor = vscode.window.visibleTextEditors.find(ed => ed.document.fileName === reaction.path);
                    if (old_editor !== undefined) {
                        editor = old_editor;
                    }
                }
                if (editor === null) {
                    let source = yield vscode.workspace.openTextDocument(reaction.path);
                    editor = yield vscode.window.showTextDocument(source, {
                        preserveFocus: nullmap(reaction.preserve_focus),
                        preview: nullmap(reaction.preview),
                        selection: reaction.selection !== null ? convrange(reaction.selection) : undefined,
                        viewColumn: reaction.view_column !== null ? webview.convert_view_column(reaction.view_column) : undefined,
                    });
                }
                if (reaction.cursor !== null) {
                    let newPosition = convpos(reaction.cursor);
                    editor.selection = new vscode.Selection(newPosition, newPosition);
                    editor.revealRange(new vscode.Range(newPosition, newPosition), vscode.TextEditorRevealType.InCenter);
                }
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            }))();
        }
        else if (reaction.tag === "progress_start") {
            progresses.start(reaction.hid, {
                title: reaction.title,
                location: reaction.location,
                cancellable: reaction.cancellable
            });
        }
        else if (reaction.tag === "progress_update") {
            progresses.update(reaction.hid, {
                increment: reaction.increment,
                message: reaction.message
            });
        }
        else if (reaction.tag === "progress_end") {
            progresses.finish(reaction.hid);
        }
        else if (reaction.tag === 'progress_register_cancel') {
            progresses.register_cancel(reaction.hid, reaction.aid);
        }
        else if (reaction.tag === "query_document_text") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                let contents = doc.getText(undefined);
                logic.send({ tag: 'async', aid: reaction.aid, value: contents });
            });
        }
        else if (reaction.tag === "edit_paste") {
            vscode.workspace.openTextDocument(vscode.Uri.file(reaction.path)).then(doc => {
                vscode.window.showTextDocument(doc, undefined, true).then(edi => {
                    edi.edit(edit_builder => {
                        edit_builder.insert(new vscode.Position(reaction.position.line, reaction.position.character), reaction.text);
                    }, undefined).then(_ => {
                        logic.send({ tag: 'async', aid: reaction.aid, value: null });
                    });
                });
            });
        }
        else if (reaction.tag === 'webview_create') {
            webviews.create(reaction.hid, new webview.Panel(reaction, logic));
        }
        else if (reaction.tag === 'webview_set_html') {
            webviews.run(reaction.hid, w => w.set_html(reaction.html));
        }
        else if (reaction.tag === 'webview_post_message') {
            webviews.run(reaction.hid, w => w.post_message(reaction.message));
        }
        else if (reaction.tag === 'webview_register_listener') {
            webviews.run(reaction.hid, w => w.register_listener(reaction.aid));
        }
        else if (reaction.tag === 'webview_register_disposer') {
            webviews.run(reaction.hid, w => w.register_disposer(reaction.aid));
        }
        else if (reaction.tag === 'webview_was_disposed') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.was_disposed(), true));
        }
        else if (reaction.tag === 'webview_reveal') {
            webviews.run(reaction.hid, w => w.reveal(reaction.view_column, reaction.preserve_focus));
        }
        else if (reaction.tag === 'webview_dispose') {
            webviews.run(reaction.hid, w => w.dispose());
        }
        else if (reaction.tag === 'webview_is_visible') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.is_visible(), false));
        }
        else if (reaction.tag === 'webview_is_active') {
            answer(reaction.aid, webviews.query(reaction.hid, w => w.is_active(), false));
        }
        else if (reaction.tag === 'reaction_memento_get') {
            let memento = reaction.dst === 'workspace' ? ctx.workspaceState : ctx.globalState;
            let value = memento.get(reaction.key);
            let found = value !== undefined;
            if (value === undefined) {
                value = null;
            }
            logic.send({ tag: 'async', aid: reaction.aid, value: { value, found } });
        }
        else if (reaction.tag === 'reaction_memento_set') {
            let memento = reaction.dst === 'workspace' ? ctx.workspaceState : ctx.globalState;
            memento.update(reaction.key, reaction.val);
        }
        else if (reaction.tag === 'active_editor_file') {
            let editor = vscode.window.activeTextEditor;
            if (editor !== undefined) {
                logic.send({ tag: 'async', aid: reaction.aid, value: editor.document.fileName });
            }
            else {
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            }
        }
        else if (reaction.tag === 'clipboard_write') {
            vscode.env.clipboard.writeText(reaction.val).then(() => {
                logic.send({ tag: 'async', aid: reaction.aid, value: null });
            });
        }
        else if (reaction.tag === 'terminal_create') {
            terminals.create(reaction.hid, vscode.window.createTerminal({
                cwd: nullmap(reaction.cwd),
                env: nullmap(reaction.env),
                name: nullmap(reaction.name),
                shellArgs: nullmap(reaction.shellArgs),
                shellPath: nullmap(reaction.shellPath)
            }));
        }
        else if (reaction.tag === 'terminal_write') {
            terminals.run(reaction.hid, t => t.sendText(reaction.text, reaction.addNewLine));
        }
        else if (reaction.tag === 'terminal_show') {
            terminals.run(reaction.hid, t => t.show(reaction.preserveFocus));
        }
        else if (reaction.tag === 'open_dialog') {
            let options = {
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
                }
                else {
                    logic.send({ tag: 'async', aid: reaction.aid, value: null });
                }
            });
        }
        else if (reaction.tag === 'open_external') {
            vscode.env.openExternal(vscode.Uri.parse(reaction.url)).then(success => {
                logic.send({ tag: 'async', aid: reaction.aid, value: success });
            });
        }
        else if (reaction.tag === 'telemetry_event') {
            telemetry.sendTelemetryEvent(reaction.event_name, reaction.properties, reaction.measurements);
        }
        else if (reaction.tag === 'kill') {
            telemetry.dispose();
            logic.kill();
        }
    };
    logic.recv(callback);
}
exports.activate = activate;
function nullmap(x) {
    return x === null ? undefined : x;
}
function convpos(x) {
    return new vscode.Position(x.line, x.column);
}
function convrange(x) {
    return new vscode.Range(convpos(x.start), convpos(x.end));
}
function deactivate() {
}
exports.deactivate = deactivate;
var channel;
(function (channel) {
    class Channel {
        constructor() {
            this.tx = null;
            this.buffer = [];
        }
        wake() {
            if (this.buffer.length > 0) {
                return Promise.resolve();
            }
            else {
                return this.reset();
            }
        }
        recv() {
            let value = this.buffer.shift();
            if (value === undefined) {
                unreachable('Channel.recv.(value === undefined)');
                throw new Error;
            }
            else {
                return value;
            }
        }
        send(x) {
            this.buffer.push(x);
            if (this.tx !== null) {
                let tx = this.tx;
                this.tx = null;
                tx();
            }
        }
        reset() {
            return new Promise(resolve => {
                this.tx = resolve;
            });
        }
    }
    channel.Channel = Channel;
    function unreachable(site) {
        console.error(`UNREACHABLE ${site}`);
        throw new Error;
    }
})(channel || (channel = {}));
var native;
(function (native) {
    class Logic {
        constructor(extensionPath, workspacePath, crit) {
            this.crit = crit;
            this.killed = false;
            this.parser = new multijson.Parser();
            process.env.RUST_BACKTRACE = '1';
            if (os.platform() === 'linux') {
                this.path = `${extensionPath}/data/bin/linux`;
            }
            else {
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
        send(impulse) {
            // console.log(`  --> ${JSON.stringify(impulse)}`);
            if (this.kid.stdin === null) {
                throw this.crit.error('an unexpected internal error has occured', 'this.kid.stdin === null');
            }
            this.kid.stdin.write(`${JSON.stringify(impulse)}\n`);
        }
        recv(callback) {
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
    native.Logic = Logic;
})(native || (native = {}));
var critical;
(function (critical) {
    class Critical {
        constructor(name, repository, telemetry) {
            this.name = name;
            this.repository = repository;
            this.telemetry = telemetry;
        }
        error(message, extended_log) {
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
        os_support() {
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
    critical.Critical = Critical;
})(critical || (critical = {}));
var multijson;
(function (multijson) {
    class Parser {
        constructor() {
            this.buffer = Buffer.alloc(0);
        }
        write(chunk) {
            this.buffer = Buffer.concat([this.buffer, chunk]);
        }
        read() {
            let objs = [];
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
    multijson.Parser = Parser;
})(multijson || (multijson = {}));
var progress;
(function (progress) {
    class Register {
        constructor(logic) {
            this.logic = logic;
            this.cancel_aids = new Map;
            this.channels = new Map;
        }
        start(hid, options) {
            let comms = new channel.Channel();
            this.channels.set(hid, comms);
            vscode.window.withProgress({
                cancellable: options.cancellable,
                location: this.convert_location(options.location),
                title: options.title !== null ? options.title : undefined,
            }, (progress_handle, cancel_token) => __awaiter(this, void 0, void 0, function* () {
                cancel_token.onCancellationRequested(_ => {
                    comms.send('cancel');
                });
                while (true) {
                    yield comms.wake();
                    let event = comms.recv();
                    if (event === 'cancel') {
                        let aid = this.cancel_aids.get(hid);
                        if (aid === 'premature') {
                            // unreachable
                        }
                        else if (aid !== undefined) {
                            this.logic.send({ tag: 'async', aid, value: null });
                        }
                        else {
                            this.cancel_aids.set(hid, 'premature');
                        }
                        break;
                    }
                    else if (event === 'finish') {
                        break;
                    }
                    else {
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
            }));
        }
        update(hid, update) {
            let comms = this.channels.get(hid);
            if (comms !== undefined) {
                comms.send(update);
            }
        }
        finish(hid) {
            let comms = this.channels.get(hid);
            if (comms !== undefined) {
                comms.send('finish');
            }
        }
        register_cancel(hid, aid) {
            if (this.channels.has(hid)) {
                this.cancel_aids.set(hid, aid);
            }
            else if (this.cancel_aids.get(hid) === 'premature') {
                this.logic.send({ tag: 'async', aid, value: null });
                this.cancel_aids.delete(hid);
            }
        }
        convert_location(location) {
            if (location === 'notification') {
                return vscode.ProgressLocation.Notification;
            }
            else if (location === 'source_control') {
                return vscode.ProgressLocation.SourceControl;
            }
            else if (location === 'window') {
                return vscode.ProgressLocation.Window;
            }
            else {
                throw new Error(`unrecognized progress location ${JSON.stringify(location)}`);
            }
        }
    }
    progress.Register = Register;
})(progress || (progress = {}));
var webview;
(function (webview) {
    class AsyncBuffer {
        constructor(logic) {
            this.logic = logic;
            this.aid = null;
            this.buffer = [];
        }
        send(value) {
            if (this.aid !== null) {
                this.logic.send({ tag: 'async', aid: this.aid, value });
            }
            else {
                this.buffer.push(value);
            }
        }
        register(aid) {
            this.aid = aid;
            for (let value of this.buffer) {
                this.logic.send({ tag: 'async', aid, value });
            }
            this.buffer = [];
        }
    }
    webview.AsyncBuffer = AsyncBuffer;
    class Panel {
        constructor(options, logic) {
            this.listener = new AsyncBuffer(logic);
            this.disposer = new AsyncBuffer(logic);
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
        set_html(html) {
            if (this.panel !== null) {
                this.panel.webview.html = html;
            }
        }
        post_message(message) {
            if (this.panel !== null) {
                this.panel.webview.postMessage(message);
            }
        }
        register_listener(aid) {
            this.listener.register(aid);
        }
        register_disposer(aid) {
            this.disposer.register(aid);
        }
        was_disposed() {
            return this.panel === null;
        }
        is_visible() {
            return this.panel !== null && this.panel.visible;
        }
        reveal(view_column, preserve_focus) {
            if (this.panel !== null) {
                this.panel.reveal(convert_view_column(view_column), preserve_focus);
            }
        }
        dispose() {
            if (this.panel !== null) {
                this.panel.dispose();
            }
        }
        is_active() {
            return this.panel !== null && this.panel.active;
        }
    }
    webview.Panel = Panel;
    function convert_view_column(col) {
        if (col === 'active') {
            return vscode.ViewColumn.Active;
        }
        else if (col === 'beside') {
            return vscode.ViewColumn.Beside;
        }
        else if (col === 'eight') {
            return vscode.ViewColumn.Eight;
        }
        else if (col === 'five') {
            return vscode.ViewColumn.Five;
        }
        else if (col === 'four') {
            return vscode.ViewColumn.Four;
        }
        else if (col === 'nine') {
            return vscode.ViewColumn.Nine;
        }
        else if (col === 'one') {
            return vscode.ViewColumn.One;
        }
        else if (col === 'seven') {
            return vscode.ViewColumn.Seven;
        }
        else if (col === 'six') {
            return vscode.ViewColumn.Six;
        }
        else if (col === 'three') {
            return vscode.ViewColumn.Three;
        }
        else if (col === 'two') {
            return vscode.ViewColumn.Two;
        }
        else {
            throw new Error('unrecognized view column');
        }
    }
    webview.convert_view_column = convert_view_column;
})(webview || (webview = {}));
var register;
(function (register) {
    class Register {
        constructor() {
            this.store = new Map;
        }
        create(hid, obj) {
            this.store.set(hid, obj);
        }
        run(hid, f) {
            let obj = this.store.get(hid);
            if (obj !== undefined) {
                f(obj);
            }
        }
        query(hid, f, def) {
            let obj = this.store.get(hid);
            if (obj !== undefined) {
                return f(obj);
            }
            else {
                return def;
            }
        }
    }
    register.Register = Register;
})(register || (register = {}));
//# sourceMappingURL=extension.js.map