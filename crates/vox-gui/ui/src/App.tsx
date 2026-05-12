import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { CommandCatalogForm } from './components/CommandCatalogForm';
import type { CommandCatalog } from './types/catalog';

export default function App() {
  const [catalog, setCatalog] = useState<CommandCatalog | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke('get_command_catalog')
      .then((res: any) => setCatalog(res))
      .catch((err) => setError(String(err)));
  }, []);

  return (
    <div className="flex flex-col h-full w-full">
      {error ? (
        <div className="p-4 text-red-500 font-mono">Error loading CLI Manifest: {error}</div>
      ) : catalog ? (
        <CommandCatalogForm catalog={catalog} />
      ) : (
        <div className="p-4 text-cyan font-mono">Loading CLI Manifest...</div>
      )}
    </div>
  );
}
