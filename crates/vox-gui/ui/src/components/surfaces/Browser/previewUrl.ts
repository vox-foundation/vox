/** Host of a preview URL. Matches `preview_url_host` in `vox-gui` browser.rs. */
export function previewUrlHost(url: string): string {
  const remainder = url.includes('://') ? (url.split('://')[1] ?? url) : url;
  const authority = remainder.split(/[/?#]/, 1)[0] ?? '';
  if (authority.includes('\\')) {
    return '';
  }
  const afterAt = authority.includes('@')
    ? authority.slice(authority.lastIndexOf('@') + 1)
    : authority;
  if (afterAt.startsWith('[')) {
    const end = afterAt.indexOf(']');
    return (end >= 0 ? afterAt.slice(0, end + 1) : afterAt).toLowerCase();
  }
  return (afterAt.split(':')[0] ?? '').replace(/\.+$/, '').toLowerCase();
}

/** Preview iframe may only load localhost / 127.0.0.1 / [::1]. */
export function isLoopbackPreviewUrl(url: string): boolean {
  return ['localhost', '127.0.0.1', '[::1]'].includes(previewUrlHost(url));
}
