import { useEffect, useRef } from 'react';

export type SoundType = 'doubt_start' | 'validated' | 'overruled' | 'achievement' | 'level_up';

const SOUND_URLS: Record<SoundType, string> = {
    doubt_start: 'https://assets.mixkit.co/active_storage/sfx/212/212-preview.mp3', // Suspense/Suspect
    validated: 'https://assets.mixkit.co/active_storage/sfx/1435/1435-preview.mp3', // Ding/Success
    overruled: 'https://assets.mixkit.co/active_storage/sfx/2955/2955-preview.mp3', // Error/Alert
    achievement: 'https://assets.mixkit.co/active_storage/sfx/2013/2013-preview.mp3', // Fanfare
    level_up: 'https://assets.mixkit.co/active_storage/sfx/2018/2018-preview.mp3', // Major Ding
};

export function useSoundEffects(enabled: boolean = true) {
    const audioRefs = useRef<Partial<Record<SoundType, HTMLAudioElement>>>({});

    useEffect(() => {
        if (!enabled) return;

        // Preload sounds
        Object.entries(SOUND_URLS).forEach(([key, url]) => {
            const audio = new Audio(url);
            audio.preload = 'auto';
            audioRefs.current[key as SoundType] = audio;
        });

        return () => {
            Object.values(audioRefs.current).forEach(audio => {
                if (audio) {
                    audio.pause();
                    audio.src = '';
                }
            });
        };
    }, [enabled]);

    const playSound = (type: SoundType) => {
        if (!enabled) return;
        
        const audio = audioRefs.current[type];
        if (audio) {
            audio.currentTime = 0;
            audio.play().catch(err => console.warn('Audio playback failed:', err));
        }
    };

    return { playSound };
}
