import { voxTransport } from '../transport';

export function getVsCodeApi() {
    return {
        postMessage: (msg: any) => {
            const type = msg.type;
            if (type === 'agentPause') {
                voxTransport.callTool('vox_pause_agent', { agent_id: msg.agentId });
            } else if (type === 'agentResume') {
                voxTransport.callTool('vox_resume_agent', { agent_id: msg.agentId });
            } else if (type === 'agentDrain') {
                voxTransport.callTool('vox_drain_agent', { agent_id: msg.agentId });
            } else if (type === 'agentRetire') {
                voxTransport.callTool('vox_retire_agent', { agent_id: msg.agentId });
            } else if (type === 'setAgentBudget') {
                voxTransport.callTool('vox_set_agent_budget', { agent_id: msg.agentId, max_cost_usd: msg.maxCostUsd });
            } else if (type === 'doubtTask') {
                voxTransport.callTool('vox_doubt_task', { task_id: msg.taskId });
            } else if (type === 'cancelTask') {
                voxTransport.callTool('vox_cancel_task', { task_id: msg.taskId });
            } else if (type === 'rebalance') {
                voxTransport.callTool('vox_rebalance', {});
            } else if (type === 'emergencyStop') {
                voxTransport.callTool('vox_emergency_stop', {});
            } else if (type === 'ludusAckAllNotifications') {
                voxTransport.callTool('vox_ludus_notifications_ack_all', {});
            } else if (type === 'ludusAckNotification') {
                voxTransport.callTool('vox_ludus_notification_ack', { notification_id: msg.notificationId });
            } else if (type === 'setAttentionPreference') {
                voxTransport.callTool('vox_preference_set', { key: msg.key, value: msg.value });
            } else if (type === 'attentionReset') {
                voxTransport.callTool('vox_attention_reset', {});
            } else if (type === 'trustOverride') {
                voxTransport.callTool('vox_trust_override', { agent_id: msg.agentId, tier: msg.tier, reason: msg.reason });
            } else if (type === 'setSocratesGate') {
                voxTransport.callTool('vox_preference_set', { key: 'socrates_gate_enforced', value: String(msg.enforce) });
            } else {
                console.warn('Unhandled legacy postMessage type mapped in shim:', type, msg);
            }
        },
        setState: (s: any) => console.log("VSCode SetState:", s),
        getState: () => null,
    };
}
