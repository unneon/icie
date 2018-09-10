'use strict';
import * as vscode from 'vscode';
import { homedir } from 'os';
import * as afs from './afs';
import * as ci from './ci';
import * as conf from './conf';
import * as mnfst from './manifest';
import { PanelRun, TestCase } from './panel_run';
import * as os from 'os';

// this method is called when your extension is activated
// your extension is activated the very first time the command is executed
export function activate(context: vscode.ExtensionContext) {
    for (let i=0; i<5; ++i) {
        console.log('');
    }
    console.log('Congratulations, your extension "icie" is now active!');

    let icie = new ICIE();
    register('icie.build', 'Build', context, () => icie.triggerBuild());
    register('icie.test', 'Test', context, () => icie.triggerTest());
    register('icie.init', 'Init', context, () => icie.triggerInit());
    register('icie.submit', 'Submit', context, () => icie.triggerSubmit());
    register('icie.run', 'Run', context, () => icie.triggerRun());
    register('icie.download', 'Download', context, () => icie.triggerDownload());
    icie.launch().catch(reason => vscode.window.showErrorMessage(`ICIE: ${reason}`));
}
function register(command_name: string, human_name: string, context: vscode.ExtensionContext, f: () => Promise<void>) {
    let disposable = vscode.commands.registerCommand(command_name, async () => {
        try {
            await f();
            vscode.window.showInformationMessage(`ICIE ${human_name} succeded`);
        } catch (e) {
            vscode.window.showErrorMessage(`ICIE ${human_name} failed: ${e}`);
        }
    });
    context.subscriptions.push(disposable);
}

// this method is called when your extension is deactivated
export function deactivate() {
}

const requiredCiVersion = "1.2.0";

class ICIE {

    ci: ci.Ci;
    dir: Directory;
    panel_run: PanelRun;
    status: Status;

    public constructor() {
        this.ci = new ci.Ci(""); // TODO handle lack of ci properly
        this.dir = new Directory(vscode.workspace.rootPath || ""); // TODO handle undefined properly
        this.panel_run = new PanelRun(testCase => this.addTest(testCase));
        this.status = new Status;
    }

    @astatus('Lauching')
    public async launch(): Promise<void> {
        let installation = await this.assureInstalled(await ci.Ci.findCiPath());
        console.log(`Ci detected, ${installation.type} install at ${installation.path} with version ${installation.version}`);
        this.ci = new ci.Ci(installation.path);
        let _config: Promise<conf.Config> = conf.load();
        let source = await vscode.workspace.openTextDocument(this.dir.source());
        let editor = await vscode.window.showTextDocument(source);
        let oldPosition = editor.selection.active;
        let config = await _config;
        let newPosition = oldPosition.with(config.template.start.row - 1, config.template.start.column - 1);
        let newSelection = new vscode.Selection(newPosition, newPosition);
        editor.selection = newSelection;
    }
    @astatus('Building')
    public async triggerBuild(): Promise<void> {
        console.log(`ICIE.@triggerBuild`);
        await this.assureAllSaved();
        let source = this.dir.source();
        let library = await afs.exists(this.dir.library()) ? this.dir.library() : undefined;
        await this.ci.build(source, library);
    }
    @astatus('Testing')
    public async triggerTest(): Promise<void> {
        await this.assureCompiled();
        let executable = this.dir.executable();
        let testdir = this.dir.testsDirectory();
        console.log(`[ICIE.triggerTest] Checking ${executable} agains ${testdir}`);
        let tests = await this.ci.test(executable, testdir);
        let all_accepted = tests.every(test => test.outcome == "Accept");
        if (!all_accepted || this.panel_run.isOpen()) {
            let _run_update = this.panel_run.update(tests);
            if (!all_accepted) {
                _run_update.then(() => this.panel_run.show());
                throw 'Some tests failed';
            }
        }
    }
    public async triggerRun(): Promise<void> {
        if (!this.panel_run.isOpen()) {
            this.panel_run.show();
            await this.triggerTest();
        }
    }

