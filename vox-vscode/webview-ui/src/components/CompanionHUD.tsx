import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Trophy, Star, Activity, Award, CheckCircle, Zap } from 'lucide-react';

export function CompanionHUD({ gamify }: any) {
    const [achievements, setAchievements] = useState<any[]>([]);

    // Read exclusively from GamifyState prop
    const level = gamify?.level || 1;
    const currentXp = gamify?.xp || 0;
    const prestige = Math.floor(level / 50); // Derived from level 50 prestige mechanic
    
    const xpForCurrentLevel = 25 * Math.pow(level, 2) + 25 * level - 50;
    const xpForNextLevel = 25 * Math.pow(level + 1, 2) + 25 * (level + 1) - 50;
    const progress = Math.max(0, Math.min(100, ((currentXp - xpForCurrentLevel) / (xpForNextLevel - xpForCurrentLevel)) * 100));

    // Listen for incoming dynamic achievements
    useEffect(() => {
        if (!gamify?.achievements) return;
        const now = Date.now();
        const fresh = gamify.achievements.filter((a: any) => a.unlocked_at && (now - a.unlocked_at * 1000 < 10000));
        setAchievements(fresh);
    }, [gamify?.achievements]);

    const prestigeColors = [
        'bg-zinc-500 border-zinc-400', 
        'bg-blue-500 border-blue-400', 
        'bg-violet-500 border-violet-400 glow-violet', 
        'bg-amber-500 border-amber-400 shadow-[0_0_15px_rgba(245,158,11,0.5)]'
    ];
    const currentPrestigeStyle = prestigeColors[Math.min(prestige, 3)];

    return (
        <div className="fixed bottom-6 right-6 z-50 flex flex-col items-end gap-4 pointer-events-none">
            
            {/* Achievement Popup */}
            <AnimatePresence>
                {achievements.map((ach) => (
                    <motion.div 
                        key={ach.id}
                        initial={{ opacity: 0, scale: 0.9, y: 20 }}
                        animate={{ opacity: 1, scale: 1, y: 0 }}
                        exit={{ opacity: 0, scale: 0.9, y: -20 }}
                        className="glass bg-white/[0.05] border border-amber-500/50 rounded-2xl p-4 flex items-center gap-4 shadow-[0_0_30px_rgba(245,158,11,0.15)] glow-amber backdrop-blur-xl mb-2 pointer-events-auto"
                    >
                        <div className="w-10 h-10 rounded-full bg-amber-500/20 text-amber-500 flex items-center justify-center border border-amber-500/50">
                            {ach.icon}
                        </div>
                        <div>
                            <div className="text-[9px] font-bold uppercase tracking-widest text-amber-500 mb-0.5">Achievement Unlocked</div>
                            <div className="text-sm font-black text-white">{ach.title}</div>
                            <div className="text-[10px] text-zinc-400">{ach.desc}</div>
                        </div>
                    </motion.div>
                ))}
            </AnimatePresence>

            {/* Main HUD */}
            <div className="glass bg-black/40 border border-white/10 rounded-[2rem] p-4 backdrop-blur-xl w-72 flex flex-col gap-4 pointer-events-auto hover:bg-black/60 transition-all">
                
                <div className="flex items-center gap-4">
                    <div className="relative">
                        <svg className="w-14 h-14 transform -rotate-90">
                            <circle cx="28" cy="28" r="24" fill="transparent" stroke="rgba(255,255,255,0.05)" strokeWidth="4" />
                            <motion.circle 
                                cx="28" 
                                cy="28" 
                                r="24" 
                                fill="transparent" 
                                stroke="currentColor" 
                                strokeWidth="4"
                                strokeDasharray={150}
                                strokeDashoffset={150 - (150 * progress) / 100}
                                className={prestige >= 2 ? 'text-violet-500' : 'text-blue-500'}
                                style={{ strokeLinecap: 'round' }}
                            />
                        </svg>
                        <div className="absolute inset-0 flex items-center justify-center">
                            <span className="text-xl font-black text-white">{level}</span>
                        </div>
                        {prestige > 0 && (
                            <div className={`absolute -bottom-1 -right-1 w-5 h-5 rounded-full flex items-center justify-center border-2 border-[#09090b] ${currentPrestigeStyle}`}>
                                <Star size={10} className="text-white" />
                            </div>
                        )}
                    </div>

                    <div className="flex-1">
                        <div className="flex justify-between items-end mb-1">
                            <span className="text-xs font-bold text-white uppercase tracking-widest">Vox Ludus</span>
                            <span className="text-[10px] text-zinc-400 font-mono">{currentXp} XP</span>
                        </div>
                        <div className="text-[9px] text-zinc-500 uppercase font-bold">{xpForNextLevel - currentXp} XP to Level {level + 1}</div>
                    </div>
                </div>

                <div className="grid grid-cols-2 gap-2 border-t border-white/5 pt-3">
                    <div className="flex flex-col gap-1">
                        <span className="text-[8px] text-zinc-500 font-bold uppercase tracking-widest flex items-center gap-1"><Activity size={10} /> Daily Reset</span>
                        <span className="text-[10px] text-zinc-300 font-mono">{gamify?.daily_reset_ms ? new Date(gamify.daily_reset_ms).toISOString().substr(11, 5) : '--'}</span>
                    </div>
                    <div className="flex flex-col gap-1">
                        <span className="text-[8px] text-zinc-500 font-bold uppercase tracking-widest flex items-center gap-1"><Award size={10} /> Reputation</span>
                        <span className="text-[10px] text-zinc-300 font-mono text-emerald-400">{gamify?.companion_mood || '--'}</span>
                    </div>
                </div>

            </div>
        </div>
    );
}
