/**
 * Host → webview messages (zod). Keep in sync with SidebarProvider.postMessage + extension agents/gamify.
 */
import { z } from 'zod';

function entry<T extends string>(type: T) {
    return z.object({
        type: z.literal(type),
        value: z.unknown().optional(),
    });
}

export const hostToWebviewSchema = z.discriminatedUnion('type', [
    entry('chatHistory'),
    entry('chatMeta'),
    entry('gamifyUpdate'),
    entry('workflowStatus'),
    entry('meshStatus'),
    entry('intentionMatrix'),
    entry('capabilitiesUpdate'),
    entry('workspaceContext'),
    entry('composerState'),
    entry('inspectorState'),
    entry('voxStatus'),
    entry('languageSurface'),
    entry('pipelineStatus'),
    entry('a2aTasks'),
    entry('oplog'),
    entry('budgetHistory'),
    entry('modelList'),
    entry('astResult'),
    entry('activeEditorChanged'),
    entry('agentsUpdate'),
    entry('planUpdate'),
    entry('planAdequacyQuestions'),
    entry('ludusProgressSnapshot'),
    entry('attentionStatus'),
    entry('attentionAlert'),
]);

export type HostToWebviewMessage = z.infer<typeof hostToWebviewSchema>;

export function parseHostToWebviewMessage(raw: unknown): HostToWebviewMessage | null {
    const r = hostToWebviewSchema.safeParse(raw);
    return r.success ? r.data : null;
}
