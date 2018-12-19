import * as icie_wrap from 'icie-wrap';

export import Reaction = icie_wrap.Reaction;
export import Impulse = icie_wrap.Impulse;

export function send(impulse: icie_wrap.Impulse) {
    console.log(`   ~> ${JSON.stringify(impulse)}`);
    let ermsg = icie_wrap.message_send(impulse);
    if (!ermsg.startsWith("Message sent successfully")) {
        console.error(`   #> ${ermsg}`);
    }
}
export function recv(callback: (err: any, reaction: icie_wrap.Reaction) => void) {
    icie_wrap.message_recv((err, reaction) => {
        console.log(`<~    ${JSON.stringify(reaction)}`);
        callback(err, reaction);
    });
}