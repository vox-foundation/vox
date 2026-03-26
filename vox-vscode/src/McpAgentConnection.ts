import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import * as vscode from 'vscode';
import { parseMcpToolResult, unwrapVoxToolEnvelope } from './core/mcpToolResult';

const DEFAULT_FILES = [{ path: '.', access: 'read' as const }];

/** Standalone stdio MCP client (legacy / tests); prefer {@link VoxMcpClient} for the extension. */
export class McpAgentConnection {
    private client: Client;
    private seenEventIds = new Set<string>();
    private transport: StdioClientTransport;
    private pollTimer?: ReturnType<typeof setInterval>;

    constructor(
        private outputChannel: vscode.OutputChannel,
        serverPath: string = 'vox',
        private onEvent?: (event: unknown) => void,
    ) {
        this.transport = new StdioClientTransport({
            command: serverPath,
            args: ['mcp'],
        });

        this.client = new Client(
            {
                name: 'vox-vscode-mcp-agent',
                version: '0.2.0',
            },
            {
                capabilities: {},
            },
        );

        this.client.fallbackNotificationHandler = async (notification) => {
            this.outputChannel.appendLine(`Received Notification: ${JSON.stringify(notification)}`);
            this.onEvent?.(notification);
        };
    }

    async connect(): Promise<void> {
        try {
            this.outputChannel.appendLine('Connecting to Vox MCP Server...');
            await this.client.connect(this.transport);
            this.outputChannel.appendLine('Connected to Vox MCP Server!');

            const toolsResponse = await this.client.listTools();
            this.outputChannel.appendLine(`Found ${toolsResponse.tools.length} MCP tools.`);

            if (this.pollTimer) clearInterval(this.pollTimer);
            this.pollTimer = setInterval(() => void this.pollEvents(), 3000);
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            this.outputChannel.appendLine(`Failed to connect to MCP: ${msg}`);
        }
    }

    dispose(): void {
        if (this.pollTimer) clearInterval(this.pollTimer);
        void this.client.close();
    }

    private async pollEvents(): Promise<void> {
        if (!this.onEvent) return;
        try {
            const [eventsResult, budgetResult] = await Promise.all([
                this.client.callTool({ name: 'vox_poll_events', arguments: { limit: 10 } }).catch(() => null),
                this.client.callTool({ name: 'vox_budget_status', arguments: {} }).catch(() => null),
            ]);

            if (eventsResult) {
                const parsed = parseMcpToolResult(eventsResult);
                const unwrapped = unwrapVoxToolEnvelope(parsed, this.outputChannel, 'vox_poll_events');
                if (Array.isArray(unwrapped)) {
                    for (const ev of unwrapped.reverse()) {
                        const rec = ev as { id?: unknown; agent_id?: unknown; timestamp?: unknown; event_type?: unknown; type?: unknown };
                        const id =
                            rec.id !== undefined && rec.id !== null
                                ? String(rec.id)
                                : `${rec.agent_id}-${rec.timestamp}-${rec.event_type ?? rec.type}`;
                        if (!this.seenEventIds.has(id)) {
                            this.seenEventIds.add(id);
                            this.onEvent(ev);
                        }
                    }
                }
            }

            if (budgetResult) {
                const parsed = parseMcpToolResult(budgetResult);
                const unwrapped = unwrapVoxToolEnvelope(parsed, this.outputChannel, 'vox_budget_status');
                this.onEvent({ type: 'budget_status', data: unwrapped });
            }
        } catch {
            /* poll is best-effort */
        }
    }

    async submitTask(task: string) {
        try {
            const result = await this.client.callTool({
                name: 'vox_submit_task',
                arguments: {
                    description: task,
                    files: DEFAULT_FILES,
                },
            });
            return result;
        } catch (e: unknown) {
            const msg = e instanceof Error ? e.message : String(e);
            this.outputChannel.appendLine(`Error submitting task: ${msg}`);
            return null;
        }
    }
}