    @astatus('Preparing project')
    public async triggerInit(): Promise<void> {
        let task_url = await this.askDescriptionURL();
        let project_name = await this.randomProjectName(5);
        let project_dir = homedir() + '/' + project_name;
        await afs.mkdir(project_dir);
        await this.ci.init(task_url, project_dir, domain => this.respondAuthreq(domain));
        let manifest: mnfst.Manifest = { task_url };
        await mnfst.save(project_dir + '/.icie', manifest);
        let config = await conf.load();
        let dir2 = new Directory(project_dir);
        await afs.copy(config.template.path, dir2.source());
        await vscode.commands.executeCommand('vscode.openFolder', vscode.Uri.file(project_dir), false);
    }
    @astatus('Submitting')
    public async triggerSubmit(): Promise<void> {
        console.log('ICIE Submit triggered');
        await this.assureTested();
        let manifest = await mnfst.load(vscode.workspace.rootPath + '/.icie');
        console.log(`ICIE.@triggerSubmit.#manifest = ${manifest}`);
        let id = await this.ci.submit(this.dir.source(), manifest.task_url, authreq => this.respondAuthreq(authreq));
        this.trackSubmit(manifest.task_url, id); // in background
    }
    @astatus('Downloading')
    public async triggerDownload(): Promise<void> {
        let manifest = await mnfst.load(vscode.workspace.rootPath + '/.icie');
        let resources = await this.ci.listResources(manifest.task_url);
        for (let resource of resources) {
            console.log(`ICIE.@triggerDownload ${JSON.stringify(resource)}`);
        }
        let choice = await vscode.window.showQuickPick(resources.map(resource => { return {
            label: resource.name,
            detail: resource.description,
            resource: resource
        }; }), {
            matchOnDescription: true,
            matchOnDetail: true
        }, undefined);
        if (choice === undefined) {
            throw new Error('Did not choose file to download');
        }
        let resource = choice.resource;
        console.log(`ICIE.@triggerDownload Chosen ${JSON.stringify(choice)}`);
        await this.ci.downloadToFile(manifest.task_url, resource.id, `${this.dir.base}/${resource.filename}`);
        // TODO check filename for malicious stuff
    }
    private async respondAuthreq(authreq: ci.AuthRequest): Promise<ci.AuthResponse> {
        let username = await inputbox({ prompt: `Username at ${authreq.domain}` });
        let password = await inputbox({ prompt: `Password for ${username} at ${authreq.domain}`});
        return { username, password };
    }
    private async assureInstalled(installation: ci.Installation | undefined): Promise<ci.Installation> {
        console.log(`ICIE.@assureInstalled.#installation ${JSON.stringify(installation)}`);
        if (installation === undefined) {
            return await this.askInstall('Ci is not installed', 'Install');
        } else if (installation.version !== requiredCiVersion) {
            if (installation.type === "Managed") {
                return await this.askInstall('Ci installation is outdated', 'Update');
            } else {
                return await this.askInstall(`Ci version ${installation.version} was found, but version ${requiredCiVersion} is required`, 'Install ICIE-managed Ci');
            }
        } else {
            return installation;
        }
    }
    private async askInstall(message: string, action_name: string): Promise<ci.Installation> {
        let action = await vscode.window.showErrorMessage(message, action_name);
        if (action === undefined) {
            throw new Error('User decided not to install/update Ci');
        }
        return await this.install();
    }
    private async install(): Promise<ci.Installation> {
        let version = requiredCiVersion;
        let platform = recognizePlatform();
        let url = `https://github.com/matcegla/ci/releases/download/v${version}/ci-${version}-${platform}`;
        console.log(`ICIE.@install.#url ${url}`);
        await afs.assureDir(`${homedir()}/.local`);
        await afs.assureDir(`${homedir()}/.local/share`);
        await afs.assureDir(`${homedir()}/.local/share/icie`);
        await ci.exec('/usr/bin/wget', ['-O', `${homedir()}/.local/share/icie/ci`, url], {}); // TODO: I looooooove javascript
        await afs.chmod(`${homedir()}/.local/share/icie/ci`, 0o700);
        console.log(`ICIE.@install after request`);
        vscode.window.showInformationMessage(`ICIE has been installed`);
        return {
            path: `${homedir()}/.local/share/icie/ci`,
            type: "Managed",
            version: await new ci.Ci(`${homedir()}/.local/share/icie/ci`).version(),
        };
    }

