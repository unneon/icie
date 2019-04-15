import { spawn, ChildProcess } from "child_process";
import * as multijson from './multijson';

export interface QuickPickItem {
    label: string;
    description: string | null;
    detail: string | null;
    id: string;
}
export interface TestviewTreeTest {
    name: string;
    input: string;
    output: string;
    desired: string | null;
    timing: number | null; // milliseconds
    in_path: string;
    outcome: Outcome;
}
export type TestviewTree = TestviewTreeTest | TestviewTreeTest[];
export function isTest(tree: TestviewTree): tree is TestviewTreeTest {
    return (<TestviewTreeTest>tree).input !== undefined;
}
export type Outcome = 'accept' | 'wrong_answer' | 'runtime_error' | 'time_limit_exceeded' | 'ignored_no_out';
export type MessageKind = 'info' | 'warning' | 'error';
export interface MessageItem {
    title: string;
    is_close_affordance: boolean | null;
    id: string;
}
export interface MessageItems {
    id: string;
    list: MessageItem[];
}
export interface Position {
    line: number;
    character: number;
}

export interface ReactionStatus {
    tag: "status";
    message: string | null;
}
export interface ReactionMessage {
    tag: "message";
    message: string;
    kind: MessageKind;
    items: MessageItems | null;
    modal: boolean | null;
}
export interface ReactionQuickPick {
    tag: "quick_pick";
    items: QuickPickItem[];
    matchOnDescription: boolean;
    matchOnDetail: boolean;
}
export interface ReactionInputBox {
    tag: "input_box";
    prompt: string | null;
    placeholder: string | null;
    password: boolean;
    ignoreFocusOut: boolean;
    value: string | null;
    valueSelection: [number, number] | null;
}
export interface ReactionConsoleLog {
    tag: "console_log";
    message: string;
}
export interface ReactionSaveAll {
    tag: "save_all";
}
export interface ReactionOpenFolder {
    tag: "open_folder";
    path: string;
    in_new_window: boolean;
}
export interface ReactionConsoleError {
    tag: "console_error";
    message: string;
}
export interface ReactionOpenEditor {
    tag: "open_editor";
    path: string;
    row: number;
    column: number;
}
export interface ReactionProgressStart {
    tag: "progress_start";
    id: string;
    title: string | null;
}
export interface ReactionProgressUpdate {
    tag: "progress_update";
    id: string;
    increment: number | null;
    message: string | null;
}
export interface ReactionProgressEnd {
    tag: "progress_end";
    id: string;
}
export interface ReactionTestviewFocus {
    tag: "testview_focus";
}
export interface ReactionTestviewUpdate {
    tag: "testview_update";
    tree: TestviewTree;
}
export interface ReactionMultitestViewFocus {
    tag: "multitest_view_focus";
}
export interface ReactionDiscoveryRow {
	tag: 'discovery_row';
	number: number;
	outcome: Outcome;
	fitness: number;
	input: string | null;
}
export interface ReactionDiscoveryState {
	tag: 'discovery_state';
	running: boolean;
	reset: boolean;
}
export interface ReactionQueryDocumentText {
    tag: 'query_document_text';
    path: string;
}
export interface ReactionPasteEdit {
    tag: 'edit_paste';
    position: Position;
    text: string;
    path: string;
}

