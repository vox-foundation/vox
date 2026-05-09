import { precacheAndRoute } from 'workbox-precaching';
import { Queue } from 'workbox-background-sync';
import { registerRoute } from 'workbox-routing';
import { NetworkFirst, CacheFirst } from 'workbox-strategies';

precacheAndRoute(self.__WB_MANIFEST);

const mutationQueue = new Queue('vox-mutations', {
    onSync: async ({ queue }) => {
        let entry;
        while ((entry = await queue.shiftRequest())) {
            try {
                await fetch(entry.request.clone());
            } catch (err) {
                await queue.unshiftRequest(entry);
                throw err;
            }
        }
    },
    maxRetentionTime: 7 * 24 * 60,  // 7 days
});

registerRoute(
    /\/api\/.*/,
    async ({ event, request }) => {
        if (request.method === 'GET') {
            return new NetworkFirst({ cacheName: 'api-get' }).handle({ event, request });
        }
        try {
            const res = await fetch(request.clone());
            return res;
        } catch (err) {
            await mutationQueue.pushRequest({ request });
            return new Response(JSON.stringify({ queued: true }), {
                status: 202,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    },
);

registerRoute(/\/(static|assets)\/.*/, new CacheFirst({ cacheName: 'static-v1' }));
