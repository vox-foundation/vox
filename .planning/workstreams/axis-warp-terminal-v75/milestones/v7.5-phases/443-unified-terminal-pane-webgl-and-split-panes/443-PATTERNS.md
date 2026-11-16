# Phase 443: Unified Terminal Pane, WebGL & Split Panes - Pattern Map

**Mapped:** 2026-11-15
**Workstream:** `axis-warp-terminal-v75` (`v7.5`)
**Files analyzed:** 24 target files (10 new, 14 modified across production + Vitest suites)
**Analogs found:** 22 / 24 (2 novel zero-dependency TypeScript helpers with partial in-repo analogs)

## File Classification

Every file below stays strictly inside `crates/vox-gui/ui/src/` (no new npm dependencies in `package.json`, no Rust crate edits unless fixing an optional TS DTO alignment, and `< 500` non-blank LoC per module for `vox-code-audit`'s `god_object` detector).

| New / Modified File | Action | Current LoC -> Target Budget | Role | Data Flow | Closest In-Repo Analog | Match Quality |
|---|---|---|---|---|---|---|
| `crates/vox-gui/ui/src/transport.ts` | Modify | ~1,850 (touch `L870-1105` only; net +35 LoC) | ipc-client | request-response + event-listen | `crates/vox-gui/ui/src/transport.ts:870-1105` + `crates/vox-gui/src/commands/pty.rs:275-281` | exact |
| `crates/vox-gui/ui/src/transport.console.test.ts` | Modify | 85 -> ~145 LoC | unit-test | invoke-mock | `crates/vox-gui/ui/src/transport.console.test.ts:1-85` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` | **Create** | 0 -> ~140 LoC | parser / utility | string -> styled span tokens | `crates/vox-gui/ui/src/components/surfaces/Chat/ArenaComparisonCard.tsx:18-45` (`FILE_LINE_RE` token splitter) | partial (novel SGR state machine) |
| `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.test.ts` | **Create** | 0 -> ~120 LoC | unit-test | pure transform assertions | `crates/vox-gui/ui/src/hooks/useVirtualList.test.ts:1-104` | role-match |
| `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts` | Modify | 92 -> ~120 LoC | xterm-addon | lifecycle + context-loss fallback | `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts:1-92` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts` | **Create** | 0 -> ~230 LoC | xterm-addon | buffer search + `ILinkProvider` + Unicode 11 | `crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts:24-91` + `node_modules/@xterm/xterm/typings/xterm.d.ts` | partial (first-party xterm addons) |
| `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.test.ts` | **Create** | 0 -> ~160 LoC | unit-test | mock `Terminal` buffer + link hover/click + search | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx:22-72` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx` | Modify | 281 -> ~320 LoC | component | PTY stream <-> xterm + addons + `ALT_SCREEN` spy | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:1-281` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx` | Modify | 138 -> ~200 LoC | component-test | RTL + `@xterm/xterm` mock | `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.test.tsx:1-138` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockCard.tsx` | **Create** | 0 -> ~230 LoC | component | block item -> Warp card UI + actions | `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx:95-235` | exact (extracted from `BlockStreamView.tsx`) |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx` | Modify | 246 -> ~240 LoC | component | block list -> virtualized / direct stream + sticky prompt | `crates/vox-gui/ui/src/components/surfaces/Explorer/ExplorerTree.tsx:75-107,272-298` + `src/hooks/useVirtualList.ts` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx` | Modify | 168 -> ~260 LoC | component-test | RTL + jsdom `clientHeight` virtualization test | `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.test.tsx:1-168` + `src/hooks/useVirtualList.test.ts:6-35` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.tsx` | **Create** | 0 -> ~270 LoC | component | unified single-viewport BlockStream + warm VT + search bar + composer | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:396-520` + `ChatSurface.tsx:273-312` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/TerminalSessionPane.test.tsx` | **Create** | 0 -> ~200 LoC | component-test | RTL (`altScreen` `<16 ms` swap, `afterSeq`, search bar, composer) | `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx:1-163` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.tsx` | **Create** | 0 -> ~190 LoC | component | recursive split tree (`<= 4` leaves) -> `ResizablePanelGroup` | `crates/vox-gui/ui/src/components/ui/BentoGrid.tsx:378-402` + `src/components/ui/Resizable.tsx` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSplitGrid.test.tsx` | **Create** | 0 -> ~150 LoC | component-test | RTL split H/V, max-4 cap, close leaf, focus border | `crates/vox-gui/ui/src/components/ui/BentoGrid.test.tsx:44-90` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/ConsoleSessionBar.tsx` | **Create** | 0 -> ~190 LoC | component | session tabs + split buttons + security/control `VoxSelect`s | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:251-395` | exact (extracted header/bar) |
| `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx` | Modify | 524 -> ~330 LoC | surface-root | multi-session map + split tree state + keyboard shortcuts | `crates/vox-gui/ui/src/components/surfaces/Console/Console.tsx:1-524` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx` | Modify | 163 -> ~230 LoC | component-test | RTL Console surface assertions + wire policy calls | `crates/vox-gui/ui/src/components/surfaces/Console/Console.test.tsx:1-163` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.tsx` | **Create** | 0 -> ~190 LoC | component | `vox_terminal_*` turn receipt / live bridge block -> inline card | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTurnEventRow.tsx:76-146` + `TurnTrace.tsx:18-95` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTerminalBlockCard.test.tsx` | **Create** | 0 -> ~150 LoC | component-test | RTL inline card + `Open in Console` navigation | `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.test.tsx:1-126` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx` | Modify | 96 -> ~130 LoC | component | routes `vox_terminal_*` tool events to `ChatTerminalBlockCard` | `crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx:1-96` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx` | Modify | 785 (touch `Props` + `TurnTrace` call site only; net +12 LoC) | component | threads `onNavigate` down to `TurnTrace` / `ChatTerminalBlockCard` | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx:31-75,455-475` | exact |
| `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` | Modify | 1,867 (replace `TerminalDockPanel` `L273-312` with `<TerminalSessionPane compact ... />`; net -15 LoC) | surface-root | unified dock terminal + `onNavigate` pass-through | `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx:273-312,1136-1159,1243-1311` | exact |

> **Note on existing `BlockStreamView` duplicate test wrapper:** `crates/vox-gui/ui/src/components/surfaces/Console/__tests__/BlockStreamView.test.tsx` is a 1-line re-export (`import '../BlockStreamView.test';`). Keep all assertions in `Console/BlockStreamView.test.tsx` and leave the 1-line re-export untouched so both Vitest entrypoints run identically.

---

## Exact UI Primitives & Icon Registry Reference (Anti-Hallucination SSOT)

### 1. `Icon` Keys (`crates/vox-gui/ui/src/components/ui/Icons.tsx` + `RomanIcons.tsx`)

`Icons.tsx` exports `export const Icon = { ...RomanIcon, ... } as const` (where each entry is `() => JSX.Element` defaulting to `width="15" height="15"`).

**Every valid `Icon.<key>` in `Icons.tsx` (lines 12–426) + `RomanIcons.tsx` (lines 12–130):**
- **Navigation / Chevrons / Arrows:** `Icon.chevR`, `Icon.chevL`, `Icon.chevronDown`, `Icon.chevronUp`, `Icon.arrowUp`, `Icon.arrowDown`, `Icon.arrowLeft`, `Icon.arrowRight`, `Icon.arrowLeftRight`, `Icon.cornerDownLeft`, `Icon.externalLink`, `Icon.rotateCcw`, `Icon.refresh`
- **Terminal / Code / Git / Files:** `Icon.terminal`, `Icon.command`, `Icon.branch`, `Icon.folder`, `Icon.file`, `Icon.book`, `Icon.link`, `Icon.globe`
- **Actions / Controls:** `Icon.search`, `Icon.pin`, `Icon.play`, `Icon.pause`, `Icon.stop`, `Icon.send`, `Icon.plus`, `Icon.minus`, `Icon.close`, `Icon.x`, `Icon.edit`, `Icon.trash`, `Icon.check`, `Icon.scissors`, `Icon.gripVertical`, `Icon.power`, `Icon.ban`
- **Status / Security / Telemetry:** `Icon.shield`, `Icon.lockKeyhole`, `Icon.alert`, `Icon.bell`, `Icon.clock`, `Icon.bolt`, `Icon.spark`, `Icon.eye`, `Icon.cpu`, `Icon.memory`, `Icon.scale`, `Icon.gavel`, `Icon.doubt`, `Icon.target`, `Icon.circleDot`, `Icon.circle`, `Icon.square`, `Icon.star`, `Icon.trophy`, `Icon.trendUp`, `Icon.trendDown`
- **Workspace / Surfaces:** `Icon.dashboard`, `Icon.list`, `Icon.flow`, `Icon.catalog`, `Icon.matrix`, `Icon.settings`, `Icon.message`, `Icon.mic`, `Icon.puzzle`, `Icon.beaker`, `Icon.brain`, `Icon.users`
- **Spread `RomanIcon` keys (also on `Icon.*`):** `Icon.romanColumn`, `Icon.laurelWreath`, `Icon.gladiusShield`, `Icon.denariusCoin`, `Icon.hourglassVia`, `Icon.brazierTorch`, `Icon.astrolabeRadar`, `Icon.aqueductArch`, `Icon.waxSeal`, `Icon.scrollTabula`, `Icon.romanGalley`, `Icon.anvilForge`, `Icon.amphoraUrn`, `Icon.solInvictus`, `Icon.triremeShip`, `Icon.crystalGem`, `Icon.homeVilla`, `Icon.penQuill`

**HALLUCINATION TRAP — Keys that DO NOT EXIST on `Icon`:**
- `Icon.copy` (DOES NOT EXIST)
- `Icon.split` / `Icon.columns` / `Icon.rows` / `Icon.layout` (DO NOT EXIST)
- `Icon.chevronRight` / `Icon.chevronLeft` (DO NOT EXIST — use `Icon.chevR` / `Icon.chevL`)
- `Icon.maximize` / `Icon.minimize` / `Icon.expand` (DO NOT EXIST)
- `Icon.filter` / `Icon.history` / `Icon.key` (DO NOT EXIST — use `Icon.lockKeyhole` or `Icon.clock`)

**Recommended Icon Mapping for Phase 443 UI Controls:**
- **Using `Icon.*` directly (`import { Icon } from '../../ui/Icons'`):**
  - Terminal badge / VT toggle: `<Icon.terminal />`
  - Search open / input icon: `<Icon.search />`
  - Search previous / next match: `<Icon.chevronUp />` / `<Icon.chevronDown />`
  - Close search / close pane / close tab: `<Icon.close />` (or `<Icon.x />`)
  - Pin block: `<Icon.pin />`
  - Collapse / expand block output: `<Icon.chevR />` (collapsed) / `<Icon.chevronDown />` (expanded)
  - Re-run command / run input: `<Icon.play />` or `<Icon.cornerDownLeft />`
  - Running spinner / interrupt (`Ctrl+C`): `<Icon.stop />`
  - Git branch badge: `<Icon.branch />`
  - Working directory badge: `<Icon.folder />`
  - Open in Console / open file jump: `<Icon.externalLink />`
  - Credential prompt lock badge: `<Icon.lockKeyhole />`
  - New session tab: `<Icon.plus />`
- **Using `<VoxIcon>` + `lucide-react` (`^1.72.0` installed in `package.json`, sanctioned by `zeroEmojiAndDingbat.test.ts:194`) for Copy & Split icons missing from `Icon.*`:**
  ```tsx
  import { Columns2, Copy, Check, Rows2 } from 'lucide-react';
  import { VoxIcon } from '../../ui/VoxIcon';

  // Copy command / output button:
  <VoxIcon icon={copied ? Check : Copy} className="size-3.5" />
  // Split Right (horizontal orientation):
  <VoxIcon icon={Columns2} className="size-3.5" />
  // Split Down (vertical orientation):
  <VoxIcon icon={Rows2} className="size-3.5" />
  ```
  `VoxIcon` signature (`crates/vox-gui/ui/src/components/ui/VoxIcon.tsx:8-33`):
  ```tsx
  export type VoxIconProps = Omit<LucideProps, 'ref'> & {
    icon: LucideIcon;
    label?: string; // Omit when inside a labeled <button>; sets aria-hidden when omitted
  };
  ```

### 2. `VoxSelect` (`crates/vox-gui/ui/src/components/ui/Select.tsx:22-55`)

Radix `@radix-ui/react-select` wrapper (NEVER render raw `<select>` — blocked by `src/guards/noRawNativeControlsGuard.test.ts`).
```tsx
import { VoxSelect, type VoxSelectOption } from '../../ui/Select';

export interface VoxSelectOption<V extends string = string> {
  value: V;
  label: React.ReactNode;
  disabled?: boolean;
}

export interface VoxSelectProps<V extends string = string> {
  value: V;
  onValueChange: (next: V) => void;
  options: readonly VoxSelectOption<V>[];
  placeholder?: string;
  id?: string;
  'aria-label'?: string;
  'aria-labelledby'?: string;
  'data-testid'?: string;
  title?: string;
  disabled?: boolean;
  size?: 'sm' | 'xs';       // default 'sm'; use 'xs' for compact console bars
  className?: string;
  contentClassName?: string;
}
```
> **Testing `VoxSelect` in Vitest/jsdom:** `VoxSelect` renders `<SelectPrimitive.Trigger role="combobox" aria-label={ariaLabel} data-testid={dataTestId}>` displaying the selected option's `label` inside `<SelectPrimitive.Value>`. Query it with `screen.getByTestId('security-level-select')` or `screen.getByRole('combobox', { name: /security/i })`.

### 3. `Button` (`crates/vox-gui/ui/src/components/ui/Button.tsx:52-75`)

```tsx
import { Button } from '../../ui/Button';

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'outline' | 'danger' | 'roman' | 'subtle' | 'link';
  size?: 'xs' | 'sm' | 'md' | 'lg' | 'icon';
  loading?: boolean;
  icon?: ReactNode;
  trailingIcon?: ReactNode;
  asChild?: boolean;
}
// Defaults type="button" unless overridden (use type="submit" inside <form>).
```

### 4. `ResizablePanelGroup`, `ResizablePanel`, `ResizableHandle` (`crates/vox-gui/ui/src/components/ui/Resizable.tsx:1-59` + `BentoGrid.tsx:378-402`)

`package.json` installs `react-resizable-panels@^4.22.1`. **CRITICAL v4 API difference:** v4 uses `orientation="horizontal" | "vertical"` on `Group` (`ResizablePanelGroup`), **NOT** v2/v3's `direction` prop! And sizes passed as strings (`defaultSize="50%"`, `minSize="20%"`) match `BentoGrid.tsx:383-398`.

Exact wrapper exports in `crates/vox-gui/ui/src/components/ui/Resizable.tsx`:
```tsx
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from '../../ui/Resizable';