export interface ImpulseQuickPick {
    tag: "quick_pick";
    response: string | null;
}
export interface ImpulseInputBox {
    tag: "input_box";
    response: string | null;
}
export interface ImpulseTriggerBuild {
    tag: "trigger_build";
}
export interface ImpulseWorkspaceInfo {
    tag: "workspace_info";
    root_path: string | null;
}
export interface ImpulseTriggerTest {
    tag: "trigger_test";
}
export interface ImpulseSavedAll {
    tag: "saved_all";
}
export interface ImpulseTriggerInit {
    tag: "trigger_init";
}
export interface ImpulseTriggerSubmit {
    tag: "trigger_submit";
}
export interface ImpulseTriggerManualSubmit {
    tag: "trigger_manual_submit";
}
export interface ImpulseTriggerTemplateInstantiate {
    tag: "trigger_template_instantiate";
}
export interface ImpulseTriggerTestview {
    tag: "trigger_testview";
}
export interface ImpulseTriggerRR {
    tag: "trigger_rr";
    in_path: string;
}
export interface ImpulseNewTest {
    tag: "new_test";
    input: string;
    desired: string;
}
export interface ImpulseMessageResponse {
    tag: "message_response";
    id: string;
    response: string | null;
}
export interface ImpulseTriggerMultitestView {
    tag: "trigger_multitest_view";
}
export interface ImpulseDiscoveryStart {
	tag: 'discovery_start';
}
export interface ImpulseDiscoveryPause {
	tag: 'discovery_pause';
}
export interface ImpulseDiscoveryReset {
	tag: 'discovery_reset';
}
export interface ImpulseDiscoverySave {
	tag: 'discovery_save';
	input: string;
}
export interface ImpulseTriggerPastePick {
    tag: 'trigger_paste_pick';
}
export interface ImpulseDocumentText {
    tag: 'document_text';
    contents: string;
}
export interface ImpulseAcknowledgeEdit {
    tag: 'acknowledge_edit';
}
export interface ImpulseTriggerTerminal {
    tag: 'trigger_terminal';
}
export interface ImpulseTriggerInitExisting {
    tag: 'trigger_init_existing';
}
export interface ImpulseTriggerQistruct {
    tag: 'trigger_qistruct';
}

export type Reaction = ReactionStatus | ReactionMessage | ReactionQuickPick | ReactionInputBox | ReactionConsoleLog | ReactionSaveAll | ReactionOpenFolder | ReactionConsoleError | ReactionOpenEditor | ReactionProgressStart | ReactionProgressUpdate | ReactionProgressEnd | ReactionTestviewFocus | ReactionTestviewUpdate | ReactionMultitestViewFocus | ReactionDiscoveryRow | ReactionDiscoveryState | ReactionQueryDocumentText | ReactionPasteEdit;
export type Impulse = ImpulseQuickPick | ImpulseInputBox | ImpulseTriggerBuild | ImpulseWorkspaceInfo | ImpulseSavedAll | ImpulseTriggerTest | ImpulseTriggerInit | ImpulseTriggerSubmit | ImpulseTriggerManualSubmit | ImpulseTriggerTemplateInstantiate | ImpulseTriggerTestview | ImpulseTriggerRR | ImpulseNewTest | ImpulseMessageResponse | ImpulseTriggerMultitestView | ImpulseDiscoveryPause | ImpulseDiscoveryReset | ImpulseDiscoverySave | ImpulseDiscoveryStart | ImpulseTriggerPastePick | ImpulseDocumentText | ImpulseAcknowledgeEdit | ImpulseTriggerTerminal | ImpulseTriggerInitExisting | ImpulseTriggerQistruct;

export class Logic {
    path: string;
    kid: ChildProcess;
    parser: multijson.Parser<Reaction>;
    constructor(extensionPath: string) {
        process.env.RUST_BACKTRACE = '1';
        this.path = `${extensionPath}/assets/bin-linux`;
        this.kid = spawn(this.path, [], {});
        this.parser = new multijson.Parser<Reaction>();
    }
    send(impulse: Impulse) {
        console.log(`   ~> ${JSON.stringify(impulse)}`);
        this.kid.stdin.write(`${JSON.stringify(impulse)}\n`);
    }
    recv(callback: (reaction: Reaction) => void) {
        this.kid.stdout.on('data', chunk => {
            if (typeof chunk === 'string' || chunk instanceof String) {
                throw new Error('icie_stdio.stdout.on [data] returned string instead of Buffer');
            }
            this.parser.write(chunk);
            for (let reaction of this.parser.read()) {
                console.log(`<~    ${JSON.stringify(reaction)}`);
                callback(reaction);
            }
        });
    }
    kill() {
        this.kid.kill('SIGKILL');
    }
}
