import { homedir } from "os";
import * as cp from 'child_process';
import { Writable, Readable } from "stream";
import * as afs from './afs';

export class Ci {

    private exepath: string;

    public constructor(exepath: string) {
        this.exepath = exepath;
    }

    public async build(source: string, library: string | undefined): Promise<void> {
        console.log(`Ci.@build`);
        let libopts = library !== undefined ? ['--lib', library] : [];
        let out = await exec(this.exepath, ['build', source].concat(libopts), {});
        if (out.err !== null && out.err.message.startsWith('Command failed:')) {
            throw new Error('Compiler error');
        }
    }
    public async test(executable: string, testdir: string): Promise<Test[]> {
        let ciout = await exec(this.exepath, ['--format', 'json', 'test', '--print-output', executable, testdir], {});
        let outs = ciout.stdout.split('\n');
        outs.pop();
        return outs.map(line => JSON.parse(line));
    }
    public async init(task_url: string, project_dir: string, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<void> {
        console.log(`Ci.@init`);
        await execInteractive(this.exepath, ['--format', 'json', 'init', task_url], { cwd: project_dir }, kid => this.handleAuthRequests(kid, auth));
    }
    public async submit(source: string, task_url: string, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<string> {
        console.log(`Ci.@submit`);
        console.log(`Ci.@submit.#source = ${source}`);
        console.log(`Ci.@submit.#task_url = ${task_url}`);
        let id: string|undefined = undefined;
        let out = await execInteractive(this.exepath, ['--format', 'json', 'submit', source, task_url], {}, async kid => {
            handleStdin(kid, async msg => {
                if (msg.id === undefined) {
                    this.handleAuthRequest(kid, msg, auth);
                } else {
                    id = msg.id;
                }
            });
        });
        if (out.code !== 0) {
            throw new Error(`Submit failed (code ${out.code})`);
        }
        if (id === undefined) {
            throw new Error(`Submit did not return submission id`);
        }
        return id;
    }
    public async trackSubmit(task_url: string, id: string, callback: (msg: Track) => Promise<void>): Promise<void> {
        await execInteractive(this.exepath, ['--format', 'json', 'track-submit', task_url, id, '3s'], {}, async kid => {
            handleStdin(kid, async msg => {
                callback(msg);
            });
        });
    }
    public async version(): Promise<string> {
        let ciout = await exec(this.exepath, ['--version'], {});
        return ciout.stdout.slice(2).trim();
    }
    public static async findCiPath(): Promise<Installation | undefined> {
        let candidates: { path: string, type: InstallationType }[] = [
            { path: `${homedir()}/.local/share/icie/ci`, type: "Managed" },
            { path: `${homedir()}/.cargo/bin/ci`, type: "System" }
        ];
        for (let candidate of candidates)
            if (await afs.exists(candidate.path))
                return Object.assign(candidate, { version: await new Ci(candidate.path).version() });
        return undefined;
    }
    private async handleAuthRequests(kid: cp.ChildProcess, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<void> {
        await handleStdin(kid, async msg => {
            await this.handleAuthRequest(kid, msg, auth);
        });
    }
    private async handleAuthRequest(kid: cp.ChildProcess, msg: AuthRequest, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<void> {
        let resp = await auth(msg);
        console.log(`Ci.@handleAuthRequests.#resp = ${JSON.stringify(resp)}`);
        await write(kid.stdin, JSON.stringify(resp), "utf8");
        console.log(`Ci.@handleAuthRequests All written`);
        kid.stdin.end(); // TODO this is a horrible way to flush, but node sucks and I can't be bothered
    }

}

async function handleStdin(kid: cp.ChildProcess, callback: (msg: any) => Promise<void>): Promise<void> {
    console.log(`Ci.@handleAuthRequests`);
    await ondata(kid.stdout, async chunk => {
        console.log(`Ci.@handleAuthRequests.#chunk = ${chunk}`);
        let req = JSON.parse(chunk);
        await callback(req);
    });
}

export type InstallationType = "System" | "Managed" | "None";
export interface Installation {
    path: string,
    type: InstallationType,
    version: string,
}
export type TrackScore = { Score: number; };
export type TrackCompilation = "Pending" | "Success" | "Failure";
export type TrackOutcome = "Unsupported" | "Skipped" | "Waiting" | "Pending" | "Success" | "Failure" | TrackScore;
export interface Track {
    compilation: TrackCompilation,
    initial: TrackOutcome,
    full: TrackOutcome,
}

export interface AuthRequest {
    domain: string;
}
export interface AuthResponse {
    username: string;
    password: string;
}
export type TestOutcome = "Accept" | "WrongAnswer" | "RuntimeError" | "IgnoredNoOut";
export interface Test {
    outcome: TestOutcome,
    in_path: string,
    output: string,
}

interface ExecOutput {
    err: Error | null,
    stdout: string,
    stderr: string,
}
interface ExecInteractiveOutput {
    kid: cp.ChildProcess,
    code: number,
    signal: string,
}
export function exec(file: string, args: string[], options: cp.ExecFileOptions): Promise<ExecOutput> {
    return new Promise((resolve, reject) => cp.execFile(file, args, options, (err, stdout, stderr) => {
        if (err !== null && err.message.endsWith('ENOENT')) {
            reject(new Error(`ci is not installed in ~/.cargo/bin/. Check the README for ICIE installation instructions.`));
        } else {
            resolve({ err, stdout, stderr })
        }
    }));
}
function execInteractive(file: string, args: string[], options: cp.ExecFileOptions, while_running: (kid: cp.ChildProcess) => Promise<void>): Promise<ExecInteractiveOutput> {
    return new Promise((resolve, reject) => {
        let kid = cp.spawn(file, args, options);
        kid.on('exit', (code, signal) => {
            console.log(`@execInteractive.#code = ${code}`);
            console.log(`@execInteractive.#signal = ${signal}`);
            resolve({ kid, code, signal });
        });
        while_running(kid).catch(reason => reject(reason));
    });
}
type WriteEncoding = "utf8";
function write(file: Writable, chunk: any, encoding: WriteEncoding | undefined): Promise<void> {
    return new Promise(resolve => file.write(chunk, encoding, () => resolve()));
}
function ondata(file: Readable, callback: (chunk: string) => Promise<void>): Promise<void> {
    return new Promise((resolve, reject) => file.on('data', chunk => callback(chunk.toString()).catch(reason => reject(reason))));
}
