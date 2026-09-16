import React from 'react';

export interface SafeExternalLinkProps {
  url: string;
  className?: string;
  children?: React.ReactNode;
}

/**
 * Normalizes DOIs (e.g. 10.1038/... or doi:10.1038/...) and validates external URLs
 * using the standard URL parser. Only http: and https: protocols are permitted.
 */
export function normalizeAndValidateUrl(rawUrl: string): string | null {
  if (!rawUrl || typeof rawUrl !== 'string') {
    return null;
  }
  let trimmed = rawUrl.trim();
  if (trimmed.toLowerCase().startsWith('doi:')) {
    trimmed = `https://doi.org/${trimmed.slice(4)}`;
  } else if (/^10\.\d{4,9}\/[-._;()/:A-Za-z0-9]+$/.test(trimmed)) {
    trimmed = `https://doi.org/${trimmed}`;
  }

  try {
    const parsed = new URL(trimmed);
    if (parsed.protocol === 'http:' || parsed.protocol === 'https:') {
      return parsed.href;
    }
  } catch {
    return null;
  }
  return null;
}

/**
 * Returns true if the URL is valid and safe for navigation.
 */
export function isSafeUrl(rawUrl: string): boolean {
  return normalizeAndValidateUrl(rawUrl) !== null;
}

export function SafeExternalLink({ url, className, children }: SafeExternalLinkProps) {
  const normalizedUrl = normalizeAndValidateUrl(url);

  if (!normalizedUrl) {
    return (
      <span
        className={`text-text-muted select-text ${className ?? ''}`}
        title={`Unsafe or invalid link: ${url}`}
        data-testid="unsafe-url"
      >
        {children ?? url}
      </span>
    );
  }

  return (
    <a
      href={normalizedUrl}
      target="_blank"
      rel="noopener noreferrer"
      className={className ?? 'text-brass hover:text-brass/80 underline decoration-dotted truncate'}
      data-testid="safe-url"
    >
      {children ?? url}
    </a>
  );
}