    public dispose() {

    }

    private async requiresCompilation(): Promise<boolean> {
        console.log(`ICIE.@assureCompiled`);
        let src = this.dir.source();
        console.log(`[ICIE.assureCompiled] Checking whether ${src} is compiled`);
        let exe = this.dir.executable();
        await this.assureAllSaved();
        console.log(`ICIE.@assureCompiled All files have been saved`);
        let statsrc = await afs.stat(src);
        try {
            var statexe = await afs.stat(exe);
        } catch (err) {
            console.log(`ICIE.@assureCompiled ${src} needs compiling`);
            return true;
        }
        if (statsrc.mtime <= statexe.mtime) {
            console.log(`[ICIE.assureCompiled] ${src} was compiled already`);
            return false;
        } else {
            console.log(`[ICIE.assureCompiled] ${src} needs compiling`);
            return true;
        }
    }
    private async addTest(testCase: TestCase): Promise<void> {
        await this.saveTest(testCase);
        await this.triggerTest();
    }
    private async saveTest(testCase: TestCase): Promise<void> {
        let i = 1;
        while (await afs.exists(`${this.dir.customTests()}/${i}.in`))
            ++i;
        await afs.assureDir(this.dir.customTests());
        await afs.write(`${this.dir.customTests()}/${i}.in`, testCase.input);
        await afs.write(`${this.dir.customTests()}/${i}.out`, testCase.desired);
    }
    private async trackSubmit(task_url: string, id: string): Promise<void> {
        let last_message: string | undefined = undefined;
        await vscode.window.withProgress({
            cancellable: false,
            location: vscode.ProgressLocation.Notification,
            title: `Tracking submit ${id}...`
        }, async (progress, token) => {
            await this.ci.trackSubmit(task_url, id, async track => {
                console.log(`ICIE.@trackSubmit ${JSON.stringify(track)}`);
                let message = this.formatTrack(track);
                progress.report({ message });
                last_message = message;
            });
        });
        if (last_message !== undefined) {
            vscode.window.showInformationMessage(last_message);
        } else {
            console.log('Tracking submit FAILED');
        }
    }
    private formatTrack(track: ci.Track): string {
        if (track.compilation === "Pending") {
            return `Compilation pending...`;
        } else if (track.compilation === "Failure") {
            return `Compilation error`;
        } else if (track.initial === "Pending") {
            return `Initial tests pending...`;
        } else if (track.full === "Pending") {
            return `${this.formatInitial(track.initial)}Score pending...`;
        } else {
            return this.formatFull(track.full);
        }
    }
    private formatInitial(outcome: ci.TrackOutcome): string {
        if ((<ci.TrackScore>outcome).Score !== undefined) {
            return `Initial tests scored ${(<ci.TrackScore>outcome).Score}. `;
        } else if (outcome === "Failure") {
            return `Initial tests failed. `;
        } else if (outcome === "Success") {
            return `Initial tests passed. `;
        } else if (outcome === "Unsupported") {
            return ``;
        } else if (outcome === "Skipped") {
            return ``;
        } else {
            throw new Error(`Invalid initial tests outcome ${JSON.stringify(outcome)}`);
        }
    }
    private formatFull(outcome: ci.TrackOutcome): string {
        if ((<ci.TrackScore>outcome).Score !== undefined) {
            return `Scored ${(<ci.TrackScore>outcome).Score}!`;
        } else if (outcome === "Failure") {
            return `Rejected`;
        } else if (outcome === "Success") {
            return `Accepted`;
        } else {
            console.log(`ICIE.@formatFull ${JSON.stringify(outcome)}`);
            throw new Error(`Invalid tests outcome ${JSON.stringify(outcome)}`);
        }
    }

