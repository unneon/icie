import * as vscode from 'vscode';

export interface TestCase {
    input: string;
    desired: string;
}

export class PanelRun {

    private panel: vscode.WebviewPanel | undefined;
    private onSave: (testCase: TestCase) => void;

    public constructor(onSave: (testCase: TestCase) => void) {
        this.onSave = onSave;
    }

    public show() {
        this.get().reveal();
    }

    private get(): vscode.WebviewPanel {
        return this.panel || this.create();
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
        this.panel.webview.html = `
            <html>
                <head>
                    <meta charset="UTF-8">
                    <meta name="viewport" content="width=device-width, initial-scale=1.0">
                    <title>ICIE Run</title>
                    <style>
                        #wrapper-inputs {
                            display: grid;
                            grid-template-columns: 47.5% 47.5%;
                            grid-gap: 5%;
                        }
                        #wrapper-actions {
                            display: grid;
                            grid-template-columns: 100%;
                            grid-gap: 5%;
                        }
                        .large-input {
                            width: 100%;
                        }
                        .large-button {
                            width: 100%;
                        }
                    </style>
                </head>
                <body>
                    <div id="wrapper-inputs">
                        <div>
                            <h2>Input</h2>
                            <textarea id="input" name="input" rows="20" class="large-input"></textarea>
                        </div>
                        <div>
                            <h2>Desired output</h2>
                            <textarea id="desired" name="desired-output" rows="20" class="large-input"></textarea>
                        </div>
                    </div>
                    <br />
                    <div id="wrapper-actions">
                        <div>
                            <button id="runsave" type="button" class="large-button">Save test</button>
                        </div>
                    </div>
                    <script>
                        let elinput = document.getElementById('input');
                        let eldesired = document.getElementById('desired');
                        let elrunsave = document.getElementById('runsave');
                        let vscode = acquireVsCodeApi();

                        elrunsave.addEventListener('click', () => {
                            vscode.postMessage({
                                'input': elinput.value,
                                'desired': eldesired.value
                            });
                        });
                    </script>
                </body>
            </html>
        `;
        this.panel.webview.onDidReceiveMessage(message => this.onSave(message));
        return this.panel;
    }

}