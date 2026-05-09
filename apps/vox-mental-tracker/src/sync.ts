export async function registerServiceWorker() {
    if ('serviceWorker' in navigator) {
        try {
            await navigator.serviceWorker.register('/sw.js');
        } catch (err) {
            console.error('SW register failed', err);
        }
    }
}
