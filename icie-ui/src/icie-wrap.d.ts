declare module 'icie-wrap' {

    export interface QuickPickItem {
        label: string,
        description?: string,
        detail?: string,
        id: string,
    }

    export interface ReactionStatus {
        tag: "status",
        message?: string,
    }
    export interface ReactionInfoMessage {
        tag: "info_message",
        message: string,
    }
    export interface ReactionErrorMessage {
        tag: "error_message",
        message: string,
    }
    export interface ReactionQuickPick {
        tag: "quick_pick",
        items: QuickPickItem[],
    }
    export interface ReactionInputBox {
        tag: "input_box",
        prompt?: string,
        placeholder?: string,
        password: boolean,
        ignoreFocusOut: boolean,
    }
    export interface ReactionConsoleLog {
        tag: "console_log",
        message: string,
    }
    export interface ReactionSaveAll {
        tag: "save_all",
    }
    export interface ReactionOpenFolder {
        tag: "open_folder",
        path: string,
        in_new_window: boolean,
    }
    export interface ReactionConsoleError {
        tag: "console_error",
        message: string,
    }

    export interface ImpulseQuickPick {
        tag: "quick_pick",
        response: string | null,
    }
    export interface ImpulseInputBox {
        tag: "input_box",
        response: string | null,
    }
    export interface ImpulseTriggerBuild {
        tag: "trigger_build",
    }
    export interface ImpulseWorkspaceInfo {
        tag: "workspace_info",
        root_path: string | null,
    }
    export interface ImpulseTriggerTest {
        tag: "trigger_test",
    }
    export interface ImpulseSavedAll {
        tag: "saved_all",
    }
    export interface ImpulseTriggerInit {
        tag: "trigger_init",
    }
    export interface ImpulseTriggerSubmit {
        tag: "trigger_submit",
    }
    export interface ImpulseTriggerManualSubmit {
        tag: "trigger_manual_submit",
    }

    export type Reaction = ReactionStatus | ReactionInfoMessage | ReactionErrorMessage | ReactionQuickPick | ReactionInputBox | ReactionConsoleLog | ReactionSaveAll | ReactionOpenFolder | ReactionConsoleError;
    export type Impulse = ImpulseQuickPick | ImpulseInputBox | ImpulseTriggerBuild | ImpulseWorkspaceInfo | ImpulseSavedAll | ImpulseTriggerTest | ImpulseTriggerInit | ImpulseTriggerSubmit | ImpulseTriggerManualSubmit;

    export function message_recv(callback: (error: any, reaction: Reaction) => void): void;
    export function message_send(impulse: Impulse): string;

}