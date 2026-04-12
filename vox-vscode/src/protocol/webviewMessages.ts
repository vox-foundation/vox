/**
 * Webview → host messages (zod structural check + normalization).
 */
import { z } from 'zod';

const incomingSchema = z.union([
    z.object({ type: z.literal('getInitialData') }),
    z.object({ type: z.literal('pickModel') }),
    z.object({ type: z.literal('rebalance') }),
    z.object({
        type: z.literal('submitTask'),
        value: z.union([
            z.string(),
            z.object({
                prompt: z.string(),
                contextFiles: z.array(z.string()).optional(),
                sessionId: z.string().optional(),
                cognitiveProfile: z.enum(['fast', 'reasoning', 'creative']).optional(),
            }),
        ]).optional(),
    }),
    z.object({ type: z.literal('applyChanges'), value: z.unknown().optional() }),
    z.object({
        type: z.literal('composerGenerate'),
        prompt: z.string().optional(),
        files: z.array(z.string()).optional(),
    }),
    z.object({
        type: z.literal('composerApply'),
        paths: z.array(z.string()).optional(),
    }),
    z.object({
        type: z.literal('composerDiscard'),
        path: z.string().optional(),
    }),
    z.object({ type: z.literal('composerDiscardAll') }),
    z.object({ type: z.literal('refreshInspector') }),
    z.object({
        type: z.literal('inspectContextKey'),
        key: z.string().optional(),
    }),
    z.object({
        type: z.literal('contextSetValue'),
        agentId: z.number().int().min(0).optional(),
        key: z.string().optional(),
        value: z.string().optional(),
        ttlSeconds: z.number().int().min(0).optional(),
    }),
    z.object({
        type: z.literal('repoQueryText'),
        query: z.string().optional(),
        limit: z.number().int().min(1).max(100).optional(),
    }),
    z.object({
        type: z.literal('planGoalPreview'),
        goal: z.string().optional(),
        depth: z.enum(['minimal', 'standard', 'deep']).optional(),
    }),
    z.object({
        type: z.literal('browserOpen'),
        url: z.string().optional(),
    }),
    z.object({
        type: z.literal('browserNavigate'),
        url: z.string().optional(),
    }),
    z.object({
        type: z.literal('browserExtract'),
        instruction: z.string().optional(),
    }),
    z.object({
        type: z.literal('browserScreenshot'),
        path: z.string().optional(),
    }),
    z.object({
        type: z.literal('projectInit'),
        projectName: z.string().optional(),
        packageKind: z.string().optional(),
        template: z.string().optional(),
        targetSubdir: z.string().optional(),
    }),
    z.object({
        type: z.literal('updateBudgetCap'),
        value: z.union([z.number(), z.string()]).optional(),
    }),
    z.object({
        type: z.literal('setAgentBudget'),
        agentId: z.number().int().min(0),
        maxTokens: z.number().min(0).optional(),
        maxCostUsd: z.number().min(0).optional(),
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
    z.object({
        type: z.literal('ludusAckNotification'),
        notificationId: z.string().optional(),
    }),
    z.object({ type: z.literal('ludusAckAllNotifications') }),
    z.object({ type: z.literal('ludusRefreshSnapshot') }),
    z.object({ type: z.literal('runTerminalCommand'), value: z.string() }),
    z.object({ type: z.literal('restartMcpServer') }),
    z.object({ type: z.literal('emergencyStop') }),
    z.object({
        type: z.literal('setAttentionPreference'),
        key: z.string(),
        value: z.string(),
    }),
    z.object({
        type: z.literal('attentionReset'),
        newMaxMs: z.number().optional(),
    }),
    z.object({
        type: z.literal('trustOverride'),
        agentId: z.number().int(),
        tier: z.string(),
        reason: z.string(),
    }),
    z.object({
        type: z.literal('doubtTask'),
        taskId: z.string(),
    }),
]);

export type WebviewToHostMessage =
    | { type: 'getInitialData' }
    | {
          type: 'submitTask';
          value: {
              prompt: string;
              contextFiles?: string[];
              sessionId?: string;
              cognitiveProfile?: 'fast' | 'reasoning' | 'creative';
          };
      }
    | { type: 'applyChanges'; value: { path: string; content: string } | null | undefined }
    | { type: 'composerGenerate'; prompt: string; files: string[] }
    | { type: 'composerApply'; paths: string[] }
    | { type: 'composerDiscard'; path: string }
    | { type: 'composerDiscardAll' }
    | { type: 'refreshInspector' }
    | { type: 'inspectContextKey'; key: string }
    | { type: 'contextSetValue'; agentId: number; key: string; value: string; ttlSeconds?: number }
    | { type: 'repoQueryText'; query: string; limit?: number }
    | { type: 'planGoalPreview'; goal: string; depth?: 'minimal' | 'standard' | 'deep' }
    | { type: 'browserOpen'; url: string }
    | { type: 'browserNavigate'; url: string }
    | { type: 'browserExtract'; instruction: string }
    | { type: 'browserScreenshot'; path: string }
    | {
          type: 'projectInit';
          projectName: string;
          packageKind?: string;
          template?: string;
          targetSubdir?: string;
      }
    | { type: 'pickModel' }
    | { type: 'updateBudgetCap'; value: number | string }
    | { type: 'setAgentBudget'; agentId: number; maxTokens?: number; maxCostUsd?: number }
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
    | { type: 'ludusRefreshSnapshot' }
    | { type: 'runTerminalCommand'; value: string }
    | { type: 'restartMcpServer' }
    | { type: 'emergencyStop' }
    | { type: 'setAttentionPreference'; key: string; value: string }
    | { type: 'attentionReset'; newMaxMs?: number }
    | { type: 'trustOverride'; agentId: number; tier: string; reason: string }
    | { type: 'doubtTask'; taskId: string };

export function parseWebviewMessage(raw: unknown): WebviewToHostMessage | null {
    const r = incomingSchema.safeParse(raw);
    if (!r.success) return null;
    const o = r.data;
    switch (o.type) {
        case 'getInitialData':
        case 'pickModel':
        case 'rebalance':
        case 'emergencyStop':
            return { type: o.type };
        case 'submitTask':
            if (typeof o.value === 'string') {
                return { type: 'submitTask', value: { prompt: o.value } };
            }
            if (o.value && typeof o.value === 'object' && typeof o.value.prompt === 'string') {
                return {
                    type: 'submitTask',
                    value: {
                        prompt: o.value.prompt,
                        contextFiles: Array.isArray(o.value.contextFiles)
                            ? o.value.contextFiles.filter((v): v is string => typeof v === 'string')
                            : [],
                        sessionId: typeof o.value.sessionId === 'string' ? o.value.sessionId : undefined,
                        cognitiveProfile: o.value.cognitiveProfile,
                    },
                };
            }
            return null;
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
        case 'composerGenerate':
            return {
                type: 'composerGenerate',
                prompt: typeof o.prompt === 'string' ? o.prompt : '',
                files: Array.isArray(o.files) ? o.files.filter((v): v is string => typeof v === 'string') : [],
            };
        case 'composerApply':
            return {
                type: 'composerApply',
                paths: Array.isArray(o.paths) ? o.paths.filter((v): v is string => typeof v === 'string') : [],
            };
        case 'composerDiscard':
            return typeof o.path === 'string' ? { type: 'composerDiscard', path: o.path } : null;
        case 'composerDiscardAll':
            return { type: 'composerDiscardAll' };
        case 'refreshInspector':
            return { type: 'refreshInspector' };
        case 'inspectContextKey':
            return typeof o.key === 'string' ? { type: 'inspectContextKey', key: o.key } : null;
        case 'contextSetValue':
            if (typeof o.agentId !== 'number' || typeof o.key !== 'string' || typeof o.value !== 'string') {
                return null;
            }
            return {
                type: 'contextSetValue',
                agentId: o.agentId,
                key: o.key,
                value: o.value,
                ttlSeconds: o.ttlSeconds,
            };
        case 'repoQueryText':
            return typeof o.query === 'string' ? { type: 'repoQueryText', query: o.query, limit: o.limit } : null;
        case 'planGoalPreview':
            return typeof o.goal === 'string' ? { type: 'planGoalPreview', goal: o.goal, depth: o.depth } : null;
        case 'browserOpen':
            return typeof o.url === 'string' ? { type: 'browserOpen', url: o.url } : null;
        case 'browserNavigate':
            return typeof o.url === 'string' ? { type: 'browserNavigate', url: o.url } : null;
        case 'browserExtract':
            return typeof o.instruction === 'string' ? { type: 'browserExtract', instruction: o.instruction } : null;
        case 'browserScreenshot':
            return typeof o.path === 'string' ? { type: 'browserScreenshot', path: o.path } : null;
        case 'projectInit':
            return typeof o.projectName === 'string'
                ? {
                      type: 'projectInit',
                      projectName: o.projectName,
                      packageKind: typeof o.packageKind === 'string' ? o.packageKind : undefined,
                      template: typeof o.template === 'string' ? o.template : undefined,
                      targetSubdir: typeof o.targetSubdir === 'string' ? o.targetSubdir : undefined,
                  }
                : null;
        case 'updateBudgetCap':
            if (o.value === undefined) return null;
            return { type: 'updateBudgetCap', value: o.value };
        case 'setAgentBudget':
            return {
                type: 'setAgentBudget',
                agentId: o.agentId,
                maxTokens: o.maxTokens,
                maxCostUsd: o.maxCostUsd,
            };
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
        case 'runTerminalCommand':
            return { type: 'runTerminalCommand', value: typeof o.value === 'string' ? o.value : '' };
        case 'restartMcpServer':
            return { type: 'restartMcpServer' };
        case 'setAttentionPreference':
            if (typeof o.key === 'string' && typeof o.value === 'string') {
                return { type: 'setAttentionPreference', key: o.key, value: o.value };
            }
            return null;
        case 'attentionReset':
            return { type: 'attentionReset', newMaxMs: typeof o.newMaxMs === 'number' ? o.newMaxMs : undefined };
        case 'trustOverride':
            if (typeof o.agentId === 'number' && typeof o.tier === 'string' && typeof o.reason === 'string') {
                return { type: 'trustOverride', agentId: o.agentId, tier: o.tier, reason: o.reason };
            }
            return null;
        case 'doubtTask':
            return typeof o.taskId === 'string' ? { type: 'doubtTask', taskId: o.taskId } : null;
        default:
            return null;
    }
}
