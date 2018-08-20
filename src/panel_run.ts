import * as vscode from 'vscode';
import { Test } from './ci';
import * as afs from './afs';

export interface TestCase {
    input: string;
    desired: string;
}

export class PanelRun {

    private panel: vscode.WebviewPanel | undefined;

    public async show(tests: Test[]): Promise<void> {
        await this.update(tests);
        this.get().reveal();
    }

    private get(): vscode.WebviewPanel {
        return this.panel || this.create();
    }
    private async update(tests: Test[]): Promise<void> {
        this.get().webview.html = await this.view(tests);
    }
    private create() {
        this.panel = vscode.window.createWebviewPanel(
            'icieRun',
            'ICIE Run',
            vscode.ViewColumn.One,
            {
                enableScripts: true,
                localResourceRoots: []
            }
        );
        return this.panel;
    }

    private async view(tests: Test[]): Promise<string> {
        return `
        <html>
            <head>
                <meta charset="UTF-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <title>ICIE Run</title>
                <style>
                    .wrapper {
                        display: grid;
                        grid-template-columns: 32% 32% 32%;
                        grid-gap: 2%;
                    }
                    .stringarea {
                        font-size: 24px;
                        font-family: Ubuntu Mono;
                        width: 100%;
                    }
                    .output-good {
                        background-color: #C0FFC0;
                    }
                    .output-bad {
                        background-color: #FFC0C0;
                    }
                    .large-button {
                        width: 100%;
                    }
                </style>
            </head>
            <body>
                <div class="wrapper">
                    <h2>Input</h2>
                    <h2>Output</h2>
                    <h2>Desired output</h2>
                </div>
                ${(await this.viewTests(tests)).join('')}
                <br />
            </body>
        </html>
        `;
    }

    private async viewTests(tests: Test[]): Promise<string[]> {
        return await Promise.all(tests.map(async test => await this.viewTest(test)));
    }
    private async viewTest(test: Test): Promise<string> {
        let output_class = test.outcome == "Accept" ? 'output-good' : 'output-bad';
        let text_input = await afs.read(test.in_path, 'utf8');
        let text_output = test.output;
        let desired_path = `${test.in_path.slice(undefined, test.in_path.length-3)}.out`;
        let text_desired_output = await afs.read(desired_path, 'utf8');
        let rows = Math.max(...[text_input, text_output, text_desired_output].map(docrows));
        return `
            <h3>Test ${test.in_path}</h3>
            <div class="wrapper">
                <textarea rows="${rows}" class="stringarea">${trailendl(text_input)}</textarea>
                <textarea rows="${rows}" class="stringarea ${output_class}">${trailendl(text_output)}</textarea>
                <textarea rows="${rows}" class="stringarea">${trailendl(text_desired_output)}</textarea>
            </div>
        `;
    }

}

function trailendl(s: string): string {
    return s.slice(undefined, s.length > 0 && s[s.length-1] == '\n' ? s.length-1 : s.length);
}
function docrows(s: string): number {
    return Array.from(trailendl(s)).filter(c => c === '\n').length + 1;
}
