// RED stub (D-17): replaced in the GREEN commit.
import { DEFAULT_BINDINGS, type Bindings } from './keybinds';
let snapshot: Bindings = DEFAULT_BINDINGS;
export function getBindings(): Bindings { return snapshot; }
export function subscribeBindings(_fn: () => void): () => void { return () => {}; }
export function setBindings(next: Bindings): void { snapshot = next; }
export function resetKeybindStoreForTests(): void { snapshot = DEFAULT_BINDINGS; }
