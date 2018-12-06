declare module 'icie-wrap' {

    export interface QuickPickItem {
        label: string,
        description?: string,
        detail?: string,
        id: string,
    }

    export interface ReactionStatus {
        tag: "status",
        message: string | null,
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

    export interface ImpulsePing {
        tag: "ping",
    }
    export interface ImpulseQuickPick {
        tag: "quick_pick",
        response: string | null,
    }

    export type Reaction = ReactionStatus | ReactionInfoMessage | ReactionErrorMessage | ReactionQuickPick;
    export type Impulse = ImpulsePing | ImpulseQuickPick;

    export function message_recv(callback: (error: any, reaction: Reaction) => void): void;
    export function message_send(impulse: Impulse): string;

}