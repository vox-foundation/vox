import React from 'react';

export function ModelPickerSearch({
  value,
  onChange,
}: {
  value: string;
  onChange: (query: string) => void;
}) {
  return (
    <input
      type="search"
      role="searchbox"
      aria-label="Search models"
      placeholder="Search models…"
      value={value}
      onChange={e => onChange(e.target.value)}
      onKeyDown={e => {
        e.stopPropagation();
        if (e.key === 'Enter') e.preventDefault();
      }}
      className="mb-1 w-full rounded-md border border-border-subtle bg-overlay-subtle px-2 py-1 font-mono text-[10px] text-text-primary placeholder:text-text-muted focus:border-brass/40 focus:outline-hidden"
    />
  );
}
