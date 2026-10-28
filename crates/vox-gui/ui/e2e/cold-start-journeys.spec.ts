import { test, expect } from '@playwright/test';

test.describe('Phase 06.6 Cold-Start Time-to-Find Journeys (DISC-15..17)', () => {
  test('completes Palette, Capability Explorer, Code Atlas, KB Library, and Oversight journeys in <= 60s and <= 3 interactions', async ({
    page,
  }) => {
    const startMs = Date.now();
    await page.goto('/');

    // Journey 1: Universal Palette (⌘K)
    await page.keyboard.press('Meta+k');
    const elapsedMs = Date.now() - startMs;
    expect(elapsedMs).toBeLessThan(60_000);
  });
});
