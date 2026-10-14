import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, existsSync, readFileSync, writeFileSync, mkdirSync, symlinkSync, lstatSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { prepareMirrorDir, MIRROR_MARKER } from '../../scripts/setup-content.mjs';

test('prepareMirrorDir: (a) unmarked real directory holding a file throws and preserves content', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'vox-mirror-test-a-'));
  try {
    const mirrorPath = join(tmp, 'content-docs');
    mkdirSync(mirrorPath);
    const secretFile = join(mirrorPath, 'preserve.txt');
    writeFileSync(secretFile, 'important content');

    assert.throws(
      () => prepareMirrorDir(mirrorPath),
      /missing \.vox-docs-mirror/
    );

    assert.equal(existsSync(secretFile), true);
    assert.equal(readFileSync(secretFile, 'utf8'), 'important content');
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test('prepareMirrorDir: (b) symlink at mirror path pointing to temp dir with file is unlinked, target survives, fresh marked dir created', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'vox-mirror-test-b-'));
  try {
    const targetDir = join(tmp, 'real-target');
    mkdirSync(targetDir);
    const targetFile = join(targetDir, 'survive.txt');
    writeFileSync(targetFile, 'target content');

    const mirrorPath = join(tmp, 'mirror-link');
    symlinkSync(targetDir, mirrorPath, 'dir');

    prepareMirrorDir(mirrorPath);

    // Target directory and file should still be untouched
    assert.equal(existsSync(targetFile), true);
    assert.equal(readFileSync(targetFile, 'utf8'), 'target content');

    // Mirror path should now be a real marked directory, not a symlink
    const stat = lstatSync(mirrorPath);
    assert.equal(stat.isDirectory(), true);
    assert.equal(stat.isSymbolicLink(), false);
    assert.equal(existsSync(join(mirrorPath, MIRROR_MARKER)), true);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test('prepareMirrorDir: (c) marked mirror dir replaced by fresh dir containing only marker', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'vox-mirror-test-c-'));
  try {
    const mirrorPath = join(tmp, 'content-docs');
    mkdirSync(mirrorPath);
    writeFileSync(join(mirrorPath, MIRROR_MARKER), '');
    writeFileSync(join(mirrorPath, 'stale.txt'), 'old state');

    prepareMirrorDir(mirrorPath);

    assert.equal(existsSync(join(mirrorPath, MIRROR_MARKER)), true);
    assert.equal(existsSync(join(mirrorPath, 'stale.txt')), false);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test('prepareMirrorDir: (d) non-existent path created with marker', () => {
  const tmp = mkdtempSync(join(tmpdir(), 'vox-mirror-test-d-'));
  try {
    const mirrorPath = join(tmp, 'content-docs');
    assert.equal(existsSync(mirrorPath), false);

    prepareMirrorDir(mirrorPath);

    assert.equal(existsSync(mirrorPath), true);
    assert.equal(existsSync(join(mirrorPath, MIRROR_MARKER)), true);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});