    private async assureTested(): Promise<void> {
        await this.triggerTest();
    }
    private async assureCompiled(): Promise<void> {
        console.log(`ICIE.@assureCompiled`);
        if (await this.requiresCompilation()) {
            await this.triggerBuild();
        }
    }
    private async assureAllSaved(): Promise<void> {
        return vscode.workspace.saveAll(false).then(something => {});
    }

    private askDescriptionURL(): Promise<string> {
        return inputbox({ prompt: 'Task description URL: ' });
    }

    private async randomProjectName(tries: number): Promise<string> {
        for (; tries>0; --tries) {
            let name = this.randomName();
            if (!await afs.exists(homedir() + '/' + name)) {
                return name;
            }
        }
        return Promise.reject('failed to find free project name');
    }
    private randomName(): string {
        let adjectives = [
            "playful",
            "shining",
            "sparkling",
            "rainbow",
            "kawaii",
            "superb",
            "amazing",
            "glowing",
            "blessed",
            "smiling",
            "exquisite", // suggested by Xeoeen
            "cuddly",
            "caramel",
            "serene",
            "sublime",
            "beaming",
        ];
        let animals = [
            "capybara",
            "squirrel",
            "spider",
            "anteater",
            "hamster",
            "whale",
            "eagle",
            "zebra",
            "dolphin",
            "hedgehog",
            "penguin", // suggested by Xeoeen
            "wombat", // suggested by Xeoeen
            "ladybug",
            "platypus", // suggested by Xeoeen
            "squid", // suggested by Xeoeen
        ];
        return choice(adjectives) + '-' + choice(animals);
    }

}

class Directory {

    base: string;

    public constructor(base: string) {
        this.base = base;
    }

    public source(): string {
        return this.base + '/main.cpp';
    }
    public executable(): string {
        let source = this.source();
        return source.substr(0, source.length - 4) + '.e';
    }
    public testsDirectory(): string {
        return this.base + '/tests';
    }
    public customTests(): string {
        return this.testsDirectory() + '/' + os.userInfo().username;
    }
    public library(): string {
        return this.base + '/lib.cpp';
    }

}

class Status {

    stack: string[];
    item: vscode.StatusBarItem;

    constructor() {
        this.stack = [];
        this.item = vscode.window.createStatusBarItem();
    }

    public push(name: string) {
        this.stack.push(name);
        this.update();
    }
    public pop() {
        this.stack.pop();
        this.update();
    }

    private update() {
        if (this.stack.length > 0) {
            let message = this.stack[this.stack.length-1];
            this.item.text = `ICIE ${message}`;
            this.item.show();
        } else {
            this.item.hide();
        }
    }

}

function astatus(message: string) {
    return (target: any, propertyKey: string, descriptor: PropertyDescriptor) => {
        let oldf = descriptor.value;
        descriptor.value = async function (...args) {
            let this2: ICIE = this as any;
            this2.status.push(message);
            try {
                return await oldf.apply(this, args);
            } finally {
                this2.status.pop();
            }
        };
    };
}

type Platform = "linux-amd64";
function recognizePlatform(): Platform {
    let plat = os.platform();
    let arch = os.arch();
    if (plat === 'linux' && arch === 'x64') {
        return 'linux-amd64';
    } else {
        throw new Error(`Unrecognized platform (${plat}, ${arch})`);
    }
}

function choice<T>(xs: T[]): T {
    return xs[Math.floor(Math.random() * xs.length)];
}
async function inputbox(options: vscode.InputBoxOptions): Promise<string> {
    let maybe = await vscode.window.showInputBox(options);
    if (maybe !== undefined) {
        return maybe;
    } else {
        throw new Error("did not get input on input box");
    }
}
