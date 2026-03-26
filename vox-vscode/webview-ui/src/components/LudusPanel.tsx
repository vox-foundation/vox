import React from 'react';
import { Bell, RefreshCw, Trophy } from 'lucide-react';
import { Panel } from './ui/Panel';
import { getVsCodeApi } from '../utils/vscode';

const vscode = getVsCodeApi();

const muted = 'var(--vscode-descriptionForeground, rgba(161,161,170,1))';
const accent = 'var(--vscode-textLink-foreground, #60a5fa)';

export function LudusPanel({ snapshot }: { snapshot: Record<string, unknown> | null }) {
    const kpi = (snapshot?.kpi ?? snapshot?.Kpi) as Record<string, unknown> | undefined;
    const notifications = (snapshot?.notifications as unknown[]) ?? [];
    const err = snapshot?.error as string | undefined;

    return (
        <div
            className="p-8 h-full overflow-y-auto space-y-6"
            style={{
                background: 'var(--vscode-sideBar-background, #09090b)',
                color: 'var(--vscode-sideBar-foreground, #fafafa)',
            }}
        >
            <div className="flex items-center gap-3">
                <div
                    className="w-11 h-11 rounded-2xl flex items-center justify-center"
                    style={{ background: 'rgba(245,158,11,0.12)', color: '#f59e0b' }}
                >
                    <Trophy size={22} />
                </div>
                <div>
                    <h2 className="text-xl font-extrabold tracking-tight">Ludus</h2>
                    <p className="text-xs font-medium" style={{ color: muted }}>
                        Progress snapshot via <code style={{ color: accent }}>vox_ludus_progress_snapshot</code>
                    </p>
                </div>
                <button
                    type="button"
                    className="ml-auto px-3 py-1.5 rounded-lg text-[11px] font-bold uppercase tracking-wider flex items-center gap-2"
                    style={{
                        border: '1px solid var(--vscode-button-border, rgba(255,255,255,0.12))',
                        background: 'var(--vscode-button-secondaryBackground, transparent)',
                    }}
                    onClick={() => vscode.postMessage({ type: 'ludusRefreshSnapshot' })}
                >
                    <RefreshCw size={14} /> Refresh
                </button>
            </div>

            {err ? <Panel className="!p-4 text-sm text-red-400">{err}</Panel> : null}

            <Panel className="!p-6">
                <h3 className="text-[11px] font-bold uppercase tracking-widest mb-4" style={{ color: muted }}>
                    KPI
                </h3>
                <div className="grid grid-cols-2 gap-4 text-sm font-mono">
                    <div>
                        <div style={{ color: muted }} className="text-[10px] uppercase font-bold mb-1">
                            Events
                        </div>
                        {String(kpi?.events_recorded ?? '—')}
                    </div>
                    <div>
                        <div style={{ color: muted }} className="text-[10px] uppercase font-bold mb-1">
                            XP (policy)
                        </div>
                        {String(kpi?.total_xp_from_policy ?? kpi?.total_xp ?? '—')}
                    </div>
                    <div>
                        <div style={{ color: muted }} className="text-[10px] uppercase font-bold mb-1">
                            Crystals
                        </div>
                        {String(kpi?.total_crystals_from_policy ?? kpi?.total_crystals ?? '—')}
                    </div>
                    <div>
                        <div style={{ color: muted }} className="text-[10px] uppercase font-bold mb-1">
                            Streak days
                        </div>
                        {String(kpi?.streak_days ?? '—')}
                    </div>
                </div>
            </Panel>

            <Panel className="!p-6">
                <div className="flex items-center justify-between mb-4">
                    <div className="flex items-center gap-2">
                        <Bell size={16} style={{ color: accent }} />
                        <h3 className="text-[11px] font-bold uppercase tracking-widest" style={{ color: muted }}>
                            Unread notifications
                        </h3>
                    </div>
                    {notifications.length > 0 ? (
                        <button
                            type="button"
                            className="text-[10px] font-bold uppercase tracking-wider px-2 py-1 rounded-md"
                            style={{
                                border: '1px solid var(--vscode-button-border)',
                                background: 'var(--vscode-button-secondaryBackground)',
                            }}
                            onClick={() => vscode.postMessage({ type: 'ludusAckAllNotifications' })}
                        >
                            Ack all
                        </button>
                    ) : null}
                </div>
                {notifications.length === 0 ? (
                    <div className="text-xs py-4 text-center font-medium" style={{ color: muted }}>
                        No unread notifications.
                    </div>
                ) : (
                    <ul className="space-y-2">
                        {notifications.map((n: any, i: number) => (
                            <li
                                key={String(n.id ?? n.notification_id ?? i)}
                                className="flex gap-3 items-start text-xs border rounded-lg p-3"
                                style={{ borderColor: 'var(--vscode-panel-border, rgba(255,255,255,0.08))' }}
                            >
                                <div className="flex-1 min-w-0">
                                    <div className="font-bold truncate">{String(n.title ?? n.heading ?? 'Notice')}</div>
                                    <div className="opacity-80 line-clamp-3 mt-1">{String(n.body ?? n.message ?? '')}</div>
                                </div>
                                <button
                                    type="button"
                                    className="shrink-0 px-2 py-1 rounded text-[10px] font-bold uppercase"
                                    style={{
                                        background: 'var(--vscode-button-background, #0ea5e9)',
                                        color: 'var(--vscode-button-foreground, #fff)',
                                    }}
                                    onClick={() =>
                                        vscode.postMessage({
                                            type: 'ludusAckNotification',
                                            notificationId: String(n.id ?? n.notification_id ?? ''),
                                        })
                                    }
                                >
                                    Ack
                                </button>
                            </li>
                        ))}
                    </ul>
                )}
            </Panel>
        </div>
    );
}
