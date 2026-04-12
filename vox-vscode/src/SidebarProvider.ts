import * as vscode from 'vscode';
import * as path from 'path';
import { VoxMcpClient } from './core/VoxMcpClient';
import { WorkspaceContextEngine } from './context/WorkspaceContextEngine';
import { ChatController, SIDEBAR_CHAT_SESSION_ID } from './chat/ChatController';
import { VoxPreferenceKey, byokPreferenceKey } from './core/preferenceKeys';
import { ConfigManager } from './core/ConfigManager';
import { parseWebviewMessage } from './protocol/webviewMessages';
import { parseHostToWebviewMessage } from './protocol/hostToWebviewMessages';
import type {
    ComposerDraft,
    ComposerState,
    WorkspaceInspectorState,
} from './types';


const SIDEBAR_WS_DEBOUNCE_MS = 300;

const LUDUS_SNAPSHOT_MIN_MS = 3000;
const INSPECTOR_REFRESH_MS = 15000;

export class SidebarProvider implements vscode.WebviewViewProvider {
    private _view?: vscode.WebviewView;
    private _chatController: ChatController;
    private _contextEngine = new WorkspaceContextEngine();
    private _ws?: WebSocket;
    private _wsReconnectTimer?: NodeJS.Timeout;
    private _wsDebounceTimer?: NodeJS.Timeout;
    private _lastLudusSnapshotPoll = 0;
    private _lastInspectorRefresh = 0;
    private _composerState: ComposerState = {
        availableFiles: [],
        drafts: [],
        isGenerating: false,
        lastError: null,
    };
    private _inspectorState: WorkspaceInspectorState = {
        activeEditor: {
            filePath: '',
            line: 0,
            selectedText: '',
            languageId: '',
            diagnostics: [],
        },
        openFiles: [],
        contextKeys: [],
        lastPlan: null,
        lastChatMeta: null,
        browserState: null,
    };

    constructor(
        private readonly _extensionUri: vscode.Uri,
        private readonly _mcp: VoxMcpClient,
    ) {
        this._chatController = new ChatController(
            this._mcp,
            SIDEBAR_CHAT_SESSION_ID,
            (messages, meta) => {
                this.postMessage({ type: 'chatHistory', value: messages });
                if (meta) {
                    this._inspectorState.lastChatMeta = meta;
                    this.postMessage({ type: 'chatMeta', value: meta });
                    this.postMessage({ type: 'inspectorState', value: this._inspectorState });
                }
            },
        );
    }

