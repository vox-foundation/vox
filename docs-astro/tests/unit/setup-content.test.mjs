import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { MIRROR_EXCLUDED, MIRROR_MARKER, buildMirror, prepareMirrorDir } from '../../scripts/setup-content.mjs';

function withTemp(fn) {
  const root = mkdtempSync(join(tmpdir(), 'setup-content-'));
  try {
    fn(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function assertFreshMirror(mirror) {
  assert.ok(lstatSync(mirror).isDirectory(), 'mirror is a real directory');
  assert.ok(!lstatSync(mirror).isSymbolicLink(), 'mirror is not a symlink');
  assert.deepEqual(readdirSync(mirror), [MIRROR_MARKER]);
}

test('(a) an unmarked real directory is refused and its contents survive', () => {
  withTemp((root) => {
    const mirror = join(root, 'docs');
    mkdirSync(mirror);
    writeFileSync(join(mirror, 'keep.md'), 'real content');
    assert.throws(() => prepareMirrorDir(mirror), /without the \.vox-docs-mirror marker/);
    assert.equal(readFileSync(join(mirror, 'keep.md'), 'utf8'), 'real content');
  });
});

test('(b) a symlink is unlinked without touching its target', () => {
  withTemp((root) => {
    const target = join(root, 'target');
    mkdirSync(target);
    writeFileSync(join(target, 'page.md'), 'target content');
    const mirror = join(root, 'docs');
    symlinkSync(target, mirror, process.platform === 'win32' ? 'junction' : 'dir');
    prepareMirrorDir(mirror);
    assert.equal(readFileSync(join(target, 'page.md'), 'utf8'), 'target content');
    assertFreshMirror(mirror);
  });
});

test('(c) a marked mirror is replaced, and links inside it are not followed', () => {
  withTemp((root) => {
    const target = join(root, 'target');
    mkdirSync(target);
    writeFileSync(join(target, 'page.md'), 'target content');
    const mirror = join(root, 'docs');
    mkdirSync(mirror);
    writeFileSync(join(mirror, MIRROR_MARKER), '');
    writeFileSync(join(mirror, 'stale.md'), 'stale');
    symlinkSync(target, join(mirror, 'section'), process.platform === 'win32' ? 'junction' : 'dir');
    prepareMirrorDir(mirror);
    assertFreshMirror(mirror);
    assert.equal(readFileSync(join(target, 'page.md'), 'utf8'), 'target content');
  });
});

test('(d) a missing path is created with the marker', () => {
  withTemp((root) => {
    const mirror = join(root, 'content', 'docs');
    prepareMirrorDir(mirror);
    assertFreshMirror(mirror);
  });
});

test('buildMirror links every top-level entry except the excluded ones', () => {
  withTemp((root) => {
    const source = join(root, 'src');
    for (const dir of ['reference', 'archive', '.well-known']) mkdirSync(join(source, dir), { recursive: true });
    writeFileSync(join(source, 'index.mdx'), 'home');
    writeFileSync(join(source, 'SUMMARY.md'), 'generated');
    const mirror = join(root, 'docs');
    const { mirrored } = buildMirror(source, mirror);
    assert.equal(mirrored, 2);
    assert.deepEqual(readdirSync(mirror).sort(), [MIRROR_MARKER, 'index.mdx', 'reference'].sort());
    for (const name of MIRROR_EXCLUDED) assert.ok(!existsSync(join(mirror, name)), name);
    buildMirror(source, mirror);
    assert.ok(existsSync(join(source, 'archive')), 'rebuild leaves the source untouched');
  });
});
