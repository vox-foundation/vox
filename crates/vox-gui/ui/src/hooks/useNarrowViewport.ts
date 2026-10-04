import { useEffect, useState } from 'react';

/** True while the viewport is narrower than `maxPx`; false where `matchMedia` is unavailable (jsdom, SSR). */
export function useNarrowViewport(maxPx = 640): boolean {
  const query = `(max-width: ${maxPx - 1}px)`;
  const read = () => typeof window.matchMedia === 'function' && window.matchMedia(query).matches;
  const [narrow, setNarrow] = useState(read);
  useEffect(() => {
    if (typeof window.matchMedia !== 'function') return;
    const mq = window.matchMedia(query);
    const sync = () => setNarrow(mq.matches);
    sync();
    mq.addEventListener('change', sync);
    return () => mq.removeEventListener('change', sync);
  }, [query]);
  return narrow;
}