// ResizablePanelGroup props: React.ComponentProps<typeof Group> -> orientation: 'horizontal' | 'vertical', className, id, ...
// ResizablePanel props: React.ComponentProps<typeof Panel> -> id?: string, defaultSize?: number | string, minSize?: number | string, className, ...
// ResizableHandle props: React.ComponentProps<typeof Separator> & { withHandle?: boolean }
```
Exact canonical usage from `crates/vox-gui/ui/src/components/ui/BentoGrid.tsx:378-402` (verified working in jsdom without mocks in `BentoGrid.test.tsx:44-60`):
```tsx
<ResizablePanelGroup orientation="horizontal" className="h-full min-h-0 w-full flex-1">
  <ResizablePanel id={leftNode.id} defaultSize="50%" minSize="20%" className="min-h-0 min-w-0">
    {/* child pane */}
  </ResizablePanel>
  <ResizableHandle withHandle aria-label="Resize terminal split" />
  <ResizablePanel id={rightNode.id} defaultSize="50%" minSize="20%" className="min-h-0 min-w-0">
    {/* child pane */}
  </ResizablePanel>
</ResizablePanelGroup>
```

### 5. `useVirtualList` (`crates/vox-gui/ui/src/hooks/useVirtualList.ts:1-38` + `ExplorerTree.tsx:75-107,272-298`)

`package.json` installs `@tanstack/react-virtual@^3.14.10`. Always import via `src/hooks/useVirtualList.ts`:
```tsx
import { useVirtualList } from '../../../hooks/useVirtualList';
import { defaultRangeExtractor, type Range } from '@tanstack/react-virtual';
```
Signature (`src/hooks/useVirtualList.ts:9-37`):
```ts
export interface UseVirtualListOptions {
  containerRef: RefObject<HTMLElement | null>;
  count: number;
  estimateSize: (index: number) => number;
  overscan?: number; // default 8
  rangeExtractor?: (range: Range) => number[];
}
// Returns { virtualizer, totalSize, virtualItems }
```
**CRITICAL jsdom Fallback Rule (`src/hooks/useVirtualList.test.ts:6-16`):**
In Vitest/jsdom, `HTMLElement.prototype.clientHeight` is `0` and `getBoundingClientRect()` returns `height: 0` unless explicitly mocked on the scroll container. When `clientHeight === 0`, `@tanstack/react-virtual` computes `0` visible items (`virtualItems.length === 0`) unless `overscan` or a fallback applies.
Therefore, in `BlockStreamView.tsx`:
1. Only activate virtualized positioning when `blocks.length > VIRTUALIZE_THRESHOLD` (`VIRTUALIZE_THRESHOLD = 20`) **AND** `(scrollRef.current?.clientHeight ?? 0) > 0`.
2. When `blocks.length <= 20` **OR** `(scrollRef.current?.clientHeight ?? 0) === 0`, render `blocks.map(...)` directly in normal document flow!
3. When virtualizing (`blocks.length > 20 && containerHeight > 0`), attach `ref={(el) => { if (el) virtualizer.measureElement(el); }}` and `data-index={vItem.index}` on each absolute-positioned row wrapper (`style={{ position: 'absolute', top: 0, left: 0, width: '100%', transform: \`translateY(\${vItem.start}px)\` }}`) inside `<div style={{ height: \`\${totalSize}px\`, position: 'relative', width: '100%' }}>` (matching `ExplorerTree.tsx:273-295`), and include pinned/running tail indices via `rangeExtractor` if needed.
4. In `BlockStreamView.test.tsx`, test large-stream virtualization (e.g., 100 blocks) by mocking dimensions on the scroll container just like `useVirtualList.test.ts:6-16`:
   ```ts
   Object.defineProperty(scrollEl, 'clientHeight', { value: 360, configurable: true });
   scrollEl.getBoundingClientRect = () => ({
     width: 640, height: 360, top: 0, left: 0, bottom: 360, right: 640, x: 0, y: 0, toJSON: () => ({}),
   });
   ```

---

## Pattern Assignments

### 1. `crates/vox-gui/ui/src/transport.ts` & `transport.console.test.ts` (IPC & Wire Alignment)

**Analog:** `crates/vox-gui/ui/src/transport.ts:870-1105` + `crates/vox-gui/src/commands/pty.rs:275-281` + `crates/vox-terminal-core/src/session.rs:17-80` + `crates/vox-terminal-core/src/block.rs:58-75`.

**Exact Backend Contract Findings (Verified against Rust source):**
1. **`term_session_attach` (`crates/vox-gui/src/commands/pty.rs:275-281`):**
   ```rust
   pub async fn term_session_attach(
       state: State<'_, AppState>,
       session_id: String,
       after_seq: Option<u64>,
   ) -> Result<TermAttachmentDto, String>
   ```
   Tauri v2 maps snake_case Rust args to camelCase keys (`sessionId`, `afterSeq`).
   **Backwards-compatibility invariant:** `transport.console.test.ts:56`, `Console.test.tsx:88`, and `TerminalTab.test.tsx:97` assert `expect(invoke).toHaveBeenCalledWith('term_session_attach', { sessionId: 's-1' })` when `afterSeq` is omitted!
   Update `termSessionAttach` in `transport.ts:959-961` so `afterSeq` is only included when defined:
   ```ts
   export async function termSessionAttach(
     sessionId: string,
     afterSeq?: number,
   ): Promise<TermAttachmentDto> {
     return invoke<TermAttachmentDto>('term_session_attach', {
       sessionId,
       ...(afterSeq !== undefined ? { afterSeq } : {}),
     });
   }
   ```
2. **`TermAttachmentDto` & `TermBlockDto` (`transport.ts:887-925` vs `pty.rs:15-31` & `block.rs:58-75`):**
   - `TermBlockDto`: Rust `Block` serializes `id`, `kind`, `origin`, `input`, `output`, `status`, `exit_code`, `pinned`, `prompt_kind`, `credential_prompt`, `cwd: Option<String>`, `git_branch: Option<String>`. (`started_ms` / `ended_ms` are optional UI-tracked timestamps and are not always present on raw Rust `Block` snapshots!). Update `TermBlockDto`:
     ```ts
     export interface TermBlockDto {
       id: number;
       kind: TermBlockKind;
       origin: TermCommandOrigin;
       input: string;
       output: string;
       status: TermBlockStatus;
       exit_code: number | null;
       started_ms?: number;
       ended_ms?: number | null;
       pinned: boolean;
       cwd?: string | null;
       git_branch?: string | null;
       prompt_tail?: string;
       prompt_kind?: TermPromptKind | null;
       credential_prompt?: boolean;
     }
     ```
   - `TermAttachmentDto`: Rust `pty.rs:15-31` emits `session_id`, `seq`, `replayed_events: Vec<SessionEvent>`, `resync_required: bool`, `blocks: Vec<Block>`, `alt_screen: bool`, `cwd: Option<String>`, `git_branch: Option<String>`, `security_level`, `control_mode`. Add `replayed_events?: TermSessionEventPayload[]; resync_required?: boolean;` to `TermAttachmentDto`.
3. **Security Level & Control Mode Wire Mapping (`Console.tsx` UI values vs `TermSecurityLevelWire` / `TermControlModeWire`):**
   - `Console.test.tsx:129-133` asserts the `<VoxSelect>` label `'L3 Workspace Write'` (`l3_workspace_write`), `'L2 Safe Exec'` (`l2_safe_exec`), `'L4 System Exec'` (`l4_system_exec`), whereas Rust `SecurityLevel` (`transport.ts:874-879`) uses snake_case `'l1_read_only' | 'l2_workspace_build' | 'l3_structured_default' | 'l4_host_gated' | 'l5_autopilot_floor'`, and Rust `ControlMode` (`transport.ts:881`) uses `'user' | 'co_pilot' | 'autopilot'` (note `'co_pilot'` with underscore vs UI `'copilot'`).
   - Export pure bidirectional mappers in `transport.ts` (or a shared Console helper):
     ```ts
     export type ConsoleUiSecurityLevel =
       | 'l1_read_only'
       | 'l2_safe_exec'
       | 'l3_workspace_write'
       | 'l4_system_exec'
       | 'l5_autopilot_floor';
     export type ConsoleUiControlMode = 'user' | 'copilot' | 'autopilot';

     export function toWireSecurityLevel(ui: ConsoleUiSecurityLevel): TermSecurityLevelWire {
       switch (ui) {
         case 'l1_read_only': return 'l1_read_only';
         case 'l2_safe_exec': return 'l2_workspace_build';
         case 'l3_workspace_write': return 'l3_structured_default';
         case 'l4_system_exec': return 'l4_host_gated';
         case 'l5_autopilot_floor': return 'l5_autopilot_floor';
       }
     }

     export function fromWireSecurityLevel(wire: string): ConsoleUiSecurityLevel {
       switch (wire) {
         case 'l1_read_only': return 'l1_read_only';
         case 'l2_workspace_build':
         case 'l2_safe_exec': return 'l2_safe_exec';
         case 'l4_host_gated':
         case 'l4_system_exec': return 'l4_system_exec';
         case 'l5_autopilot_floor': return 'l5_autopilot_floor';
         default: return 'l3_workspace_write';
       }
     }

     export function toWireControlMode(ui: ConsoleUiControlMode): TermControlModeWire {
       return ui === 'copilot' ? 'co_pilot' : ui;
     }

     export function fromWireControlMode(wire: string): ConsoleUiControlMode {
       return wire === 'co_pilot' || wire === 'copilot' ? 'copilot' : wire === 'autopilot' ? 'autopilot' : 'user';
     }
     ```
4. **Externally-Tagged `SessionEvent` Discriminated Union (`crates/vox-terminal-core/src/session.rs:17-80`):**
   Rust `#[derive(Serialize)] pub enum SessionEvent` uses serde's default external tagging (`{ BlockOpened: { ... } }`, `{ OutputAppended: { ... } }`, `{ BlockClosed: { ... } }`, `{ AgentMessage: { ... } }`, `{ CwdChanged: { ... } }`, `{ AltScreenChanged: { active: boolean } }`, `{ SnapshotResync: { blocks, resync_required, alt_screen, cwd, git_branch } }`, `{ BlockAwaitingInput: { id, prompt_tail, prompt_kind, credential_prompt } }`, `{ BlockPinned: { id, pinned } }`, `{ SecurityLevelChanged: { security_level } }`, `{ ControlModeChanged: { control_mode } }`). Export `TermSessionEventPayload` in `transport.ts` so `TerminalSessionPane.tsx` handles every variant without `as unknown as Record<string, unknown>` casts.

---

### 2. `sgrToSpans.ts` & `sgrToSpans.test.ts` (Zero-Dep ANSI SGR -> Styled Spans + Plain Text Stripper)

**Role:** Converts raw block `output` containing CSI SGR sequences (`\x1b[...m`), OSC sequences (`\x1b]...(\x07|\x1b\\)`), and other CSI control sequences into:
1. `stripAnsi(raw: string): string` — clean text for Copy Output & block search matching.
2. `parseSgrSpans(raw: string): SgrSpan[]` — compact run-length merged spans `{ text: string; fg?: string; bg?: string; bold?: boolean; dim?: boolean; italic?: boolean; underline?: boolean }` supporting:
   - Reset (`0`), bold (`1`), dim (`2`), italic (`3`), underline (`4`), normal intensity (`22`), no italic (`23`), no underline (`24`)
   - 16 standard/bright foreground (`30–37`, `90–97`, default `39`) and background (`40–47`, `100–107`, default `49`) mapped to theme-harmonized terminal palette CSS variables / hex values (NEVER using banned dark-brown hexes `#0a0805`..`#2a2011` from `src/lib/themeContract.ts:99-112`!)
   - 256-color (`38;5;n` / `48;5;n`) and 24-bit TrueColor (`38;2;r;g;b` / `48;2;r;g;b`)
   - Carriage-return (`\r` without `\n`) progress-bar overwrite normalization so cargo/pnpm spinners don't render 500 stacked lines inside a single block!

**Theme Contract Guardrail (`src/lib/themeContract.ts:86-113`):**
- Never emit any hex in `BANNED_HEX_PATTERNS`: `#0a0805`, `#0e0b07`, `#0f0e0c`, `#110f0d`, `#120e09`, `#14100a`, `#141210`, `#15110b`, `#17120c`, `#171512`, `#18130d`, `#1b150e`, `#1c1916`, `#1e1b17`, `#221b12`, `#2a2011`.
- Use the existing terminal palette from `TerminalTab.tsx:221-237` (`#ef4444`, `#22c55e`, `#eab308`, `#3b82f6`, `#a855f7`, `#06b6d4`, `#e4e4e7`, `#52525b`, etc.).

---

### 3. `webglAddon.ts` & `xtermAddons.ts` & `xtermAddons.test.ts` (WebGL + Buffer Search + File:Line Links + Unicode 11)

**Existing Pattern (`crates/vox-gui/ui/src/components/surfaces/Console/webglAddon.ts:1-92`):**
```ts
import type { Terminal } from '@xterm/xterm';
// Dynamic import via variable string `const WEBGL_SPECIFIER = '@xterm/addon-webgl'`
// Falls back cleanly to canvas renderer on context loss or missing optional peer.
```
**Enhancements in `webglAddon.ts`:**
- Add an optional `onRendererChange?: (mode: 'webgl' | 'canvas') => void` callback on `AttachWebglOptions` and guard `typeof WebGL2RenderingContext === 'undefined'` immediately in jsdom/headless environments before attempting dynamic import, invoking `onContextLoss`/`onRendererChange('canvas')` cleanly on `webglcontextlost`.

**Exact `@xterm/xterm` (`6.0.0`) Types Verified in `node_modules/@xterm/xterm/typings/xterm.d.ts` for `xtermAddons.ts`:**
- **Link Provider (`ILinkProvider`, `ILink`, `IBufferRange`):**
  ```ts
  import type { IBufferRange, ILink, ILinkProvider, Terminal } from '@xterm/xterm';

  // terminal.registerLinkProvider?(provider: ILinkProvider): IDisposable
  // provideLinks(bufferLineNumber: number, callback: (links: ILink[] | undefined) => void): void
  // NOTE: `bufferLineNumber` in xterm's provideLinks is 1-BASED!
  // `terminal.buffer.active.getLine(bufferLineNumber - 1)?.translateToString(true)` is 0-BASED!
  // `IBufferRange.start` and `IBufferRange.end` have `{ x: number, y: number }` which are BOTH 1-BASED (`x = matchIndex + 1`, `y = bufferLineNumber`)!
  ```
- **Path + Line:Col Regex & Click Action (shared between `xtermAddons.ts` and `BlockCard.tsx`):**
   Analog: `crates/vox-gui/ui/src/components/surfaces/Chat/ArenaComparisonCard.tsx:18-45` (`FILE_LINE_RE`) + `App.tsx:1187-1198` (`vox://navigate-surface`).
  Match Rust panics/diagnostics (`--> crates/foo/src/lib.rs:42:9`), TypeScript/Vitest stacks (`src/components/Foo.tsx:18:5`, `/abs/path/file.rs:12`), and URLs (`https?://[^\s)'"]+`):
  ```ts
  export interface ParsedTerminalLink {
    startIndex: number; // 0-based char offset in line
    endIndex: number;   // 0-based exclusive end offset
    text: string;
    kind: 'url' | 'file';
    path?: string;
    line?: number;
    col?: number;
  }
  ```
  On file link activation: dispatch `window.dispatchEvent(new CustomEvent('vox://open-file-location', { detail: { path, line, col } }))` and `window.dispatchEvent(new CustomEvent('vox://navigate-surface', { detail: { view: 'explorer', path, line, col } }))`. On URL activation: `window.open(url, '_blank', 'noopener,noreferrer')`.
- **First-Party Buffer Search Controller (`createTerminalSearchController(terminal: Terminal)`):**
  Reads `terminal.buffer?.active` (`length` lines, `getLine(y)?.translateToString(true)`), supports case-insensitive substring or regex query, maintains `{ matches: Array<{ row: number; col: number; length: number }>, activeIndex: number }`, calls `terminal.select?.(col, row, length)` + `terminal.scrollToLine?.(row)` on `findNext()` / `findPrevious()`, and `terminal.clearSelection?.()` on `clear()`.
- **First-Party Lightweight Unicode 11 Provider (`registerUnicode11Safe(terminal: Terminal)`):**
  Checks `if (terminal.unicode?.register && terminal.unicode) { ... }` (using `IUnicodeVersionProvider` `{ version: '11', wcwidth(codepoint: number): 0 | 1 | 2 }` covering zero-width combining marks `0x0300..0x036F`, `0x200B..0x200F`, `0xFE00..0xFE0F` -> `0`, CJK/fullwidth/emoji ranges `0x1100..0x115F`, `0x2E80..0xA4CF`, `0xAC00..0xD7A3`, `0xF900..0xFAFF`, `0xFE10..0xFE6F`, `0xFF01..0xFF60`, `0x1F300..0x1FAFF` -> `2`, ASCII/Latin/box-drawing `0x2500..0x257F` -> `1`, and setting `terminal.unicode.activeVersion = '11'`).
- **CRITICAL Mock-Safety Rule:** In `Console.test.tsx:36-58` and `TerminalTab.test.tsx:41-61`, `vi.mock('@xterm/xterm')` provides a minimal `MockTerminal` that ONLY defines `loadAddon`, `open`, `write`, `dispose`, `onData`, `cols`, `rows` (`terminal.buffer`, `terminal.unicode`, `terminal.registerLinkProvider` are `undefined`!). Every helper in `xtermAddons.ts` MUST guard optional properties (`if (!terminal?.registerLinkProvider) return { dispose: () => {} };`), so existing mocks never throw `TypeError`!

---

### 4. `TerminalTab.tsx` & `TerminalTab.test.tsx` (Warm Full-VT Surface + `<16 ms` `ALT_SCREEN` Scanner)

**Analog:** `crates/vox-gui/ui/src/components/surfaces/Console/TerminalTab.tsx:1-281`.

**Required Additions & Guardrails:**
1. **Synchronous Byte-Chunk `ALT_SCREEN` Sequence Spy (`<16 ms` TUI Handoff):**
   Export a pure helper `scanAltScreenTransition(chunk: string): boolean | null` checking DECSET/DECRST `1049`, `1047`, `47`:
   ```ts
   const ALT_ENTER_RE = /\x1b\[\?(?:1049|1047|47)h/g;
   const ALT_EXIT_RE = /\x1b\[\?(?:1049|1047|47)l/g;

   export function scanAltScreenTransition(chunk: string): boolean | null {
     let lastIdx = -1;
     let nextState: boolean | null = null;
     for (const m of chunk.matchAll(ALT_ENTER_RE)) {
       if ((m.index ?? 0) > lastIdx) {
         lastIdx = m.index ?? 0;
         nextState = true;
       }
     }
     for (const m of chunk.matchAll(ALT_EXIT_RE)) {
       if ((m.index ?? 0) > lastIdx) {
         lastIdx = m.index ?? 0;
         nextState = false;
       }
     }
     return nextState;
   }
   ```
   Inside `TerminalTab`'s `listenPtyOutput(sid, (b64) => { ... })` handler (`TerminalTab.tsx:128-140`), immediately invoke `onAltScreenHint?.(transition)` synchronously when `transition !== null` **before** waiting on the async `term:session-event` round-trip!
2. **Unmocked jsdom Resilience (`ChatSurface.test.tsx` safety!):**
   In `ChatSurface.test.tsx`, `@xterm/xterm` is **NOT** mocked (`window.matchMedia` and canvas 2D context are absent in jsdom!). Wrap `new Terminal(...)`, `term.open(containerRef.current)`, `fitAddon.fit()`, and addon registration in `try { ... } catch { /* headless/jsdom fallback */ }` inside `TerminalTab.tsx` so mounting `<TerminalSessionPane>` inside `ChatSurface.tsx`'s `TerminalDockPanel` never crashes any `ChatSurface.test.tsx` test!
3. **Font Contract Guardrail (`src/lib/themeContract.ts:86-94`):**
   Keep `fontFamily: "'JetBrains Mono', 'Cascadia Code', 'SF Mono', monospace"` (`TerminalTab.tsx:239`). **NEVER** add `Menlo`, `Monaco`, `Consolas`, or `Courier` — `themeContract.test.ts` scans all `.ts`/`.tsx` files in `src/` and fails on those literals!
4. **Expose Search Handle or Prop:**
   Accept optional props on `TerminalTabProps`:
   ```ts
   export interface TerminalTabProps {
     sessionId?: string | null;
     cwd?: string;
     className?: string;
     active?: boolean;               // triggers fitAddon.fit() + focus when toggled visible
     skipInitialAttach?: boolean;    // true when parent TerminalSessionPane already called termSessionAttach
     searchQuery?: string;
     searchStep?: number;            // increment/decrement or ref handle for next/prev match
     onSearchStatusChange?: (status: { count: number; activeIndex: number }) => void;
     onAltScreenHint?: (active: boolean) => void;
     onSessionReady?: (sessionId: string) => void;
   }
   ```
   > **Preserve standalone `TerminalTab` behavior:** When `skipInitialAttach` is omitted/`false` (as in `TerminalTab.test.tsx`), `TerminalTab` still calls `termSessionAttach(sid)` itself so `TerminalTab.test.tsx:97` (`expect(termSessionAttach).toHaveBeenCalledWith('ext-99')`) passes untouched! When rendered inside `TerminalSessionPane`, pass `skipInitialAttach` so a single session never double-attaches on mount.

---

### 5. `BlockCard.tsx` & `BlockStreamView.tsx` & `BlockStreamView.test.tsx` (Warp Block Cards + Virtualization)

**Analog:** `crates/vox-gui/ui/src/components/surfaces/Console/BlockStreamView.tsx:1-246` + `ExplorerTree.tsx:75-107,272-298`.

**Preserve Every Existing `data-testid` Required by `BlockStreamView.test.tsx` and `Console.test.tsx`:**
- `data-testid="block-stream-view"` on the scroll container (`BlockStreamView.tsx:94`)
- `data-testid={\`term-block-\${block.id}\`}` with `data-block-status={block.status}` and `data-block-origin={block.origin}` (`BlockStreamView.tsx:100-104`)
- `data-testid={\`term-block-pin-\${block.id}\`}` (`BlockStreamView.tsx:148`)
- `data-testid={\`term-block-awaiting-\${block.id}\`}` and `data-testid={\`term-block-credential-badge-\${block.id}\`}` (`BlockStreamView.tsx:167-177`)
- `data-testid={\`term-block-prompt-form-\${block.id}\`}`, `data-testid={\`term-block-prompt-input-\${block.id}\`}`, `data-testid={\`term-block-prompt-send-\${block.id}\`}` (`BlockStreamView.tsx:184-216`)

**New Warp-Parity Capabilities in `BlockCard.tsx` + `BlockStreamView.tsx`:**
1. **Header Metadata Badges:**
   - Origin badge (`user` / `agent` / `agent_background` / `system`)
   - CWD basename chip (`block.cwd`, full path in `title`) with `<Icon.folder />` + Git branch chip (`block.git_branch`) with `<Icon.branch />` when present
   - Exit status pill (`exit 0` emerald vs non-zero red `exit ${block.exit_code}`) + duration (`block.ended_ms != null && block.started_ms != null ? formatDuration(block.ended_ms - block.started_ms) : null`)
2. **Action Toolbar on Each Card:**
   - **Collapse / Expand Output** (`data-testid={\`term-block-collapse-\${block.id}\`}`, `<Icon.chevR />` / `<Icon.chevronDown />`)
   - **Copy Command** (`data-testid={\`term-block-copy-cmd-\${block.id}\`}`) — `navigator.clipboard?.writeText(block.input)`
   - **Copy Output** (`data-testid={\`term-block-copy-out-\${block.id}\`}`) — `navigator.clipboard?.writeText(stripAnsi(block.output))`
   - **Re-run Command** (`data-testid={\`term-block-rerun-\${block.id}\`}`, calls `onRerunCommand?.(block.input)`)
   - **Pin Block** (`data-testid={\`term-block-pin-\${block.id}\`}`, `<Icon.pin />`, text/aria-label preserving `Pin`/`Pinned` for `BlockStreamView.test.tsx:66`)
3. **Rich SGR + Clickable Path Output (`BlockCard.tsx`):**
   - Render `parseSgrSpans(block.output)` spans with inline `color` / `backgroundColor` / `fontWeight` / `fontStyle` / `textDecoration`, splitting span text via `parseTerminalLinks` so `file:line:col` tokens become clickable `<button type="button" className="underline decoration-dotted ...">` links!
4. **Sticky Running / Pinned Context + Virtualization (`BlockStreamView.tsx`):**
   - Filter blocks when `searchQuery` is active (matching `block.input` or `stripAnsi(block.output)` case-insensitively) and expose match count.
   - Render virtualized list via `useVirtualList` when `filteredBlocks.length > 20 && containerHeight > 0`, falling back cleanly to normal flow when `<= 20` or in unmocked `clientHeight === 0` jsdom tests.

---

### 6. `TerminalSessionPane.tsx` & `TerminalSessionPane.test.tsx` (Unified Single-Viewport Pane — `TERM-PANE-01`)

**Replaces BOTH:**
1. The stacked double-pane layout in `Console.tsx:419-457` (which previously rendered `<TerminalTab className="h-[46%]" />` ABOVE `<BlockStreamView />` simultaneously!).
2. The raw-VT-only `TerminalDockPanel` body in `ChatSurface.tsx:273-312`!

**Architecture & DOM Contract:**
```tsx
export interface TerminalSessionPaneProps {
  sessionId: string;
  cwd?: string;
  focused?: boolean;
  compact?: boolean; // true inside ChatSurface bottom dock
  initialBlocks?: TermBlockDto[];
  initialAltScreen?: boolean;
  onFocusPane?: () => void;
  onSessionMetadata?: (meta: {
    cwd?: string | null;
    gitBranch?: string | null;
    altScreen: boolean;
    securityLevel?: ConsoleUiSecurityLevel;
    controlMode?: ConsoleUiControlMode;
  }) => void;
}
```
1. **Single Viewport Stacking (`<16 ms` `ALT_SCREEN` CSS Swap + Test Landmark Compatibility):**
   - Keep **both** `<BlockStreamView>` and `<TerminalTab skipInitialAttach active={showFullVt} ... />` mounted inside one `relative min-h-0 flex-1 overflow-hidden` container:
     - `const showFullVt = altScreen || viewMode === 'raw_vt';`
     - `<div className={showFullVt ? 'hidden' : 'flex h-full min-h-0 flex-col'}>` wrapping `<BlockStreamView ... />` + bottom Warp command composer (`data-testid="console-command-input"`, `data-testid="console-interrupt-btn"`).
     - `<div data-testid="terminal-vt-layer" className={showFullVt ? 'relative z-10 h-full w-full opacity-100' : 'pointer-events-none absolute inset-0 -z-10 opacity-0'} aria-hidden={!showFullVt || undefined}>` wrapping `<TerminalTab ... />` (NOTE: do **not** put `aria-hidden="true"` on `<div aria-label="terminal" role="application">` itself when `Console.test.tsx:79` queries `screen.getByLabelText('terminal')` — `getByLabelText` ignores `aria-hidden`, whereas `getByRole` checks it; keeping `opacity-0 pointer-events-none absolute inset-0 -z-10` without `aria-hidden` preserves `screen.getByLabelText('terminal')` AND `screen.getByTestId('block-stream-view')` simultaneously in `Console.test.tsx:79-80`!).
2. **Monotonic `lastSeqRef` Re-attach & `SnapshotResync`:**
   - Track `const lastSeqRef = useRef<number | undefined>(undefined);`.
   - First attach for a fresh `sessionId` calls `termSessionAttach(sessionId)` (`afterSeq === undefined` so `invoke('term_session_attach', { sessionId })` matches existing tests!).
   - Every `listenTermSessionEvents` event updates `if (typeof evt.seq === 'number') lastSeqRef.current = Math.max(lastSeqRef.current ?? 0, evt.seq);`.
   - On reconnect / re-attach when `lastSeqRef.current !== undefined`, call `termSessionAttach(sessionId, lastSeqRef.current)`, replay `att.replayed_events`, or replace state wholesale if `att.resync_required` or `SnapshotResync` arrives.
3. **Inline Search Bar (`Ctrl/Cmd+F` or Search button, `data-testid="terminal-search-bar"`):**
   - Works seamlessly in both `blocks` mode (filters/highlights blocks + shows match count) and `raw_vt` / `altScreen` mode (drives `TerminalTab`'s buffer search controller with next/prev/close buttons using `<Icon.chevronUp />`, `<Icon.chevronDown />`, `<Icon.close />`).
4. **Pane Header Strip:**
   - Shows CWD chip + Git branch chip, `ALT SCREEN` pill when `altScreen === true`, view mode toggle (`Blocks` vs `Raw VT`, `data-testid="terminal-view-mode-toggle"`), and Search toggle (`data-testid="terminal-search-toggle"`).

---

### 7. `ConsoleSplitGrid.tsx`, `ConsoleSessionBar.tsx`, `Console.tsx` & Tests (Split Panes + Multi-Session — `TERM-PANE-02`)

**Split Tree Data Model (`ConsoleSplitGrid.tsx`):**
```ts
export type SplitLeafNode = {
  kind: 'leaf';
  paneId: string;
  sessionId: string;
};

export type SplitBranchNode = {
  kind: 'branch';
  branchId: string;
  orientation: 'horizontal' | 'vertical';
  first: SplitNode;
  second: SplitNode;
};

export type SplitNode = SplitLeafNode | SplitBranchNode;
```
**Pure Tree Operations (export for unit testing in `ConsoleSplitGrid.test.tsx`):**
- `countSplitLeaves(root: SplitNode): number` — hard cap at `MAX_SPLIT_PANES = 4` (disable Split Right / Split Down buttons when `leafCount >= 4`).
- `splitLeafNode(root: SplitNode, targetPaneId: string, orientation: 'horizontal' | 'vertical', newPane: SplitLeafNode): SplitNode`
- `removeLeafNode(root: SplitNode, targetPaneId: string): SplitNode | null` — collapses parent `SplitBranchNode` cleanly into its surviving sibling.
- `replaceLeafSession(root: SplitNode, targetPaneId: string, sessionId: string): SplitNode`

**Layout Persistence (`localStorage`):**
- Persist `{ orientation/shape }` under `vox.console.splitShape.v1` (without hardcoding ephemeral dead `sessionId`s; bind restored slots to newly spawned/attached sessions on mount, defaulting to a single leaf so `Console.test.tsx` mounts exactly 1 session by default!).

**Keyboard Shortcuts (`Console.tsx` container `onKeyDown`):**
- `Ctrl/Cmd+Shift+D` (or toolbar `[Split Right]` `data-testid="console-split-horizontal-btn"`): split focused pane horizontally (spawns a new PTY session via `termSessionCreate({ cwd })` or attaches selected tab).
- `Ctrl/Cmd+Shift+E` (or toolbar `[Split Down]` `data-testid="console-split-vertical-btn"`): split focused pane vertically.
- `Ctrl/Cmd+W` (when `leafCount > 1`, or close button `data-testid={\`console-pane-close-\${pane.paneId}\`}`): close focused split pane and call `termSessionClose(sessionId)` if no other leaf or session tab references that `sessionId`.
- `Ctrl/Cmd+F`: open `TerminalSessionPane` search bar on the focused pane.

**Preserve All Existing `Console.test.tsx` Assertions (`Console.test.tsx:66-162`):**
- `data-testid="console-surface"` on root
- `screen.getByText('Console')` header title
- `data-testid="security-level-select"` (`default 'l3_workspace_write'` -> displays `'L3 Workspace Write'`) — AND wire `onValueChange` to `termSessionSetSecurity?.(focusedSessionId, toWireSecurityLevel(v))`!
- `data-testid="control-mode-select"` (`default 'copilot'` -> displays `'Co-Pilot'`) — AND wire `onValueChange` to `termSessionSetControl?.(focusedSessionId, toWireControlMode(v))`!
- `data-testid="console-command-input"` (`placeholder` matching `/Run command/i`) — pressing `Enter` calls `termCommandSubmit(focusedSessionId, cmd)`!
- `data-testid="console-interrupt-btn"` — clicking calls `termSessionSignal(focusedSessionId, 'SIGINT')`!
- **Update `Console.test.tsx` `vi.mock('../../../transport')` block (`L21-34`)** to also export `termSessionSetSecurity: vi.fn().mockResolvedValue(undefined)` and `termSessionSetControl: vi.fn().mockResolvedValue(undefined)` alongside existing mocks, and add assertions verifying wire policy propagation (`'l2_workspace_build'`, `'co_pilot'`, etc.).

---

### 8. `ChatSurface.tsx` (`TerminalDockPanel`), `ChatTerminalBlockCard.tsx`, `TurnTrace.tsx`, `ChatTranscript.tsx` (`TERM-PANE-04`)

**Part A — Unify `ChatSurface.tsx` Bottom Dock (`ChatSurface.tsx:273-312`):**
Replace the raw `TerminalTab`-only body inside `TerminalDockPanel` (`ChatSurface.tsx:273-312`) with `<TerminalSessionPane compact sessionId={sessionId} cwd={cwd} />` (preserving `data-testid="chat-dock-terminal"`, session creation via `termSessionCreate({ cwd })`, and `publishTerminalBridgeSession(sid)`).
**CRITICAL DockWorkspaceShell Non-Resurrection Rule (`ChatSurface.tsx:1136-1159, 1243-1311`):**
- Do **NOT** modify `closedPanelIds` handling in `DockWorkspaceShell.tsx` or `ChatSurface.tsx:1243-1311`! When the user closes the `'terminal'` dock tab, `closedPanelIds` prevents background telemetry/session refreshes from resurrecting it unless the user explicitly clicks the `'Terminal'` dock strip chip.

**Part B — Inline Chat Terminal Command Cards (`ChatTerminalBlockCard.tsx` + `TurnTrace.tsx` + `ChatTranscript.tsx`):**
**CRITICAL Contract Guardrail (`turnEventContract.test.tsx:43-45`):**
`crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx` asserts strict equality between `TURN_EVENT_KINDS` (`src/lib/turnEvents.ts`) and `contracts/gui/turn-event-kinds.v1.json`.
- **DO NOT** add new kinds to `TURN_EVENT_KINDS` or touch `contracts/gui/turn-event-kinds.v1.json`!
- Instead, in `TurnTrace.tsx` (`crates/vox-gui/ui/src/components/surfaces/Chat/TurnTrace.tsx:18-95`):
  - Detect terminal tool events inside `trace.events` where `(ev.kind === 'tool_receipt' || ev.kind === 'tool_start') && typeof ev.tool === 'string' && ev.tool.startsWith('vox_terminal_')`!
  - Extract structured metadata from `ev` (`tool`, `ok`, `duration_ms`, `summary`, optional `session_id` / `block_id` / `command` / `exit_code` / `output_tail` fields on `TurnEventRecord` — note `TurnEventRecord` in `src/lib/turnEvents.ts:52` has `[key: string]: unknown` index signature!) and optionally correlate with live `subscribeTerminalBridge()` state (`src/lib/terminalBridge.ts:62-97`) when `block_id` matches an active/recent agent block!
  - Render `<ChatTerminalBlockCard event={ev} onOpenConsole={...} />` (`data-testid="chat-terminal-block-card"`) above or inline within `TurnTrace` so terminal executions stand out as rich Warp-style mini-cards even when the generic trace list is collapsed, while non-terminal events continue rendering via `<ChatTurnEventRow>` untouched!
- **`Open in Console` Action (`data-testid="chat-term-open-console"`):**
  Calls `onNavigate?.('console')` (threaded from `ChatSurface.tsx` -> `ChatTranscript.tsx` -> `TurnTrace.tsx` -> `ChatTerminalBlockCard.tsx`) **AND** dispatches:
  ```ts
  window.dispatchEvent(
    new CustomEvent('vox://navigate-surface', {
      detail: { view: 'console', sessionId, blockId },
    }),
  );
  ```
  (matching `App.tsx:1187-1198`'s global `vox://navigate-surface` listener!).

---

## Canonical Test Mock Patterns (Copy-Ready for Vitest)

### 1. `@xterm/xterm` & `@xterm/addon-fit` Mock (`Console.test.tsx:36-62`, `TerminalTab.test.tsx:41-66`)
```tsx
vi.mock('@xterm/xterm', () => {
  class MockTerminal {
    cols = 80;
    rows = 24;
    loadAddon = vi.fn();
    open = vi.fn();
    write = vi.fn();
    dispose = vi.fn();
    focus = vi.fn();
    select = vi.fn();
    clearSelection = vi.fn();
    scrollToLine = vi.fn();
    registerLinkProvider = vi.fn(() => ({ dispose: vi.fn() }));
    unicode = { register: vi.fn(), activeVersion: '6' };
    buffer = {
      active: {
        length: 0,
        getLine: vi.fn(() => undefined),
      },
    };
    onData = vi.fn(() => ({ dispose: vi.fn() }));
  }
  return { Terminal: MockTerminal };
});

vi.mock('@xterm/addon-fit', () => ({
  FitAddon: class {
    fit = vi.fn();
    dispose = vi.fn();
  },
}));
```

### 2. Console `transport` Mock (`Console.test.tsx:21-34`)
```tsx
vi.mock('../../../transport', () => ({
  termSessionCreate: vi.fn(() => Promise.resolve('s-1')),
  termSessionAttach: vi.fn(() => Promise.resolve(mockAttachment)),
  termSessionWrite: vi.fn(() => Promise.resolve()),
  termSessionResize: vi.fn(() => Promise.resolve()),
  termSessionSignal: vi.fn(() => Promise.resolve()),
  termSessionClose: vi.fn(() => Promise.resolve()),
  termSessionSetSecurity: vi.fn(() => Promise.resolve()),
  termSessionSetControl: vi.fn(() => Promise.resolve()),
  termCommandSubmit: vi.fn(() => Promise.resolve(1)),
  termPromptRespond: vi.fn(() => Promise.resolve()),
  termBlockPin: vi.fn(() => Promise.resolve()),
  listenPtyOutput: vi.fn(() => Promise.resolve(() => {})),
  listenPtyExit: vi.fn(() => Promise.resolve(() => {})),
  listenTermSessionEvents: vi.fn(() => Promise.resolve(() => {})),
  toWireSecurityLevel: (v: string) =>
    v === 'l2_safe_exec' ? 'l2_workspace_build' : v === 'l3_workspace_write' ? 'l3_structured_default' : v === 'l4_system_exec' ? 'l4_host_gated' : v,
  fromWireSecurityLevel: (v: string) =>
    v === 'l2_workspace_build' ? 'l2_safe_exec' : v === 'l3_structured_default' ? 'l3_workspace_write' : v === 'l4_host_gated' ? 'l4_system_exec' : v,
  toWireControlMode: (v: string) => (v === 'copilot' ? 'co_pilot' : v),
  fromWireControlMode: (v: string) => (v === 'co_pilot' ? 'copilot' : v),
}));
```

---

## Shared Patterns & Non-Negotiable Static Guardrails

Every commit in `crates/vox-gui/ui/src/` is scanned by repo-wide Vitest guard suites (`src/guards/*.test.ts` and `src/lib/themeContract.test.ts`). Violating any of these breaks CI immediately:

1. **`src/guards/zeroEmojiAndDingbat.test.ts`:**
   - **ZERO** emojis (`\p{Extended_Pictographic}`), dingbats (`\u2600-\u27BF`), geometric shapes (`\u25A0-\u25FF` like `●`, `▲`, `▼`, `▪`), technical return symbols (`\u23CE` `⏎`, `\u21B5` `↵`), arrows (`\u2197` `↗`, `\u21BA-\u21C4`), or math circle operators (`\u2295` `⊕`) anywhere in non-comment TypeScript/TSX strings or JSX!
   - Always render `<Icon.* />` or `<VoxIcon icon={...} />` instead of Unicode glyphs! (ASCII text like `->`, `x`, `+`, `-`, `...`, and `·` middle dot `\u00B7` are safe.)
2. **`src/guards/noRawNativeControlsGuard.test.ts`:**
   - **ZERO** `<select` tags or `<input type="range">` anywhere outside `src/components/ui/Select.tsx` and `Slider.tsx`. Always use `<VoxSelect>`.
3. **`src/guards/transportIpcGuard.test.ts` & `src/guards/ipcBoundaries.test.ts`:**
   - **ZERO** `@tauri-apps/api/*` imports outside `crates/vox-gui/ui/src/transport.ts`.
4. **`src/guards/toastBodyGuard.test.ts`:**
   - **ZERO** `body: String(...)`, `setError(String(err))`, or `String(e | err | error)` in UI components! Format errors via `err instanceof Error ? err.message : 'Terminal operation failed'` or `sanitizeErrorForToast(err)` (`src/utils/sanitizeErrorMessage.ts`).
5. **`src/lib/themeContract.test.ts` (`src/lib/themeContract.ts:86-113`):**
   - **ZERO** banned font literals (`Courier New`, `Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`) in any `.ts`/`.tsx`/`.css` file.
   - **ZERO** banned dark-brown hexes (`#0a0805`..`#2a2011`) or dark-brown `rgba(...)` backgrounds. Use semantic Tailwind tokens (`bg-zinc-950`, `bg-zinc-900/60`, `border-zinc-800/80`, `text-zinc-200`, etc.) matching `Console.tsx` and `BlockStreamView.tsx`.
6. **Knip Unused Export Hygiene (`crates/vox-gui/ui/knip.jsonc`):**
   - Every exported symbol in new files (`sgrToSpans.ts`, `xtermAddons.ts`, `BlockCard.tsx`, `TerminalSessionPane.tsx`, `ConsoleSplitGrid.tsx`, `ConsoleSessionBar.tsx`, `ChatTerminalBlockCard.tsx`) should be imported by production code or unit tests so `knip` reports zero dead exports.
7. **`vox-code-audit` `god_object` Ceiling (`< 500` non-blank LoC):**
   - Keep every new/modified `.ts`/`.tsx` file in `Console/` well below `500` non-blank lines (target `140–330` LoC per file).
8. **Scoped Verification Command Rules (`.agents/rules/00-core.md`):**
   - Run targeted Vitest suites with `timeout 180s pnpm --dir crates/vox-gui/ui exec vitest run <paths>` (including `src/guards/zeroEmojiAndDingbat.test.ts`, `src/guards/noRawNativeControlsGuard.test.ts`, `src/guards/transportIpcGuard.test.ts`, `src/lib/themeContract.test.ts`, and `src/components/surfaces/Chat/turnEventContract.test.tsx`).

---

## No Analog Found (Novel First-Party Helpers)

| File | Role | Why No Exact In-Repo Analog | Concrete Implementation Approach |
|---|---|---|---|
| `crates/vox-gui/ui/src/components/surfaces/Console/sgrToSpans.ts` | ANSI SGR -> React style span tokenizer | `BlockStreamView.tsx:225` previously rendered raw `<pre>{block.output}</pre>` with zero ANSI parsing, and `package.json` intentionally avoids external ANSI-to-HTML packages | Pure ~140 LoC state-machine scanner over `\x1b\[([0-9;]*)m` + control stripper + `\r` line-overwrite normalizer using safe Tailwind/xterm hex palette |
| `crates/vox-gui/ui/src/components/surfaces/Console/xtermAddons.ts` | Buffer search + `file:line:col` `ILinkProvider` + Unicode 11 provider | `@xterm/addon-search`, `@xterm/addon-web-links`, and `@xterm/addon-unicode11` are not installed in `package.json`; `@xterm/xterm@6.0.0` exposes `registerLinkProvider`, `buffer.active`, and `unicode.register` directly | First-party ~230 LoC module built against `node_modules/@xterm/xterm/typings/xterm.d.ts` with optional-chaining guards for lightweight Vitest `Terminal` mocks |

---

## Metadata

**Pattern mapping confidence:** HIGH — 100% of UI primitives, icon keys, wire DTOs, Tauri Rust commands, `@xterm/xterm` typings, `react-resizable-panels` v4 props, `@tanstack/react-virtual` jsdom behavior, and static repo guards were verified directly from source.
**Ready for planning (`gsd-planner`):** YES.
