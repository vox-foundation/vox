import * as vscode from 'vscode';
import { VoxMcpClient } from './core/VoxMcpClient';
import { WorkspaceContextEngine } from './context/WorkspaceContextEngine';
import { ChatController } from './chat/ChatController';

export class SidebarProvider implements vscode.WebviewViewProvider {
    private _view?: vscode.WebviewView;
    private _chatController: ChatController;
    private _contextEngine = new WorkspaceContextEngine();

    constructor(
        private readonly _extensionUri: vscode.Uri,
        private readonly _mcp: VoxMcpClient,
    ) {
        this._chatController = new ChatController(
            this._mcp,
            messages => {
                this.postMessage({ type: 'chatHistory', value: messages });
            },
        );
    }

    resolveWebviewView(view: vscode.WebviewView): void {
        this._view = view;
        view.webview.options = { enableScripts: true, localResourceRoots: [this._extensionUri] };
        view.webview.html = this._getHtml(view.webview);

        this._chatController.loadHistory();
        this._startPolling();

        view.webview.onDidReceiveMessage(async (msg: any) => {
            switch (msg.type) {
                case 'getInitialData':
                    this._sendFullState();
                    break;
                case 'submitTask':
                    await this._chatController.submitMessage(msg.value, this._contextEngine.getOpenFilePaths());
                    break;
                case 'applyChanges':
                    if (msg.value) {
                         const { path, content } = msg.value;
                         const fullPath = vscode.Uri.file(vscode.workspace.rootPath + '/' + path);
                         const edit = new vscode.WorkspaceEdit();
                         edit.replace(fullPath, new vscode.Range(0, 0, 10000, 0), content);
                         await vscode.workspace.applyEdit(edit);
                         await vscode.workspace.openTextDocument(fullPath);
                         await vscode.window.showTextDocument(fullPath);
                    }
                    break;
                case 'pickModel':
                    vscode.commands.executeCommand('vox.pickModel');
                    break;
            }
        });

        // Track active editor
        vscode.window.onDidChangeActiveTextEditor(() => {
            this._sendAst();
        });
    }

    private async _startPolling() {
        setInterval(() => {
            this._sendFullState();
        }, 5000); // 5s pulsars
    }

    private async _sendFullState() {
        if (!this._view) return;
        const status = await this._mcp.orchestratorStatus();
        if (status) this.postMessage({ type: 'gamifyUpdate', value: status });

        const providers = await this._mcp.budgetStatus();
        if (providers) this.postMessage({ type: 'voxStatus', value: providers });

        const surface = await this._mcp.languageSurface();
        if (surface) this.postMessage({ type: 'languageSurface', value: surface });

        const pipeline = await this._mcp.pipelineStatus();
        if (pipeline) this.postMessage({ type: 'pipelineStatus', value: pipeline });

        const tasks = await this._mcp.a2aTasks();
        if (tasks) this.postMessage({ type: 'a2aTasks', value: tasks });

        this._sendAst();
    }

    private async _sendAst() {
        const editor = vscode.window.activeTextEditor;
        if (editor && editor.document.languageId === 'vox') {
            const path = editor.document.uri.fsPath;
            const ast = await this._mcp.astInspect(path);
            this.postMessage({ type: 'astResult', value: ast });
            this.postMessage({ type: 'activeEditorChanged', value: path });
        }
    }

    public postMessage(msg: { type: string, value: any }): void {
        this._view?.webview.postMessage(msg);
    }

    private _getHtml(webview: vscode.Webview): string {
        const bundleUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'out', 'webview.js'));
        const styleUri = webview.asWebviewUri(vscode.Uri.joinPath(this._extensionUri, 'out', 'webview.css'));
        const nonce = getNonce();
        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link rel="stylesheet" href="${styleUri}">
    <title>Vox Sidebar</title>
</head>
<body>
    <div id="root"></div>
    <script nonce="${nonce}" src="${bundleUri}"></script>
</body>
</html>`;
    }
}

function getNonce(): string {
    let text = '';
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) text += chars.charAt(Math.floor(Math.random() * chars.length));
    return text;
}
