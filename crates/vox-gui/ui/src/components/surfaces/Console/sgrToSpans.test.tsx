import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import React from 'react';
import { voxTransport } from '../../../transport';
import {
  normalizeTerminalText,
  stripAnsi,
  parseSgrSpans,
  sgrToSpans,
  sanitizeWorkspaceFileLocator,
  extractOsc52Clipboard,
  encodeKittyKeyEvent,
} from './sgrToSpans';

describe('sgrToSpans', () => {
  beforeEach(() => {
    cleanup();
    vi.restoreAllMocks();
    document.documentElement.dataset.theme = 'arcane';
  });

  it('parses 16-color, 256-color, TrueColor SGR codes, text attributes, resets, and CR/CSI sequences', () => {
    expect(normalizeTerminalText('abcdef\rxy')).toBe('xycdef');
    expect(normalizeTerminalText('abcdef\rxy\x1b[0K')).toBe('xy');
    expect(normalizeTerminalText('abcdef\rxy\x1b[1K')).toBe('   def');
    expect(normalizeTerminalText('\x1b[31m\x1b[0m\x1b[32m\x1b[mClean')).toBe('Clean');

    const crNormalized = normalizeTerminalText(
      '\x1b]633;A\x07Downloading 10%\rDownloading 65%\r\x1b[KFinished build\r\nLine 2\r\x1b[K\nKeep\x1b[2KErased',
    );
    expect(crNormalized).toBe('Finished build\n\nErased');
    expect(stripAnsi('\x1b[1;31merror:\x1b[0m failed')).toBe('error: failed');

    const sample = [
      '\x1b[1;3;4;9;31;42mAlert\x1b[22;23;24;29;39;49m',
      '\x1b[2;94;103mDimBright\x1b[0m',
      '\x1b[38;5;196;48:5:240mIndexed\x1b[0m',
      '\x1b[38;2;12;34;56;48;2;210;220;230mTrueColor\x1b[0m',
      '\x1b[31;44;7mInverted\x1b[27mNormalOrder\x1b[0m',
    ].join(' ');

    const spans = parseSgrSpans(sample, { theme: 'arcane' });
    const alertSpan = spans.find((s) => s.text === 'Alert');
    expect(alertSpan).toBeDefined();
    expect(alertSpan?.bold).toBe(true);
    expect(alertSpan?.italic).toBe(true);
    expect(alertSpan?.underline).toBe(true);
    expect(alertSpan?.strikethrough).toBe(true);
    expect(alertSpan?.fg).toBe('#e57368');
    expect(alertSpan?.bg).toBe('#7cc47f');

    const dimSpan = spans.find((s) => s.text === 'DimBright');
    expect(dimSpan?.dim).toBe(true);
    expect(dimSpan?.fg).toBe('#9ec2f7');
    expect(dimSpan?.bg).toBe('#f0cb69');

    const indexedSpan = spans.find((s) => s.text === 'Indexed');
    expect(indexedSpan?.fg).toBe('rgb(255, 0, 0)');
    expect(indexedSpan?.bg).toBe('rgb(88, 88, 88)');

    const trueColorSpan = spans.find((s) => s.text === 'TrueColor');
    expect(trueColorSpan?.fg).toBe('rgb(12, 34, 56)');
    expect(trueColorSpan?.bg).toBe('rgb(210, 220, 230)');

    const invertedSpan = spans.find((s) => s.text === 'Inverted');
    expect(invertedSpan?.fg).toBe('#7aa8e8');
    expect(invertedSpan?.bg).toBe('#e57368');

    const uninvertedSpan = spans.find((s) => s.text === 'NormalOrder');
    expect(uninvertedSpan?.fg).toBe('#e57368');
    expect(uninvertedSpan?.bg).toBe('#7aa8e8');

    const darkSpans = parseSgrSpans('\x1b[31mDarkRed\x1b[0m \x1b[36mCyan\x1b[0m', { theme: 'slate' });
    expect(darkSpans[0]?.fg).toBe('#f87171');
    expect(darkSpans[2]?.fg).toBe('#38bdf8');

    const invArcane = parseSgrSpans('\x1b[7mWARN\x1b[0m', { theme: 'arcane' });
    expect(invArcane[0]).toMatchObject({ text: 'WARN', fg: '#15110d', bg: '#f5efe2', inverse: true });
    const invTrav = parseSgrSpans('\x1b[7mWARN\x1b[0m', { theme: 'travertine' });
    expect(invTrav[0]).toMatchObject({ text: 'WARN', fg: '#171512', bg: '#f6f1e7', inverse: true });
    const invSlate = parseSgrSpans('\x1b[7mWARN\x1b[0m', { theme: 'slate' });
    expect(invSlate[0]).toMatchObject({ text: 'WARN', fg: '#0c0e10', bg: '#f1f5f9', inverse: true });
    const invHc = parseSgrSpans('\x1b[7mWARN\x1b[0m', { theme: 'high-contrast' });
    expect(invHc[0]).toMatchObject({ text: 'WARN', fg: '#000000', bg: '#ffffff', inverse: true });

    const { container } = render(<pre>{sgrToSpans(sample)}</pre>);
    expect(container.textContent).toContain('Alert');
    expect(container.textContent).toContain('TrueColor');
  });

  it('renders clickable http(s), localhost, and file:line:col links plus case-insensitive search highlights', () => {
    const openSpy = vi
      .spyOn(voxTransport, 'openLocator')
      .mockResolvedValue({ opened: true } as never);

    const text =
      'Error in crates/vox-cli/src/main.rs:42:9 — docs at https://voxlang.org/docs. Dev server: localhost:4321/status';

    render(<pre>{sgrToSpans(text, { searchQuery: 'ERROR' })}</pre>);

    const marks = screen.getAllByTestId('term-search-highlight');
    expect(marks).toHaveLength(1);
    expect(marks[0]?.textContent).toBe('Error');

    const fileEvents: unknown[] = [];
    const onFileEvent = (e: Event) => fileEvents.push((e as CustomEvent).detail);
    window.addEventListener('vox://open-file-location', onFileEvent);

    const fileLink = screen.getByTestId('term-output-file-link');
    expect(fileLink.textContent).toBe('crates/vox-cli/src/main.rs:42:9');
    fireEvent.click(fileLink);
    expect(openSpy).toHaveBeenCalledWith({
      kind: 'file',
      value: 'crates/vox-cli/src/main.rs',
    });
    expect(fileEvents).toEqual([
      {
        path: 'crates/vox-cli/src/main.rs',
        line: 42,
        col: 9,
        value: 'crates/vox-cli/src/main.rs:42:9',
      },
    ]);
    window.removeEventListener('vox://open-file-location', onFileEvent);

    const urlLinks = screen.getAllByTestId('term-output-url-link');
    expect(urlLinks).toHaveLength(2);
    fireEvent.click(urlLinks[0]!);
    expect(openSpy).toHaveBeenCalledWith({
      kind: 'web',
      value: 'https://voxlang.org/docs',
    });

    fireEvent.click(urlLinks[1]!);
    expect(openSpy).toHaveBeenCalledWith({
      kind: 'web',
      value: 'http://localhost:4321/status',
    });
  });

  it('neutralizes XSS payloads as inert text and rejects unsafe URI schemes', () => {
    const openSpy = vi
      .spyOn(voxTransport, 'openLocator')
      .mockResolvedValue({ opened: true } as never);

    const malicious = [
      '\x1b[31m<img src=x onerror=alert(1)><script>evil()</script>\x1b[0m',
      'javascript:alert(1)',
      'data:text/html,boom',
      'data:text/plain;charset=utf-8,dir/payload.sh:1',
      'vbscript:msgbox(1)',
      'file:///etc/ssh/sshd_config.conf:1',
      '-n/foo.rs:1',
      '../secret.rs:1',
    ].join(' ');

    const { container } = render(<pre>{sgrToSpans(malicious)}</pre>);

    expect(container.querySelectorAll('script, img, iframe, a')).toHaveLength(0);
    expect(screen.queryAllByTestId('term-output-url-link')).toHaveLength(0);
    expect(screen.queryAllByTestId('term-output-file-link')).toHaveLength(0);
    expect(container.textContent).toContain('<img src=x onerror=alert(1)><script>evil()</script>');
    expect(openSpy).not.toHaveBeenCalled();

    expect(sanitizeWorkspaceFileLocator('file:///etc/ssh/sshd_config.conf:1')).toBeNull();
    expect(sanitizeWorkspaceFileLocator('-n/foo.rs:1')).toBeNull();
    expect(sanitizeWorkspaceFileLocator('../secret.rs:1')).toBeNull();
    expect(sanitizeWorkspaceFileLocator('C:\\Windows\\system32\\foo.rs:1')).toBeNull();
    expect(sanitizeWorkspaceFileLocator('crates/vox-cli/src/main.rs:42:9')).toEqual({
      path: 'crates/vox-cli/src/main.rs',
      line: 42,
      col: 9,
    });
  });

  it('renders OSC 8 hyperlinks, decodes OSC 52 clipboard payloads, and encodes Kitty CSI u keys', () => {
    const openSpy = vi
      .spyOn(voxTransport, 'openLocator')
      .mockResolvedValue({ opened: true } as never);
    const osc8 = 'See \x1b]8;;https://voxlang.org/spec\x07Vox Spec\x1b]8;;\x07 and \x1b]8;;file://crates/vox-cli/src/main.rs\x07CLI Main\x1b]8;;\x07';
    render(<pre>{sgrToSpans(osc8)}</pre>);
    const oscLinks = screen.getAllByTestId('term-output-osc8-link');
    expect(oscLinks).toHaveLength(2);
    expect(oscLinks[0]?.textContent).toBe('Vox Spec');
    fireEvent.click(oscLinks[0]!);
    expect(openSpy).toHaveBeenCalledWith({ kind: 'web', value: 'https://voxlang.org/spec' });
    fireEvent.click(oscLinks[1]!);
    expect(openSpy).toHaveBeenCalledWith({ kind: 'file', value: 'crates/vox-cli/src/main.rs' });

    const osc52 = extractOsc52Clipboard('ok\x1b]52;c;aGVsbG8tdm94\x07');
    expect(osc52.clipboardText).toBe('hello-vox');
    expect(osc52.cleaned).toBe('ok');

    expect(encodeKittyKeyEvent({ key: 'Enter', shiftKey: true })).toBe('\x1b[13;2u');
    expect(encodeKittyKeyEvent({ key: 'Enter', ctrlKey: true })).toBe('\x1b[13;5u');
    expect(encodeKittyKeyEvent({ key: 'Enter' })).toBeNull();
  });
});
