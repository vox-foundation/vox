import React, { useState } from 'react';
import { Button } from '../../ui/Button';

interface TaskComposerProps {
  onSubmit: (text: string) => void;
  busy?: boolean;
}

export function TaskComposer({ onSubmit, busy = false }: TaskComposerProps) {
  const [text, setText] = useState('');

  const handleSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    const trimmed = text.trim();
    if (!trimmed) return;
    onSubmit(trimmed);
    setText('');
  };

  return (
    <form onSubmit={handleSubmit} className="flex items-center gap-3 rounded-xl border border-white/10 bg-white/2 px-3 py-2">
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="Add a task…"
        aria-label="Add a task"
        rows={1}
        className="min-h-[28px] flex-1 resize-none bg-transparent py-1 text-[13px] text-zinc-100 outline-hidden placeholder:text-zinc-500"
        onKeyDown={(e) => {
          if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            handleSubmit();
          }
        }}
      />
      <Button
        type="button"
        variant="primary"
        size="sm"
        onClick={() => handleSubmit()}
        disabled={busy || !text.trim()}
      >
        Add
      </Button>
    </form>
  );
}
