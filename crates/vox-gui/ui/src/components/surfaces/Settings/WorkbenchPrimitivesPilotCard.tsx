import React, { useState } from 'react';
import { Bar, BarChart, XAxis } from 'recharts';
import { SlidersHorizontal } from 'lucide-react';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from '../../ui/Accordion';
import { Button, buttonVariants } from '../../ui/Button';
import { ChartContainer, ChartTooltip, ChartTooltipContent, type ChartConfig } from '../../ui/Chart';
import { Checkbox } from '../../ui/Checkbox';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../../ui/Collapsible';
import {
  Command,
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from '../../ui/Command';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogOverlay,
  AlertDialogTitle,
  AlertDialogTrigger,
  DialogFooter,
  DialogHeader,
} from '../../ui/Dialog';
import {
  DropdownMenu,
  DropdownMenuCheckboxItem,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../../ui/DropdownMenu';
import { PopoverAnchor, PopoverContent, PopoverRoot, PopoverTrigger } from '../../ui/Popover';
import { RadioGroup, RadioGroupItem } from '../../ui/RadioGroup';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from '../../ui/Resizable';
import { RomanRotaryKnob } from '../../ui/RomanRotaryKnob';
import { ScrollArea, ScrollBar } from '../../ui/ScrollArea';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from '../../ui/Select';
import {
  Sheet,
  SheetClose,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetOverlay,
  SheetTitle,
  SheetTrigger,
} from '../../ui/Sheet';
import { Slider } from '../../ui/Slider';
import { Switch } from '../../ui/Switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '../../ui/Tabs';

const PILOT_CHART_CONFIG: ChartConfig = {
  latencyMs: { label: 'Frame Budget (ms)', color: 'var(--color-chart-1)' },
};

const PILOT_SAMPLES = [
  { rung: 'Shell', latencyMs: 4 },
  { rung: 'Dialog', latencyMs: 7 },
  { rung: 'Popover', latencyMs: 5 },
  { rung: 'Tooltip', latencyMs: 3 },
];

export function WorkbenchPrimitivesPilotCard() {
  const [preset, setPreset] = useState('balanced');
  const [motionGuard, setMotionGuard] = useState(true);
  const [snapGrid, setSnapGrid] = useState(true);
  const [density, setDensity] = useState('compact');
  const [gain, setGain] = useState(64);
  const [blurScale, setBlurScale] = useState(50);
  const [advancedOpen, setAdvancedOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [auditGrid, setAuditGrid] = useState(true);

  return (
    <div
      data-testid="workbench-primitives-pilot-card"
      className="mt-4 space-y-3 rounded-xl border border-border-subtle bg-bg-surface p-3.5"
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <SlidersHorizontal className="size-4 text-primary" strokeWidth={1.75} aria-hidden="true" />
          <div>
            <div className="font-display text-[11px] font-bold tracking-[0.14em] text-text-primary uppercase">
              Primitive Kit & Z-Index Calibration Inspector
            </div>
            <div className="text-[11px] text-text-muted">
              Live dual-theme verification harness for portaled overlays, rotary chronometers, and accessible controls.
            </div>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <Button size="xs" variant="outline">
                Overlay Checks
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuLabel>Calibration Actions</DropdownMenuLabel>
              <DropdownMenuCheckboxItem checked={auditGrid} onCheckedChange={(v) => setAuditGrid(Boolean(v))}>
                Show Z-Index Ladder
              </DropdownMenuCheckboxItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => setPaletteOpen(true)}>
                Open Quick Filter
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          <PopoverRoot>
            <PopoverAnchor className="inline-flex">
              <PopoverTrigger asChild>
                <button type="button" className={buttonVariants({ variant: 'subtle', size: 'xs' })}>
                  Ladder Spec
                </button>
              </PopoverTrigger>
            </PopoverAnchor>
            <PopoverContent align="end" className="w-60 p-2.5 text-xs">
              <div className="font-display text-[10px] font-semibold tracking-wider text-text-primary uppercase">
                6-Rung Portal Scale
              </div>
              <p className="mt-1 text-[11px] text-text-muted">
                shell (40) &lt; drawer (60) &lt; dialog (70) &lt; popover (80) &lt; toast (90) &lt; tooltip (100)
              </p>
            </PopoverContent>
          </PopoverRoot>

          <Sheet>
            <SheetTrigger asChild>
              <Button size="xs" variant="roman">
                Inspect Drawer
              </Button>
            </SheetTrigger>
            <SheetContent side="right">
              <SheetHeader>
                <SheetTitle>Workbench Kit Diagnostics</SheetTitle>
                <SheetDescription>
                  Verifies drawer scrim translucency (`z-drawer: 60`) and keyboard-focusable scroll regions.
                </SheetDescription>
              </SheetHeader>
              <ScrollArea ariaLabel="Calibration command index" className="h-44 rounded-lg border border-border-subtle">
                <Command className="bg-transparent">
                  <CommandInput placeholder="Filter primitive slots..." />
                  <CommandList>
                    <CommandEmpty>No matching primitive slot.</CommandEmpty>
                    <CommandGroup heading="Overlays">
                      <CommandItem value="dialog-overlay">
                        <span>dialog-overlay</span>
                        <CommandShortcut>z-70</CommandShortcut>
                      </CommandItem>
                      <CommandItem value="popover-content">
                        <span>popover-content</span>
                        <CommandShortcut>z-80</CommandShortcut>
                      </CommandItem>
                    </CommandGroup>
                    <CommandSeparator />
                    <CommandGroup heading="Inputs">
                      <CommandItem value="roman-rotary-knob">roman-rotary-knob</CommandItem>
                    </CommandGroup>
                  </CommandList>
                </Command>
                <ScrollBar orientation="horizontal" />
              </ScrollArea>
              <div className="mt-auto flex justify-end">
                <SheetClose asChild>
                  <Button size="sm" variant="secondary">
                    Done
                  </Button>
                </SheetClose>
              </div>
            </SheetContent>
          </Sheet>

          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button size="xs" variant="ghost">
                Reset Calibration
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <DialogHeader>
                <AlertDialogTitle>Reset calibration defaults?</AlertDialogTitle>
                <AlertDialogDescription>
                  Restores rotary gain, blur budget, and density preview controls to baseline values.
                </AlertDialogDescription>
              </DialogHeader>
              <DialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  onClick={() => {
                    setGain(64);
                    setBlurScale(50);
                    setPreset('balanced');
                  }}
                >
                  Reset Defaults
                </AlertDialogAction>
              </DialogFooter>
            </AlertDialogContent>
          </AlertDialog>
        </div>
      </div>

      <Tabs defaultValue="controls">
        <TabsList>
          <TabsTrigger value="controls">Controls & Gauges</TabsTrigger>
          <TabsTrigger value="split">Split & Telemetry</TabsTrigger>
        </TabsList>

        <TabsContent value="controls" className="space-y-3 pt-1">
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
            <div className="space-y-1.5 rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
              <div className="font-display text-[10px] tracking-wider text-text-muted uppercase">
                Motion Profile
              </div>
              <Select value={preset} onValueChange={setPreset}>
                <SelectTrigger aria-label="Motion profile preset">
                  <SelectValue placeholder="Choose profile" />
                </SelectTrigger>
                <SelectContent>
                  <SelectGroup>
                    <SelectLabel>Compositor Profiles</SelectLabel>
                    <SelectItem value="balanced">Balanced (150ms)</SelectItem>
                    <SelectItem value="crisp">Crisp (90ms)</SelectItem>
                    <SelectItem value="reduced">Reduced Motion</SelectItem>
                  </SelectGroup>
                </SelectContent>
              </Select>
              <div className="flex items-center justify-between pt-1 text-xs text-text-secondary">
                <span>Compositor guard</span>
                <Switch
                  aria-label="Compositor motion guard"
                  checked={motionGuard}
                  onCheckedChange={setMotionGuard}
                />
              </div>
              <label className="flex items-center gap-2 pt-0.5 text-xs text-text-secondary">
                <Checkbox
                  aria-label="Snap rotary detents"
                  checked={snapGrid}
                  onCheckedChange={(v) => setSnapGrid(v === true)}
                />
                <span>Snap detents</span>
              </label>
            </div>

            <div className="flex items-center justify-around rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
              <RomanRotaryKnob
                ariaLabel="Chronometer gain"
                label="Gain"
                value={gain}
                defaultValue={64}
                min={0}
                max={100}
                step={1}
                detents={snapGrid ? [0, 25, 50, 64, 75, 100] : undefined}
                formatValue={(v) => `${v}%`}
                onValueChange={setGain}
              />
              <div className="w-28 space-y-1.5">
                <div className="font-display text-[10px] tracking-wider text-text-muted uppercase">
                  Scrim Blur ({blurScale}%)
                </div>
                <Slider
                  ariaLabel="Scrim blur budget"
                  min={0}
                  max={100}
                  step={5}
                  value={[blurScale]}
                  onValueChange={(vals) => setBlurScale(vals[0] ?? 50)}
                />
              </div>
            </div>

            <div className="space-y-1.5 rounded-lg border border-border-subtle bg-bg-elevated p-2.5">
              <div className="font-display text-[10px] tracking-wider text-text-muted uppercase">
                Control Density
              </div>
              <RadioGroup aria-label="Control density" value={density} onValueChange={setDensity}>
                {(['compact', 'default', 'comfortable'] as const).map((opt) => (
                  <label key={opt} className="flex items-center gap-2 text-xs text-text-secondary capitalize">
                    <RadioGroupItem value={opt} aria-label={`Density ${opt}`} />
                    <span>{opt}</span>
                  </label>
                ))}
              </RadioGroup>
            </div>
          </div>

          <Accordion type="single" collapsible className="rounded-lg border border-border-subtle bg-bg-elevated px-3">
            <AccordionItem value="escape-hatches">
              <AccordionTrigger>CSS Escape-Hatch Contract Summary</AccordionTrigger>
              <AccordionContent>
                All primitives carry explicit data-slot markers so Romanesque Carrara and Obsidian Slate rules coexist without specificity collisions.
              </AccordionContent>
            </AccordionItem>
          </Accordion>

          <Collapsible open={advancedOpen} onOpenChange={setAdvancedOpen}>
            <CollapsibleTrigger className="text-[11px] font-medium text-primary underline-offset-4 hover:underline">
              {advancedOpen ? 'Hide overlay diagnostics' : 'Show overlay diagnostics'}
            </CollapsibleTrigger>
            <CollapsibleContent className="pt-1.5 text-[11px] text-text-muted">
              Active preset: {preset} · Density: {density} · Ladder overlay guard: {auditGrid ? 'armed' : 'standby'} (Overlay refs: {String(Boolean(SheetOverlay && AlertDialogOverlay))})
            </CollapsibleContent>
          </Collapsible>
        </TabsContent>

        <TabsContent value="split" className="pt-1">
          <div className="h-40 rounded-lg border border-border-subtle bg-bg-elevated">
            <ResizablePanelGroup orientation="horizontal">
              <ResizablePanel defaultSize="55%" minSize="30%" className="p-2.5">
                <ChartContainer config={PILOT_CHART_CONFIG} className="h-full min-h-[100px]">
                  <BarChart data={PILOT_SAMPLES}>
                    <XAxis dataKey="rung" hide />
                    <ChartTooltip content={<ChartTooltipContent />} />
                    <Bar dataKey="latencyMs" fill="var(--color-latencyMs)" radius={[4, 4, 0, 0]} />
                  </BarChart>
                </ChartContainer>
              </ResizablePanel>
              <ResizableHandle withHandle />
              <ResizablePanel defaultSize="45%" minSize="25%" className="p-2.5 text-xs text-text-secondary">
                <div className="font-display text-[10px] tracking-wider text-text-muted uppercase">
                  Resizable Inspector Pane
                </div>
                <p className="mt-1 text-[11px] text-text-muted">
                  Drag or use arrow keys on the separator grip to resize panes with zero raw ResizeObserver layout thrash.
                </p>
              </ResizablePanel>
            </ResizablePanelGroup>
          </div>
        </TabsContent>
      </Tabs>

      <CommandDialog open={paletteOpen} onOpenChange={setPaletteOpen} title="Quick Primitive Filter">
        <CommandInput placeholder="Jump to preset..." />
        <CommandList>
          <CommandEmpty>No preset found.</CommandEmpty>
          <CommandGroup heading="Presets">
            <CommandItem
              value="balanced"
              onSelect={() => {
                setPreset('balanced');
                setPaletteOpen(false);
              }}
            >
              Balanced (150ms)
            </CommandItem>
            <CommandItem
              value="crisp"
              onSelect={() => {
                setPreset('crisp');
                setPaletteOpen(false);
              }}
            >
              Crisp (90ms)
            </CommandItem>
          </CommandGroup>
        </CommandList>
      </CommandDialog>
    </div>
  );
}
