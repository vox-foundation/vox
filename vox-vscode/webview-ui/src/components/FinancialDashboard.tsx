import React, { useState } from 'react';
import { LineChart, Line, XAxis, YAxis, Tooltip, ResponsiveContainer, CartesianGrid } from 'recharts';
import { AlertTriangle, Server, Database, Save, Activity } from 'lucide-react';
import { getVsCodeApi } from '../utils/vscode';

const vscode = getVsCodeApi();

export function FinancialDashboard({ stats, budgetHistory, modelList }: any) {
    const [budgetCap, setBudgetCap] = useState(stats?.budget_cap_usd || 50.0);
    const [customModel, setCustomModel] = useState(stats?.active_model || "gemini-2.0-flash-lite");
    const rpm = stats?.estimated_rpm || 0;
    const isRunaway = rpm > 500;
    
    // Chart data is now provided by props

    const handleSaveBudget = () => {
        vscode.postMessage({ type: 'updateBudgetCap', value: budgetCap });
    };

    return (
        <div className="p-10 bg-[#09090b] h-full overflow-y-auto text-white">
            <div className="flex justify-between items-center mb-8">
                <div>
                <h2 className="text-3xl font-black tracking-tighter uppercase">
                    Financial <span className="text-emerald-500">Command</span>
                </h2>
                <p className="text-[10px] text-zinc-500 mt-1 max-w-xl">
                    Operator view: budget and model signals from MCP. Disclosure SSOT: docs/src/architecture/telemetry-client-disclosure-ssot.md
                </p>
                </div>
                {isRunaway && (
                    <div className="flex items-center gap-2 bg-red-500/20 text-red-500 px-4 py-2 rounded-full border border-red-500/50 animate-pulse">
                        <AlertTriangle size={16} />
                        <span className="font-bold text-xs uppercase tracking-widest">Runaway Execution Detected</span>
                    </div>
                )}
            </div>

            <div className="grid grid-cols-3 gap-8 mb-8">
                <div className="col-span-1 glass p-6 rounded-3xl border border-white/5 bg-white/[0.02]">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-4">Current Burn</h3>
                    <div className="text-6xl font-black mb-1">${stats?.total_cost_usd?.toFixed(2) || "0.00"}</div>
                    <div className="text-[10px] text-emerald-500 uppercase tracking-widest font-bold">Total Incurred USD</div>
                </div>

                <div className="col-span-1 glass p-6 rounded-3xl border border-white/5 bg-white/[0.02]">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-4">Token Velocity</h3>
                    <div className="text-6xl font-black mb-1 flex items-baseline gap-2">
                        {rpm} <span className="text-lg text-zinc-500">RPM</span>
                    </div>
                    <div className="h-2 w-full bg-white/10 rounded-full mt-4 overflow-hidden flex">
                        <div 
                            className={`h-full ${isRunaway ? 'bg-red-500' : 'bg-emerald-500'} transition-all`} 
                            style={{ width: `${Math.min((rpm / 1000) * 100, 100)}%` }} 
                        />
                    </div>
                </div>

                <div className="col-span-1 glass p-6 rounded-3xl border border-white/5 bg-white/[0.02]">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-4">Budget Cap Control</h3>
                    <div className="flex items-center gap-4 mb-4">
                        <span className="text-xl text-zinc-400">$</span>
                        <input 
                            type="number" 
                            value={budgetCap} 
                            onChange={(e) => setBudgetCap(parseFloat(e.target.value))}
                            className="bg-transparent text-4xl font-black w-full outline-none"
                        />
                    </div>
                    <button 
                        onClick={handleSaveBudget}
                        className="w-full bg-emerald-500/10 hover:bg-emerald-500/20 text-emerald-500 border border-emerald-500/30 rounded-xl py-2 flex items-center justify-center gap-2 font-bold text-[10px] uppercase tracking-widest transition-all"
                    >
                        <Save size={14} /> Enforce Hard Cap
                    </button>
                </div>
            </div>

            <div className="grid grid-cols-3 gap-8">
                <div className="col-span-2 glass p-6 rounded-3xl border border-white/5 bg-white/[0.02] h-72">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-6 flex items-center gap-2">
                        <Activity size={14} /> Cost Trajectory (Last Hour)
                    </h3>
                    <ResponsiveContainer width="100%" height="80%">
                        <LineChart data={budgetHistory && budgetHistory.length > 0 ? budgetHistory : [{ time: '--', cost: 0 }]}>
                            <CartesianGrid strokeDasharray="3 3" stroke="rgba(255,255,255,0.05)" />
                            <XAxis dataKey="time" stroke="rgba(255,255,255,0.2)" fontSize={10} tickLine={false} axisLine={false} />
                            <YAxis stroke="rgba(255,255,255,0.2)" fontSize={10} tickLine={false} axisLine={false} tickFormatter={(val) => `$${val}`} />
                            <Tooltip content={<CustomTooltip />} cursor={{ stroke: 'rgba(255,255,255,0.1)', strokeWidth: 1, strokeDasharray: '4 4' }} />
                            <Line type="monotone" dataKey="cost" stroke="#10b981" strokeWidth={3} dot={{ r: 4, fill: '#09090b', stroke: '#10b981', strokeWidth: 2 }} />
                        </LineChart>
                    </ResponsiveContainer>
                </div>

                <div className="col-span-1 glass p-6 rounded-3xl border border-white/5 bg-white/[0.02] flex flex-col gap-4">
                    <h3 className="text-xs font-bold text-zinc-500 uppercase tracking-widest mb-2 flex items-center gap-2">
                        <Database size={14} /> Model Routing
                    </h3>
                    
                    <div>
                        <label className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest mb-2 block">Active Provider</label>
                        <select 
                            value={customModel} 
                            onChange={(e) => {
                                setCustomModel(e.target.value);
                                vscode.postMessage({ type: 'setModel', value: e.target.value });
                            }}
                            className="w-full bg-black/50 border border-white/10 rounded-xl px-4 py-3 text-sm focus:outline-none focus:border-blue-500 transition-colors"
                        >
                            {modelList && modelList.length > 0 ? modelList.map((m: any) => (
                                <option key={m.id} value={m.id}>{m.displayName}</option>
                            )) : (
                                <option value="gemini-2.0-flash-lite">Gemini 2.0 Flash Lite</option>
                            )}
                        </select>
                    </div>

                    <div className="mt-2">
                        <label className="text-[10px] text-zinc-500 font-bold uppercase tracking-widest mb-2 block">Provider Policy Override (BYOK)</label>
                        <input 
                            type="password" 
                            placeholder="sk-..." 
                            className="w-full bg-black/50 border border-white/10 rounded-xl px-4 py-3 text-sm focus:outline-none focus:border-emerald-500 transition-colors font-mono"
                            onChange={(e) => {
                                const activeModelData = (modelList || []).find((m: any) => m.id === customModel);
                                const provider = activeModelData ? activeModelData.provider : 'unknown';
                                vscode.postMessage({ type: 'updateApiKey', provider, value: e.target.value });
                            }}
                        />
                        <p className="text-[10px] text-zinc-600 mt-2">Overrides Vox.toml global settings for current session.</p>
                    </div>

                    <div className="mt-auto flex items-center justify-between p-3 bg-blue-500/10 border border-blue-500/20 rounded-xl">
                        <div className="flex gap-2 items-center">
                            <Server size={14} className="text-blue-500" />
                            <span className="text-xs font-bold text-blue-500">VRAM Allocation</span>
                        </div>
                        <span className="text-xs font-mono text-zinc-300">
                            {stats?.vram_used_gb != null ? `${stats.vram_used_gb} / ${stats.vram_total_gb} GB` : '-- GB'}
                        </span>
                    </div>
                </div>
            </div>
        </div>
    );
}

const CustomTooltip = ({ active, payload, label }: any) => {
    if (active && payload && payload.length) {
        return (
            <div className="glass bg-black/80 backdrop-blur-xl border border-white/10 p-4 rounded-2xl shadow-[0_8px_30px_rgba(0,0,0,0.5)] flex flex-col gap-1">
                <span className="text-[9px] uppercase font-bold text-zinc-500 tracking-widest">{label}</span>
                <span className="text-base font-black text-emerald-400 font-mono">
                    ${Number(payload[0].value).toFixed(2)}
                </span>
                <div className="mt-1 pt-2 border-t border-white/5 flex gap-2 items-center text-[9px] font-bold uppercase tracking-widest text-emerald-500/50">
                    <Activity size={10} /> Live Snapshot Interval
                </div>
            </div>
        );
    }
    return null;
};