    resolveWebviewView(view: vscode.WebviewView): void {
        this._view = view;
        view.webview.options = { enableScripts: true, localResourceRoots: [this._extensionUri] };
        view.webview.html = this._getHtml(view.webview);

        this._chatController.loadHistory();
        this._connectEventStream();

        view.webview.onDidReceiveMessage(async (msg: unknown) => {
            const parsed = parseWebviewMessage(msg);
            if (!parsed) return;
            switch (parsed.type) {
                case 'getInitialData':
                    await this._sendFullState();
                    break;
                case 'submitTask':
                    {
                        const activeEditor = this._contextEngine.getActiveEditorContext();
                    await this._chatController.submitMessage({
                        prompt: parsed.value.prompt,
                        contextFiles: parsed.value.contextFiles ?? [],
                        openFiles: this._contextEngine.getOpenFilePaths(),
                        activeFile: activeEditor.filePath,
                        activeLine: activeEditor.line,
                        selectedText: activeEditor.selectedText,
                        diagnostics: activeEditor.diagnostics,
                        sessionId: parsed.value.sessionId ?? SIDEBAR_CHAT_SESSION_ID,
                        cognitiveProfile: parsed.value.cognitiveProfile,
                    });
                    break;
                    }
                case 'composerGenerate':
                    await this._generateComposerDrafts(parsed.prompt, parsed.files);
                    break;
                case 'composerApply':
                    await this._applyComposerDrafts(parsed.paths);
                    break;
                case 'composerDiscard':
                    this._composerState = {
                        ...this._composerState,
                        drafts: this._composerState.drafts.filter((draft) => draft.path !== parsed.path),
                    };
                    this.postMessage({ type: 'composerState', value: this._composerState });
                    break;
                case 'composerDiscardAll':
                    this._composerState = { ...this._composerState, drafts: [], lastError: null };
                    this.postMessage({ type: 'composerState', value: this._composerState });
                    break;
                case 'refreshInspector':
                    await this._refreshInspectorData(true);
                    break;
                case 'inspectContextKey':
                    this._inspectorState.contextValue = await this._mcp.getContext(parsed.key);
                    this.postMessage({ type: 'inspectorState', value: this._inspectorState });
                    break;
                case 'contextSetValue':
                    await this._mcp.setContext({
                        agent_id: parsed.agentId,
                        key: parsed.key,
                        value: parsed.value,
                        ttl_seconds: parsed.ttlSeconds,
                    });
                    await this._refreshInspectorData(true);
                    break;
                case 'repoQueryText':
                    this._inspectorState.repoQueryResult = await this._mcp.repoQueryText({
                        query: parsed.query,
                        limit: parsed.limit ?? 8,
                    });
                    this.postMessage({ type: 'inspectorState', value: this._inspectorState });
                    break;
                case 'planGoalPreview': {
                    const planResult = await this._mcp.planGoal({
                        goal: parsed.goal,
                        write_to_disk: false,
                        max_tasks: 16,
                        plan_depth: parsed.depth ?? 'standard',
                        questioning_hints_enabled: true,
                        session_id: 'vscode-sidebar-plan-preview',
                    });
                    this._inspectorState.lastPlan = planResult;
                    if (
                        planResult?.plan_too_thin &&
                        Array.isArray(planResult.clarifying_questions) &&
                        planResult.clarifying_questions.length > 0
                    ) {
                        this.postMessage({
                            type: 'planAdequacyQuestions',
                            value: {
                                questions: planResult.clarifying_questions,
                                score: planResult.plan_adequacy_score,
                            },
                        });
                    }
                    this.postMessage({ type: 'inspectorState', value: this._inspectorState });
                    break;
                }
                case 'browserOpen':
                    await this._handleBrowserOpen(parsed.url);
                    break;
                case 'browserNavigate':
                    await this._handleBrowserNavigate(parsed.url);
                    break;
                case 'browserExtract':
                    await this._handleBrowserExtract(parsed.instruction);
                    break;
                case 'browserScreenshot':
                    await this._handleBrowserScreenshot(parsed.path);
                    break;
                case 'projectInit':
                    await this._mcp.projectInit({
                        project_name: parsed.projectName,
                        package_kind: parsed.packageKind,
                        template: parsed.template,
                        target_subdir: parsed.targetSubdir,
                    });
                    await this._refreshInspectorData(true);
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
                case 'setAgentBudget':
                    if (this._mcp.isToolAvailable('vox_set_agent_budget')) {
                        await this._mcp.setAgentBudget(
                            parsed.agentId,
                            parsed.maxTokens,
                            parsed.maxCostUsd
                        );
                        void vscode.window.showInformationMessage(`Budget for agent ${parsed.agentId} updated.`);
                    } else {
                        void vscode.window.showWarningMessage('Server does not support vox_set_agent_budget.');
                    }
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
                case 'rejectExecution': {
                    const id = parsed.intentId?.trim() ?? '';
                    const agentMatch = /^agent-(\d+)$/i.exec(id);
                    if (agentMatch) {
                        const agentId = Number(agentMatch[1]);
                        if (this._mcp.isToolAvailable('vox_drain_agent')) {
                            await this._mcp.drainAgent(agentId);
                            void vscode.window.showInformationMessage(
                                `Drained queued work for agent ${agentId} (Socrates reject).`,
                            );
                        } else {
                            void vscode.window.showInformationMessage(
                                'Reject execution requires `vox_drain_agent` on the connected MCP server.',
                            );
                        }
                    } else if (/^\d+$/.test(id)) {
                        if (this._mcp.isToolAvailable('vox_cancel_task')) {
                            await this._mcp.cancelTask(id);
                            void vscode.window.showInformationMessage(`Cancelled task ${id}.`);
                        } else {
                            void vscode.window.showInformationMessage(
                                'Reject execution requires `vox_cancel_task` on the connected MCP server.',
                            );
                        }
                    } else {
                        void vscode.window.showErrorMessage(
                            `Cannot reject execution: unrecognized intent id "${id || '(empty)'}" (expected agent-N or numeric task id).`,
                        );
                    }
                    break;
                }
                case 'doubtTask': {
                    if (this._mcp.isToolAvailable('vox_doubt_task')) {
                        await this._mcp.doubtTask(parsed.taskId);
                        void vscode.window.showInformationMessage(`Doubt flagged for task ${parsed.taskId}. Audit initiated.`);
                    } else {
                         void vscode.window.showInformationMessage('Doubt requires `vox_doubt_task` on the connected MCP server.');
                    }
                    break;
                }
                case 'runTerminalCommand': {
                    let terminal = vscode.window.terminals.find(t => t.name === 'Vox Backend');
                    if (!terminal) {
                        terminal = vscode.window.createTerminal('Vox Backend');
                    }
                    terminal.show();
                    terminal.sendText(parsed.value);
                    break;
                }
                case 'restartMcpServer': {
                    void vscode.window.showInformationMessage('Restarting Vox Extension (and MCP Connection)...');
                    vscode.commands.executeCommand('workbench.action.reloadWindow');
                    break;
                }
                case 'setAttentionPreference':
                    await this._mcp.preferenceSet(parsed.key, parsed.value);
                    await this._sendFullState();
                    break;
                case 'attentionReset':
                    await this._mcp.attentionReset('reset', parsed.newMaxMs);
                    await this._sendFullState();
                    break;
                case 'trustOverride':
                    await this._mcp.trustOverride(parsed.agentId, parsed.tier, parsed.reason);
                    await this._sendFullState();
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
                case 'emergencyStop':
                    if (this._mcp.isToolAvailable('vox_emergency_stop')) {
                        await this._mcp.emergencyStop('VS Code User triggered Stop All');
                        void vscode.window.showInformationMessage('Emergency stop triggered.');
                    } else {
                        void vscode.window.showInformationMessage(
                            'Emergency stop requires `vox_emergency_stop` on the connected MCP server.',
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
                case 'agentPause':
                    if (this._mcp.isToolAvailable('vox_pause_agent')) {
                        await this._mcp.pauseAgent(parsed.agentId);
                    } else {
                        void vscode.window.showInformationMessage(
                            'Pause requires `vox_pause_agent` on the connected MCP server.',
                        );
                    }
                    break;
                case 'agentResume':
                    if (this._mcp.isToolAvailable('vox_resume_agent')) {
                        await this._mcp.resumeAgent(parsed.agentId);
                    } else {
                        void vscode.window.showInformationMessage(
                            'Resume requires `vox_resume_agent` on the connected MCP server.',
                        );
                    }
                    break;
                case 'agentDrain':
                    if (this._mcp.isToolAvailable('vox_drain_agent')) {
                        await this._mcp.drainAgent(parsed.agentId);
                    } else {
                        void vscode.window.showInformationMessage(
                            'Drain requires `vox_drain_agent` on the connected MCP server.',
                        );
                    }
                    break;
                case 'agentRetire':
                    if (this._mcp.isToolAvailable('vox_retire_agent')) {
                        await this._mcp.retireAgent(parsed.agentId);
                    } else {
                        void vscode.window.showInformationMessage(
                            'Retire requires `vox_retire_agent` on the connected MCP server.',
                        );
                    }
                    break;
            }
        });

        vscode.window.onDidChangeActiveTextEditor(() => {
            this._sendAst();
            void this._sendWorkspaceContext();
        });
        vscode.window.onDidChangeVisibleTextEditors(() => {
            void this._sendWorkspaceContext();
        });
    }

    private _connectEventStream(): void {
        clearTimeout(this._wsReconnectTimer);
        const port = vscode.workspace.getConfiguration('vox').get<number>('mcp.httpPort') || 3921;
        const wsUrl = `ws://127.0.0.1:${port}/v1/ws`;
        try {
            this._ws = new globalThis.WebSocket(wsUrl);
            this._ws.onopen = () => {
                // Push initial full state on connect
                void this._sendFullState();
            };
            this._ws.onmessage = (event) => {
                // Debounce rapid event bursts into a single state refresh
                clearTimeout(this._wsDebounceTimer);
                this._wsDebounceTimer = setTimeout(() => {
                    void this._sendFullState();
                }, SIDEBAR_WS_DEBOUNCE_MS);

                // Check for high-signal events push from server directly
                if (typeof event.data === 'string') {
                    try {
                        const evtData = JSON.parse(event.data);
                        const kind = evtData.kind || evtData.type;
                        const payload = evtData.payload || evtData;

                        if (kind === 'AttentionBudgetAlert') {
                            const signal = payload.signal || (payload.focus_depth === 'Deep' ? 'critical' : 'high');
                            const spentRatio = payload.spent_ratio || payload.spent_ratio_estimate || 1.0;
                            
                            this.postMessage({
                                type: 'attentionAlert',
                                value: { signal, spent_ratio: spentRatio }
                            });

                            const level = vscode.workspace.getConfiguration('vox').get<string>('attention.notificationLevel') ?? 'high';
                            if (signal === 'critical' && (level === 'high' || level === 'critical')) {
                                void vscode.window.showWarningMessage(
                                    `🚨 Vox Attention Critical: Exhausted or Deep focus.`,
                                    'Reset Session', 'Dismiss'
                                ).then(choice => {
                                    if (choice === 'Reset Session') {
                                        void this._mcp.attentionReset('reset');
                                        void this._sendFullState();
                                    }
                                });
                            }
                        } else if (kind === 'task_doubted') {
                            void vscode.window.showInformationMessage(`🚩 [SUSPECT] Task ${payload.task_id} flagged for audit. Resolution Agent inbound.`);
                            this.postMessage({ type: 'playSound', value: 'doubt_start' });
                            this.postMessage({ type: 'taskDoubted', value: { taskId: payload.task_id } });
                        } else if (kind === 'task_resolved') {
                            const validated = payload.validated;
                            const report = payload.report;
                            if (validated) {
                                void vscode.window.showInformationMessage(`✅ [VALIDATED] Task ${payload.task_id} audit complete. AI was correct.`);
                                this.postMessage({ type: 'playSound', value: 'validated' });
                            } else {
                                void vscode.window.showWarningMessage(`❌ [OVERRULED] Task ${payload.task_id} audit complete. Hallucination caught!`);
                                this.postMessage({ type: 'playSound', value: 'overruled' });
                            }
                            this.postMessage({ type: 'taskResolved', value: { taskId: payload.task_id, validated, report } });
                        } else if (kind === 'achievement_earned' || kind === 'achievementEarned') {
                             const achievement = payload.achievement;
                             this.postMessage({ type: 'achievementEarned', value: achievement });
                             this.postMessage({ type: 'playSound', value: 'achievement' });
                        }
                    } catch (e) {
                        // ignore malformed JSON or non-vox event structures
                    }
                }
            };
            this._ws.onclose = () => {
                this._wsReconnectTimer = setTimeout(() => this._connectEventStream(), 5000);
            };
            this._ws.onerror = () => {
                this._ws?.close();
            };
        } catch {
            this._wsReconnectTimer = setTimeout(() => this._connectEventStream(), 5000);
        }
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

    private async _generateComposerDrafts(prompt: string, files: string[]): Promise<void> {
        const cleanPrompt = prompt.trim();
        const targets = [...new Set(files.map((file) => file.trim()).filter(Boolean))];
        if (!cleanPrompt || targets.length === 0) {
            this._composerState = {
                ...this._composerState,
                lastError: 'Choose at least one file and provide an instruction.',
            };
            this.postMessage({ type: 'composerState', value: this._composerState });
            return;
        }
        this._composerState = {
            ...this._composerState,
            isGenerating: true,
            lastPrompt: cleanPrompt,
            lastError: null,
        };
        this.postMessage({ type: 'composerState', value: this._composerState });

        const drafts: ComposerDraft[] = [];
        for (const file of targets) {
            const uri = await this._resolveWorkspaceUri(file);
            if (!uri) {
                continue;
            }
            const doc = await vscode.workspace.openTextDocument(uri);
            const fullText = doc.getText();
            const lastLine = Math.max(0, doc.lineCount - 1);
            const result = (await this._mcp.inlineEdit({
                prompt: cleanPrompt,
                file,
                start_line: 1,
                end_line: Math.max(1, doc.lineCount),
                current_text: fullText,
                language: doc.languageId,
                context_before: '',
                context_after: '',
                session_id: 'vscode-sidebar-composer',
            })) as
                | {
                      replacement?: string;
                      explanation?: string;
                      tokens?: number;
                      model_used?: string;
                  }
                | null;
            if (!result?.replacement || result.replacement === fullText) {
                continue;
            }
            drafts.push({
                path: file,
                language: doc.languageId || this._languageFromPath(file),
                original: fullText,
                proposed: result.replacement,
                explanation: result.explanation,
                tokens: result.tokens,
                model_used: result.model_used,
            });
            if (lastLine >= 0) {
                await vscode.window.showTextDocument(doc, { preview: false, preserveFocus: true });
            }
        }

        this._composerState = {
            ...this._composerState,
            drafts,
            isGenerating: false,
            snapshotRequested: drafts.length > 0,
            lastError: drafts.length === 0 ? 'No staged edits were produced for the selected files.' : null,
        };
        this.postMessage({ type: 'composerState', value: this._composerState });
    }

    private async _applyComposerDrafts(paths: string[]): Promise<void> {
        const selected = new Set(paths);
        const drafts = this._composerState.drafts.filter((draft) =>
            selected.size === 0 ? true : selected.has(draft.path),
        );
        if (drafts.length === 0) {
            return;
        }

        // Reuse the extension's existing VCS affordance to request a rollback point before apply.
        await this._mcp.submitTask(
            `[vcs] snapshot: composer apply (${drafts.length} files)`,
            drafts.map((draft) => draft.path),
            'plan',
        );

        for (const draft of drafts) {
            await this._applyChanges({ path: draft.path, content: draft.proposed });
        }

        this._composerState = {
            ...this._composerState,
            drafts: this._composerState.drafts.filter((draft) => !drafts.some((picked) => picked.path === draft.path)),
            lastAppliedPaths: drafts.map((draft) => draft.path),
            snapshotRequested: false,
            lastError: null,
        };
        this.postMessage({ type: 'composerState', value: this._composerState });
    }

    private _languageFromPath(file: string): string {
        const ext = path.extname(file).replace(/^\./, '');
        return ext || 'text';
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

    private async _sendWorkspaceContext(): Promise<void> {
        const activeEditor = this._contextEngine.getActiveEditorContext();
        const openFiles = this._contextEngine.getOpenFilePaths();
        const availableFiles = [...new Set([activeEditor.filePath, ...openFiles].filter(Boolean))];
        this._composerState = { ...this._composerState, availableFiles };
        this._inspectorState = {
            ...this._inspectorState,
            activeEditor,
            openFiles,
        };
        this.postMessage({ type: 'workspaceContext', value: { activeEditor, openFiles } });
        this.postMessage({ type: 'composerState', value: this._composerState });
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _refreshInspectorData(force: boolean): Promise<void> {
        if (!this._view) return;
        const now = Date.now();
        if (!force && now - this._lastInspectorRefresh < INSPECTOR_REFRESH_MS) {
            this.postMessage({ type: 'inspectorState', value: this._inspectorState });
            return;
        }
        this._lastInspectorRefresh = now;
        this._inspectorState = {
            ...this._inspectorState,
            repoIndexStatus: await this._mcp.repoIndexStatus(),
            repoCatalog: await this._mcp.repoCatalogList(),
            capabilityManifest: await this._mcp.capabilityModelManifest(),
            contextKeys: await this._mcp.listContext(''),
            lastUpdatedAt: now,
        };
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _handleBrowserOpen(url: string): Promise<void> {
        const result = (await this._mcp.browserOpen(url)) as Record<string, unknown> | null;
        const pageId =
            typeof result?.page_id === 'string' || typeof result?.page_id === 'number'
                ? result.page_id
                : typeof result?.pageId === 'string' || typeof result?.pageId === 'number'
                  ? result.pageId
                  : null;
        this._inspectorState.browserState = {
            ...(this._inspectorState.browserState ?? {}),
            pageId,
            url,
            lastAction: 'open',
            lastError: result ? null : 'Browser open failed.',
        };
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _handleBrowserNavigate(url: string): Promise<void> {
        const pageId = this._inspectorState.browserState?.pageId;
        if (pageId == null) {
            this._inspectorState.browserState = {
                ...(this._inspectorState.browserState ?? {}),
                lastError: 'Open a browser page first.',
            };
            this.postMessage({ type: 'inspectorState', value: this._inspectorState });
            return;
        }
        const result = await this._mcp.browserGoto(pageId, url);
        this._inspectorState.browserState = {
            ...(this._inspectorState.browserState ?? {}),
            url,
            lastAction: 'goto',
            lastError: result ? null : 'Browser navigation failed.',
        };
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _handleBrowserExtract(instruction: string): Promise<void> {
        const pageId = this._inspectorState.browserState?.pageId;
        if (pageId == null) {
            this._inspectorState.browserState = {
                ...(this._inspectorState.browserState ?? {}),
                lastError: 'Open a browser page first.',
            };
            this.postMessage({ type: 'inspectorState', value: this._inspectorState });
            return;
        }
        const result = await this._mcp.browserExtract({ page_id: pageId, instruction });
        this._inspectorState.browserState = {
            ...(this._inspectorState.browserState ?? {}),
            lastExtract: result,
            lastAction: 'extract',
            lastError: result ? null : 'Browser extract failed.',
        };
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _handleBrowserScreenshot(filePath: string): Promise<void> {
        const pageId = this._inspectorState.browserState?.pageId;
        if (pageId == null) {
            this._inspectorState.browserState = {
                ...(this._inspectorState.browserState ?? {}),
                lastError: 'Open a browser page first.',
            };
            this.postMessage({ type: 'inspectorState', value: this._inspectorState });
            return;
        }
        const result = await this._mcp.browserScreenshot({ page_id: pageId, path: filePath });
        this._inspectorState.browserState = {
            ...(this._inspectorState.browserState ?? {}),
            lastScreenshotPath: filePath,
            lastAction: 'screenshot',
            lastError: result ? null : 'Browser screenshot failed.',
        };
        this.postMessage({ type: 'inspectorState', value: this._inspectorState });
    }

    private async _sendFullState(): Promise<void> {
        if (!this._view) return;
        await this._sendWorkspaceContext();

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
        await this._refreshInspectorData(false);
        await this.maybePushLudusSnapshot();

        // Push Attention Status if panel visible
        const showPanel = vscode.workspace.getConfiguration('vox').get<boolean>('attention.panelVisible') ?? true;
        if (showPanel && this._mcp.isToolAvailable('vox_attention_status')) {
            const att = await this._mcp.attentionStatus();
            if (att) {
                this.postMessage({ type: 'attentionStatus', value: att });
            }
        }
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
