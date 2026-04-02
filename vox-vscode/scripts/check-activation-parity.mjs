/**
 * Fail if contributes.commands entries lack matching onCommand activation.
 * Non-.vox workspaces rely on lazy activation; workspaceContains alone is insufficient.
 */
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const pkgPath = path.join(__dirname, '..', 'package.json');
const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'));

const contributed = new Set(
    (pkg.contributes?.commands ?? []).map((c) => c.command).filter(Boolean),
);

const onCommand = new Set(
    (pkg.activationEvents ?? [])
        .filter((e) => typeof e === 'string' && e.startsWith('onCommand:'))
        .map((e) => e.slice('onCommand:'.length)),
);

const missing = [...contributed].filter((id) => !onCommand.has(id));
if (missing.length) {
    console.error('Activation parity: contributed commands without onCommand in activationEvents:');
    for (const id of missing) console.error(`  - ${id}`);
    process.exit(1);
}

console.log(
    `OK: ${contributed.size} contributed commands → onCommand (${onCommand.size} onCommand entries in manifest).`,
);
