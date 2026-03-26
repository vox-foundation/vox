import * as vscode from 'vscode';
import * as path from 'path';
import { VoxMcpClient } from './core/VoxMcpClient';
import { WorkspaceContextEngine } from './context/WorkspaceContextEngine';
import { ChatController } from './chat/ChatController';
import { VoxPreferenceKey, byokPreferenceKey } from './core/preferenceKeys';
import { ConfigManager } from './core/ConfigManager';
import { parseWebviewMessage } from './protocol/webviewMessages';
import { parseHostToWebviewMessage } from './protocol/hostToWebviewMessages';

const POLL_MS = 5000;

export class SidebarProvider implements vscode.WebviewViewProvider {
    private _view?: vscode.WebviewView;
    private _chatController: ChatController;
    private _contextEngine = new WorkspaceContextEngine();
    private _pollHandle?: NodeJS.Timeout;

    constructor(
        private readonly _extensionUri: vscode.Uri,
        private readonly _mcp: VoxMcpClient,
    ) {
        this._chatController = new ChatController(
            this._mcp,
            (messages) => {
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

        view.webview.onDidReceiveMessage(async (msg: unknown) => {
            const parsed = parseWebviewMessage(msg);
            if (!parsed) return;
            switch (parsed.type) {
                case 'getInitialData':
                    this._sendFullState();
                    break;
                case 'submitTask':
                    await this._chatController.submitMessage(
                        parsed.value,
                        this._contextEngine.getOpenFilePaths(),
                    );
                    break;
                case 'applyChanges':
                    await this._applyChanges(parsed.value);
                    break;
                case 'pickModel':
                    vscode.commands.executeCommand('vox.pickModel');
                    break;
                case 'updateBudgetCap':
                    await this._mcp.preferenceSet(VoxPreferenceKey.budgetCapUsd, parsed.value);
                    break;
                case 'updateApiKey':
                    await this._mcp.preferenceSet(byokPreferenceKey(parsed.provider), parsed.value);
                    break;
                case 'setModel':
                    await this._mcp.preferenceSet(VoxPreferenceKey.activeModel, parsed.value);
                    break;
                case 'resumeWorkflow':
                    if (this._mcp.isToolAvailable('vox_replan')) {
                        await this._mcp.replanSession({
                            session_id: 'vscode-sidebar',
                            delta_hint:
                                parsed.step !== undefined && String(parsed.step).trim().length > 0
                                    ? String(parsed.step)
                                    : 'resume',
                            write_to_disk: false,
                        });
                    } else {
                        void vscode.window.showInformationMessage(
                            'Workflow resume requires `vox_replan` on the connected MCP server.',
                        );
                    }
                    break;
                case 'setSocratesGate':
                    await this._mcp.preferenceSet(
                        VoxPreferenceKey.socratesGateEnforced,
                        parsed.enforce,
                    );
                    break;
                case 'rejectExecution':
                    void vscode.window.showInformationMessage(
                        'Reject execution is not yet exposed as an MCP tool; skipped.',
                    );
                    break;
                case 'rebalance':
                    if (this._mcp.isToolAvailable('vox_rebalance')) {
                        await this._mcp.rebalance();
                    } else {
                        void vscode.window.showInformationMessage(
                            'Rebalance requires `vox_rebalance` on the connected MCP server.',
                        );
                    }
                    break;
                case 'runCommand':
                    if (parsed.value) await vscode.commands.executeCommand(parsed.value);
                    break;
                case 'ludusAckNotification':
                    if (this._mcp.isToolAvailable('vox_ludus_notification_ack')) {
                        await this._mcp.ludusNotificationAck(parsed.notificationId);
                    }
                    await this.refreshLudusSnapshot(true);
                    break;
                case 'ludusAckAllNotifications':
                    if (this._mcp.isToolAvailable('vox_ludus_notifications_ack_all')) {
                        await this._mcp.ludusNotificationsAckAll();
                    }
                    await this.refreshLudusSnapshot(true);
                    break;
                case 'ludusRefreshSnapshot':
                    await this.refreshLudusSnapshot(true);
                    break;
            }
        });

        vscode.window.onDidChangeActiveTextEditor(() => {
            this._sendAst();
        });
    }

    private _startPolling(): void {
        if (this._pollHandle) clearInterval(this._pollHandle);
        this._pollHandle = setInterval(() => {
            void this._sendFullState();
        }, POLL_MS);
    }

    private async _applyChanges(
        value: { path: string; content: string } | null | undefined,
    ): Promise<void> {
        if (!value?.path) return;
        const rel = value.path.replace(/\\/g, '/');
        const uri = await this._resolveWorkspaceUri(rel);
        if (!uri) {
            void vscode.window.showErrorMessage(`Could not resolve workspace path: ${rel}`);
            return;
        }
        const doc = await vscode.workspace.openTextDocument(uri);
        const end = doc.lineAt(doc.lineCount - 1).range.end;
        const fullRange = new vscode.Range(new vscode.Position(0, 0), end);
        const edit = new vscode.WorkspaceEdit();
        edit.replace(uri, fullRange, value.content);
        await vscode.workspace.applyEdit(edit);
        await vscode.window.showTextDocument(uri);
    }

    private async _resolveWorkspaceUri(relativeOrAbsolute: string): Promise<vscode.Uri | undefined> {
        if (path.isAbsolute(relativeOrAbsolute)) {
            return vscode.Uri.file(relativeOrAbsolute);
        }
        const folders = vscode.workspace.workspaceFolders;
        if (!folders?.length) return undefined;
        const norm = relativeOrAbsolute.replace(/\\/g, '/');
        for (const folder of folders) {
            const joined = path.join(folder.uri.fsPath, norm);
            try {
                await vscode.workspace.fs.stat(vscode.Uri.file(joined));
                return vscode.Uri.file(joined);
            } catch {
                /* try next workspace root */
            }
        }
        return vscode.Uri.file(path.join(folders[0].uri.fsPath, norm));
    }

    private _toWorkspaceRelative(fsPath: string): string {
        const folders = vscode.workspace.workspaceFolders;
        if (!folders?.length) return fsPath;
        const root = folders[0].uri.fsPath;
        const rel = path.relative(root, fsPath);
        if (rel.startsWith('..')) return fsPath;
        return rel.split(path.sep).join('/');
    }

    private async _sendFullState(): Promise<void> {
        if (!this._view) return;

        const caps: Record<string, unknown> = {
            mcpConnected: this._mcp.connected,
            toolCount: this._mcp.capabilities.size,
            schemaFingerprint: this._mcp.capabilities.schemaFingerprint,
            lastMcpError: this._mcp.capabilities.lastError ?? null,
            execution_mode: null,
            worker_runtime_attached: null,
            db_configured: null,
            event_feed_mode: null,
        };

        const status = await this._mcp.orchestratorStatus();
        if (status) {
            this.postMessage({ type: 'gamifyUpdate', value: status });
            const s = status as unknown as Record<string, unknown>;
            this.postMessage({ type: 'workflowStatus', value: s.planning ?? null });
            this.postMessage({ type: 'meshStatus', value: s.mesh_snapshot ?? null });
            const rawAgents = s.agents as
                | Array<{
                      id: number;
                      name: string;
                      queued: number;
                      completed?: number;
                      paused?: boolean;
                  }>
                | undefined;
            const intentions =
                Array.isArray(rawAgents) && rawAgents.length > 0
                    ? rawAgents.map((a) => ({
                          id: `agent-${a.id}`,
                          goal: `${a.name} — queue depth ${a.queued}`,
                          confidence: a.paused ? 0.35 : 0.82,
                          active: a.queued > 0,
                          branch: `orchestrator/agent/${a.id}`,
                          prompt_trace: JSON.stringify(a),
                          reliable: 0.85,
                      }))
                    : null;
            this.postMessage({ type: 'intentionMatrix', value: intentions });
            caps.execution_mode = s.execution_mode;
            caps.worker_runtime_attached = s.worker_runtime_attached;
            caps.db_configured = s.db_configured;
            caps.event_feed_mode = s.event_feed_mode;
        }

        this.postMessage({ type: 'capabilitiesUpdate', value: caps });

        const providers = await this._mcp.budgetStatus();
        if (providers) this.postMessage({ type: 'voxStatus', value: providers });

        const surface = await this._mcp.languageSurface();
        if (surface) this.postMessage({ type: 'languageSurface', value: surface });

        const pipeline = await this._mcp.pipelineStatus();
        if (pipeline) this.postMessage({ type: 'pipelineStatus', value: pipeline });

        const tasks = await this._mcp.a2aTasks();
        if (tasks) this.postMessage({ type: 'a2aTasks', value: tasks });

        const oplog = await this._mcp.oplog();
        if (oplog) this.postMessage({ type: 'oplog', value: oplog });

        const budgetHist = await this._mcp.budgetHistory(20);
        this.postMessage({ type: 'budgetHistory', value: budgetHist });

        const modelList = await this._mcp.modelList();
        this.postMessage({ type: 'modelList', value: modelList });

        this._sendAst();
        await this.maybePushLudusSnapshot();
    }

    /** Push Ludus MCP snapshot to the webview (throttled unless `force`). */
    public async refreshLudusSnapshot(force: boolean): Promise<void> {
        if (!this._view) return;
        if (!this._mcp.connected || !this._mcp.isToolAvailable('vox_ludus_progress_snapshot')) {
            return;
        }
        const now = Date.now();
        if (!force && now - this._lastLudusSnapshotPoll < LUDUS_SNAPSHOT_MIN_MS) {
            return;
        }
        this._lastLudusSnapshotPoll = now;
        const snap = await this._mcp.ludusProgressSnapshot();
        if (snap) {
            this.postMessage({ type: 'ludusProgressSnapshot', value: snap });
        }
    }

    private async maybePushLudusSnapshot(): Promise<void> {
        if (!ConfigManager.gamifyShowHud) return;
        await this.refreshLudusSnapshot(false);
    }

    private async _sendAst(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (editor && editor.document.languageId === 'vox') {
            const rel = this._toWorkspaceRelative(editor.document.uri.fsPath);
            const ast = await this._mcp.astInspect(rel);
            this.postMessage({ type: 'astResult', value: ast });
            this.postMessage({ type: 'activeEditorChanged', value: rel });
        }
    }

    public postMessage(msg: unknown): void {
        const validated = parseHostToWebviewMessage(msg);
        if (!validated) {
            this._mcp.outputChannel.appendLine(`[Vox] Dropped invalid host→webview message`);
            return;
        }
        this._view?.webview.postMessage(validated);
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
