import React from 'react';

interface PopoverProps {
  open: boolean;
  children: React.ReactNode;
  align?: 'left' | 'right';
  className?: string;
}

export function Popover({ open, children, align = 'left', className = '' }: PopoverProps) {
  if (!open) return null;
  return (
    <div
      className={`absolute ${align === 'right' ? 'right-0' : 'left-0'} bottom-9 z-[99999] min-w-[240px] rounded-lg border border-border-strong bg-bg-surface text-text-primary p-1 backdrop-blur-xl shadow-[0_24px_60px_-20px_rgba(28,20,12,0.35)] ${className}`}
    >
      {children}
    </div>
  );
}
