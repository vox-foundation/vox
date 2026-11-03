/**
 * GUI side doors (Ingest URL, Ingest Text, Save to KB, Publish research doc) file pending
 * findings in the review inbox instead of making content searchable or writing docs/src
 * (phase 03-11, GROW-06 / D-05). Interim visibility is the toast copy, so each flow asserts
 * it and captures a screenshot into review-bundle/latest/ (default theme).
 */
import { test, expect, type Page } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { installTauriMock } from './lib/tauriMock';
import { addMockInitScript } from './lib/tauriMockShared';
import { addInvokeOverrides } from './lib/invokeOverrides';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT_DIR = join(HERE, '..', 'review-bundle', 'latest');

const PROPOSED = 'docs/src/architecture/session-1-research-2026.md';
const DRAFT = {
  slug: 'session-1-research',
  title: 'Research Session #1 Architecture SSOT (2026)',
  filename: 'session-1-research-2026.md',
  markdown_content: `---
title: "Research Session #1 Architecture SSOT (2026)"
description: "Empirically verified architecture findings and benchmarks."
category: "Architecture SSOTs"
status: "current"
---

# Research Session #1 Architecture SSOT (2026)

## 1. Executive Summary & Codebase Reality
Findings from session 1.
`,
  is_valid: true,
  validation_errors: [],
};

async function open(page: Page, view: 'memory' | 'research', responses: Record<string, unknown>) {
  await addMockInitScript(page, installTauriMock, view);
  await addInvokeOverrides(page, { responses });
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.goto('/');
  await page.waitForSelector('nav', { timeout: 15_000 });
}

async function shot(page: Page, name: string) {
  mkdirSync(OUT_DIR, { recursive: true });
  await page.screenshot({ path: join(OUT_DIR, `${name}.png`) });
}

const toasts = (page: Page) => page.getByTestId('toast-stack');

async function noPublishedWording(page: Page) {
  await expect(page.getByText(/Published/)).toHaveCount(0);
}

test.describe('KB inbox side doors', () => {
  test('Ingest URL files the page for review', async ({ page }) => {
    await open(page, 'memory', { ingest_url: { finding_id: 'f-url-1', status: 'pending' } });
    await page.getByTestId('tab-ingest').click();
    await page.getByPlaceholder('https://example.com/article').fill('https://example.com/post');
    await page.getByTestId('ingest-url-btn').click();
    await expect(toasts(page)).toContainText('Filed for review');
    await expect(toasts(page)).toContainText('not yet searchable');
    await noPublishedWording(page);
    await shot(page, 'kb-inbox-ingest-url');
  });

  test('Ingest URL reports a duplicate as already pending', async ({ page }) => {
    await open(page, 'memory', { ingest_url: { finding_id: null, status: 'duplicate' } });
    await page.getByTestId('tab-ingest').click();
    await page.getByPlaceholder('https://example.com/article').fill('https://example.com/post');
    await page.getByTestId('ingest-url-btn').click();
    await expect(toasts(page)).toContainText('Already pending review');
    await noPublishedWording(page);
    await shot(page, 'kb-inbox-ingest-url-duplicate');
  });

  test('Ingest Text files the pasted text for review', async ({ page }) => {
    await open(page, 'memory', { ingest_text: { finding_id: 'f-text-1', status: 'pending' } });
    await page.getByTestId('tab-ingest').click();
    await page.getByPlaceholder('Architecture invariants overview').fill('Reference notes');
    await page.getByTestId('ingest-text-btn').scrollIntoViewIfNeeded();
    await page.locator('textarea').first().fill('Some pasted reference text.');
    await page.getByTestId('ingest-text-btn').click();
    await expect(toasts(page)).toContainText('Filed for review');
    await expect(toasts(page)).toContainText('not yet searchable');
    await noPublishedWording(page);
    await shot(page, 'kb-inbox-ingest-text');
  });

  test('Save to KB and Publish file findings and never claim publication', async ({ page }) => {
    await open(page, 'research', {
      save_research_session_to_kb: 3,
      generate_research_doc_draft: DRAFT,
      publish_research_doc: { finding_id: 'f-pub-1', proposed_path: PROPOSED, status: 'pending' },
    });
    await page.getByText('vector db tradeoffs').first().click();

    await page.getByTestId('save-to-kb-button').click();
    await expect(toasts(page)).toContainText('3 source(s) filed for review');
    await shot(page, 'kb-inbox-save-to-kb');

    await page.getByRole('button', { name: 'Publish Architecture SSOT' }).click();
    await page.getByRole('button', { name: /Approve & Publish/i }).click();
    await expect(toasts(page)).toContainText('Filed for review');
    await expect(toasts(page)).toContainText(PROPOSED);
    await expect(toasts(page)).toContainText('pending review');
    await noPublishedWording(page);
    await shot(page, 'kb-inbox-publish-doc');
  });
});
