import { homedir } from "os";
import * as cp from 'child_process';

export class Ci {

    public async build(source: string): Promise<void> {
        console.log(`Ci.@build`);
        let out = await exec(this.exepath(), ['build', source], {});
        console.log(`Ci.@build ${JSON.stringify(out)}`);
    }
    public async test(executable: string, testdir: string, collect_outs: boolean): Promise<Test[]> {
        let ciout = await exec(this.exepath(), ['--format', 'json', 'test'].concat(collect_outs ? ['--print-output'] : []).concat([executable, testdir]), {});
        let outs = ciout.stdout.split('\n');
        outs.pop();
        return outs.map(line => JSON.parse(line));
    }
    public async init(task_url: string, project_dir: string, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<void> {
        console.log(`Ci.@init`);
        await execInteractive(this.exepath(), ['--format', 'json', 'init', task_url], { cwd: project_dir }, kid => this.handleAuthRequests(kid, auth));
    }
    public async submit(source: string, task_url: string, auth: (authreq: AuthRequest) => Promise<AuthResponse>): Promise<void> {
        console.log(`Ci.@submit`);
        console.log(`Ci.@submit.#source = ${source}`);
        console.log(`Ci.@submit.#task_url = ${task_url}`);
        try {
            await execInteractive(this.exepath(), ['--format', 'json', 'submit', source, task_url], {}, kid => this.handleAuthRequests(kid, auth));
        } catch (err) {
            console.log(`Ci.@submit.#err = ${err}`);
        }
        console.log(`Ci.@submit Finished`);
    }
    public async version(): Promise<string> {
        let ciout = await exec(this.exepath(), ['--version'], {});
        return ciout.stdout.slice(2).trim();
    }
    private handleAuthRequests(kid: cp.ChildProcess, auth: (authreq: AuthRequest) => Promise<AuthResponse>) {
        console.log(`Ci.@handleAuthRequests`);
        kid.stdout.on('data', async chunk => {
            console.log(`Ci.@handleAuthRequests.#chunk = ${chunk}`);
            let line = chunk.toString(); // TODO this is wrong, but node sucks and I can't be bothered
            let req = JSON.parse(line);
            let resp: AuthResponse[] = [];
            try {
                resp.push(await auth(req));
            } catch (err) {
                console.log(`Ci.@handleAuthRequests.#err ${err}`);
            }
            console.log(`Ci.@handleAuthRequests.#resp = ${JSON.stringify(resp)}`);
            kid.stdin.write(JSON.stringify(resp), 'utf8', () => {
                console.log(`Ci.@handleAuthRequests All written`);
                kid.stdin.end(); // TODO this is a horrible way to flush, but node sucks and I can't be botherd
            }); 
        })
    }

    private exepath() {
        return homedir() + '/.cargo/bin/ci';
    }

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
    output: string | undefined,
}

interface ExecOutput {
    err: Error,
    stdout: string,
    stderr: string,
}
function exec(file: string, args: string[], options: cp.ExecFileOptions): Promise<ExecOutput> {
    return new Promise((resolve, reject) => cp.execFile(file, args, options, (err, stdout, stderr) => {
        if (err !== null && err.message.endsWith('ENOENT')) {
            reject(new Error(`ci is not installed in ~/.cargo/bin/. Check the README for ICIE installation instructions.`));
        } else {
            resolve({ err, stdout, stderr })
        }
    }));
}
function execInteractive(file: string, args: string[], options: cp.ExecFileOptions, while_running: (kid: cp.ChildProcess) => void): Promise<void> {
    return new Promise((resolve, reject) => {
        let kid = cp.spawn(file, args, options);
        kid.on('exit', (code, signal) => {
            console.log(`@execInteractive.#code = ${code}`);
            console.log(`@execInteractive.#signal = ${signal}`);
            resolve();
        });
        kid.stderr.pipe(process.stdout);
        while_running(kid);
    });
}
