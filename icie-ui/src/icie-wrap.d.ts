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

    export interface ImpulsePing {
        tag: "ping",
    }
    export interface ImpulseQuickPick {
        tag: "quick_pick",
        response: string | null,
    }
    export interface ImpulseInputBox {
        tag: "input_box",
        response: string | null,
    }

    export type Reaction = ReactionStatus | ReactionInfoMessage | ReactionErrorMessage | ReactionQuickPick | ReactionInputBox;
    export type Impulse = ImpulsePing | ImpulseQuickPick | ImpulseInputBox;

    export function message_recv(callback: (error: any, reaction: Reaction) => void): void;
    export function message_send(impulse: Impulse): string;

}