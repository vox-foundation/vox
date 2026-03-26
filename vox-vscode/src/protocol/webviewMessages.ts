/**
 * Webview → host messages (zod structural check + normalization).
 */
import { z } from 'zod';

const incomingSchema = z.union([
    z.object({ type: z.literal('getInitialData') }),
    z.object({ type: z.literal('pickModel') }),
    z.object({ type: z.literal('rebalance') }),
    z.object({ type: z.literal('submitTask'), value: z.string().optional() }),
    z.object({ type: z.literal('applyChanges'), value: z.unknown().optional() }),
    z.object({
        type: z.literal('updateBudgetCap'),
        value: z.union([z.number(), z.string()]).optional(),
    }),
    z.object({ type: z.literal('updateApiKey'), provider: z.string().optional(), value: z.string().optional() }),
    z.object({ type: z.literal('setModel'), value: z.string().optional() }),
    z.object({
        type: z.literal('resumeWorkflow'),
        step: z.union([z.string(), z.number()]).optional(),
    }),
    z.object({ type: z.literal('setSocratesGate'), enforce: z.boolean().optional() }),
    z.object({ type: z.literal('rejectExecution'), intentId: z.string().optional() }),
    z.object({ type: z.literal('runCommand'), value: z.string().optional() }),
    z.object({ type: z.literal('agentRetire'), agentId: z.number().int().min(0) }),
    z.object({ type: z.literal('agentPause'), agentId: z.number().int().min(0) }),
    z.object({ type: z.literal('agentResume'), agentId: z.number().int().min(0) }),
    z.object({ type: z.literal('agentDrain'), agentId: z.number().int().min(0) }),
]);

export type WebviewToHostMessage =
    | { type: 'getInitialData' }
    | { type: 'submitTask'; value: string }
    | { type: 'applyChanges'; value: { path: string; content: string } | null | undefined }
    | { type: 'pickModel' }
    | { type: 'updateBudgetCap'; value: number | string }
    | { type: 'updateApiKey'; provider: string; value: string }
    | { type: 'setModel'; value: string }
    | { type: 'resumeWorkflow'; step?: string | number }
    | { type: 'setSocratesGate'; enforce: boolean }
    | { type: 'rejectExecution'; intentId?: string }
    | { type: 'rebalance' }
    | { type: 'runCommand'; value: string }
    | { type: 'agentRetire'; agentId: number }
    | { type: 'agentPause'; agentId: number }
    | { type: 'agentResume'; agentId: number }
    | { type: 'agentDrain'; agentId: number }
    | { type: 'ludusAckNotification'; notificationId: string }
    | { type: 'ludusAckAllNotifications' }
    | { type: 'ludusRefreshSnapshot' };

export function parseWebviewMessage(raw: unknown): WebviewToHostMessage | null {
    const r = incomingSchema.safeParse(raw);
    if (!r.success) return null;
    const o = r.data;
    switch (o.type) {
        case 'getInitialData':
        case 'pickModel':
        case 'rebalance':
            return { type: o.type };
        case 'submitTask':
            return { type: 'submitTask', value: typeof o.value === 'string' ? o.value : '' };
        case 'applyChanges': {
            const v = o.value;
            if (v && typeof v === 'object' && typeof (v as { path?: unknown }).path === 'string') {
                return {
                    type: 'applyChanges',
                    value: {
                        path: (v as { path: string }).path,
                        content: String((v as { content?: unknown }).content ?? ''),
                    },
                };
            }
            return { type: 'applyChanges', value: undefined };
        }
        case 'updateBudgetCap':
            if (o.value === undefined) return null;
            return { type: 'updateBudgetCap', value: o.value };
        case 'updateApiKey': {
            const p = o.provider;
            if (typeof p !== 'string') return null;
            return { type: 'updateApiKey', provider: p, value: typeof o.value === 'string' ? o.value : '' };
        }
        case 'setModel':
            return { type: 'setModel', value: typeof o.value === 'string' ? o.value : '' };
        case 'resumeWorkflow': {
            const st = o.step;
            if (st === undefined) return { type: 'resumeWorkflow', step: undefined };
            return { type: 'resumeWorkflow', step: typeof st === 'number' ? st : String(st) };
        }
        case 'setSocratesGate':
            return { type: 'setSocratesGate', enforce: !!o.enforce };
        case 'rejectExecution':
            return {
                type: 'rejectExecution',
                intentId: typeof o.intentId === 'string' ? o.intentId : undefined,
            };
        case 'runCommand':
            return { type: 'runCommand', value: typeof o.value === 'string' ? o.value : '' };
        case 'agentRetire':
            return { type: 'agentRetire', agentId: o.agentId };
        case 'agentPause':
            return { type: 'agentPause', agentId: o.agentId };
        case 'agentResume':
            return { type: 'agentResume', agentId: o.agentId };
        case 'agentDrain':
            return { type: 'agentDrain', agentId: o.agentId };
        case 'ludusAckNotification': {
            const id = o.notificationId;
            if (typeof id !== 'string' || !id.trim()) return null;
            return { type: 'ludusAckNotification', notificationId: id.trim() };
        }
        case 'ludusAckAllNotifications':
            return { type: 'ludusAckAllNotifications' };
        case 'ludusRefreshSnapshot':
            return { type: 'ludusRefreshSnapshot' };
        default:
            return null;
    }
}
