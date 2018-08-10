import { homedir } from "os";
import * as cp from 'child_process';

export class Ci {

    public async build(source: string): Promise<void> {
        console.log(`Ci.@build`);
        try {
            await exec(this.exepath(), ['build', source], {});
        } catch (err) {
            console.log(`Ci.@build.#err = ${err}`);
        }
        console.log(`Ci.@build finished`);
    }
    public async test(executable: string, testdir: string): Promise<boolean> {
        try {
            await exec(this.exepath(), ['test', executable, testdir], {});
            return true;
        } catch (e) {
            return false;
        }
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

interface ExecOutput {
    stdout: string,
    stderr: string,
}
function exec(file: string, args: string[], options: cp.ExecFileOptions): Promise<ExecOutput> {
    return new Promise((resolve, reject) => cp.execFile(file, args, options, (err, stdout, stderr) => {
        if (err) {
            reject(err);
        } else {
            resolve({ stdout: stdout, stderr: stderr });
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
