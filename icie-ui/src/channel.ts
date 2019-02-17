export interface Channel<T> {
    recv: Promise<T>;
    send: (x: T) => void;
}
export function channel<T>(): Channel<T> {
    let send: ((x: T) => void) | null = null;
    let recv: Promise<T> = new Promise(resolve => {
        send = resolve;
    });
    if (send === null) {
        throw new Error("@channel.#send === null");
    }
    return { recv, send };
}
