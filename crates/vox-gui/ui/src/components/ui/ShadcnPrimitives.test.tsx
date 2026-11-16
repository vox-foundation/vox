import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from './Accordion';
import { Button, buttonVariants } from './Button';
import { ChartContainer, ChartTooltipContent, type ChartConfig } from './Chart';
import { Checkbox } from './Checkbox';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from './Collapsible';
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandSeparator,
  CommandShortcut,
} from './Command';
import { DataTable } from './DataTable';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogTitle,
  AlertDialogTrigger,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from './Dialog';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from './DropdownMenu';
import { Popover, PopoverContent, PopoverRoot, PopoverTrigger } from './Popover';
import { RadioGroup, RadioGroupItem } from './RadioGroup';
import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from './Resizable';
import { RomanRotaryKnob } from './RomanRotaryKnob';
import { ScrollArea } from './ScrollArea';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from './Select';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from './Sheet';
import { Slider } from './Slider';
import { Switch } from './Switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from './Tabs';
import { Tooltip, TooltipProvider } from './Tooltip';

const indexCss = readFileSync(resolve(__dirname, '../../index.css'), 'utf8');

describe('Phase 431 CSS escape hatches & z-index ladder (AXIS-SHADCN-01)', () => {
  it('declares tw-animate-css, @theme inline aliases, and 6-rung z-index ladder', () => {
    expect(indexCss).toContain("@import 'tw-animate-css';");
    expect(indexCss).toContain('@theme inline');
    expect(indexCss).toContain('--z-shell-chrome: 40');
    expect(indexCss).toContain('--z-drawer: 60');
    expect(indexCss).toContain('--z-dialog: 70');
    expect(indexCss).toContain('--z-popover: 80');
    expect(indexCss).toContain('--z-toast: 90');
    expect(indexCss).toContain('--z-tooltip: 100');
  });

  it('scopes legacy button overrides away from data-slot, excludes -overlay scrims, and locks scroll-lock margin', () => {
    expect(indexCss).toContain(':not([data-slot])');
    expect(indexCss).toContain(':not([data-slot$="-overlay"])');
    expect(indexCss).toContain('[data-radix-popper-content-wrapper]');
    expect(indexCss).toContain('html body[data-scroll-locked]');
  });
});

describe('RomanRotaryKnob (AXIS-SHADCN-03)', () => {
  it('renders vector SVG with slider ARIA attributes and readout', () => {
    render(
      <RomanRotaryKnob
        ariaLabel="Sampling temperature"
        defaultValue={0.7}
        min={0}
        max={2}
        step={0.05}
        formatValue={(v) => `${v.toFixed(2)} temp`}
      />
    );

    const slider = screen.getByRole('slider', { name: 'Sampling temperature' });
    expect(slider).toHaveAttribute('data-slot', 'roman-rotary-knob');
    expect(slider).toHaveAttribute('aria-valuemin', '0');
    expect(slider).toHaveAttribute('aria-valuemax', '2');
    expect(slider).toHaveAttribute('aria-valuenow', '0.7');
    expect(slider).toHaveAttribute('aria-valuetext', '0.70 temp');
    expect(slider.querySelector('svg')).not.toBeNull();
    expect(screen.getByText('0.70 temp')).toBeInTheDocument();
  });

  it('supports vertical pointer drag and commits only on pointerup', () => {
    const onChange = vi.fn();
    const onCommit = vi.fn();
    render(
      <RomanRotaryKnob
        ariaLabel="Gain knob"
        defaultValue={50}
        min={0}
        max={100}
        step={1}
        onValueChange={onChange}
        onValueCommit={onCommit}
      />
    );

    const slider = screen.getByRole('slider', { name: 'Gain knob' });
    fireEvent.pointerDown(slider, { pointerId: 1, clientY: 200 });
    fireEvent.pointerMove(slider, { pointerId: 1, clientY: 130 }); // +70px = +50% of 140px sweep -> 100
    expect(onChange).toHaveBeenCalledWith(100);
    expect(onCommit).not.toHaveBeenCalled();

    fireEvent.pointerUp(slider, { pointerId: 1, clientY: 130 });
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(100);
  });

  it('supports Shift 0.1x fine drag, double-click reset to defaultValue, keyboard steps, and detents', () => {
    const onChange = vi.fn();
    const onCommit = vi.fn();
    render(
      <RomanRotaryKnob
        ariaLabel="Precision knob"
        defaultValue={50}
        min={0}
        max={100}
        step={1}
        detents={[0, 50, 75, 100]}
        onValueChange={onChange}
        onValueCommit={onCommit}
      />
    );

    const slider = screen.getByRole('slider', { name: 'Precision knob' });

    // Shift fine drag: +70px with Shift -> +5 instead of +50 -> 55
    fireEvent.pointerDown(slider, { pointerId: 2, clientY: 200 });
    fireEvent.pointerMove(slider, { pointerId: 2, clientY: 130, shiftKey: true });
    fireEvent.pointerUp(slider, { pointerId: 2, clientY: 130, shiftKey: true });
    expect(slider).toHaveAttribute('aria-valuenow', '55');

    // Keyboard steps: ArrowUp (+1 -> 56), Home (-> 0), End (-> 100)
    fireEvent.keyDown(slider, { key: 'ArrowUp' });
    expect(slider).toHaveAttribute('aria-valuenow', '56');
    fireEvent.keyDown(slider, { key: 'Home' });
    expect(slider).toHaveAttribute('aria-valuenow', '0');
    fireEvent.keyDown(slider, { key: 'End' });
    expect(slider).toHaveAttribute('aria-valuenow', '100');

    // Double-click resets to defaultValue (50) and commits
    onCommit.mockClear();
    fireEvent.doubleClick(slider);
    expect(slider).toHaveAttribute('aria-valuenow', '50');
    expect(onCommit).toHaveBeenCalledWith(50);

    // Detent snap near 75 (74 snaps to 75)
    fireEvent.pointerDown(slider, { pointerId: 3, clientY: 200 });
    fireEvent.pointerMove(slider, { pointerId: 3, clientY: 166.4 }); // +24 -> 74 -> snaps to 75
    fireEvent.pointerUp(slider, { pointerId: 3, clientY: 166.4 });
    expect(slider).toHaveAttribute('aria-valuenow', '75');
  });
});

describe('Shadcn / Radix Primitive Kit (AXIS-SHADCN-02 & AXIS-SHADCN-04)', () => {
  it('Slider forwards mandatory ariaLabel onto Thumb and emits onValueCommit', () => {
    const onCommit = vi.fn();
    render(
      <Slider
        ariaLabel="Routing threshold"
        defaultValue={[40]}
        min={0}
        max={100}
        step={5}
        onValueCommit={onCommit}
      />
    );

    const thumb = screen.getByRole('slider', { name: 'Routing threshold' });
    expect(thumb).toHaveAttribute('data-slot', 'slider-thumb');
    thumb.focus();
    fireEvent.keyDown(thumb, { key: 'ArrowRight' });
    expect(thumb).toHaveAttribute('aria-valuenow', '45');
    expect(onCommit).toHaveBeenCalledWith([45]);
  });

  it('Button, Switch, Checkbox, and RadioGroup expose data-slot and accessible states', async () => {
    const user = userEvent.setup();
    const onSwitch = vi.fn();
    const onCheck = vi.fn();
    const onRadio = vi.fn();

    render(
      <div>
        <Button variant="roman" size="sm">
          Execute
        </Button>
        <span className={buttonVariants({ variant: 'subtle', size: 'xs' })}>Badge</span>
        <Switch aria-label="Enable guard" onCheckedChange={onSwitch} />
        <Checkbox aria-label="Snap ticks" onCheckedChange={onCheck} />
        <RadioGroup aria-label="Density" defaultValue="compact" onValueChange={onRadio}>
          <RadioGroupItem value="compact" aria-label="Compact" />
          <RadioGroupItem value="comfortable" aria-label="Comfortable" />
        </RadioGroup>
      </div>
    );

    const btn = screen.getByRole('button', { name: 'Execute' });
    expect(btn).toHaveAttribute('data-slot', 'button');
    expect(btn).toHaveAttribute('type', 'button');

    const sw = screen.getByRole('switch', { name: 'Enable guard' });
    expect(sw).toHaveAttribute('data-slot', 'switch');
    await user.click(sw);
    expect(onSwitch).toHaveBeenCalledWith(true);

    const cb = screen.getByRole('checkbox', { name: 'Snap ticks' });
    expect(cb).toHaveAttribute('data-slot', 'checkbox');
    await user.click(cb);
    expect(onCheck).toHaveBeenCalledWith(true);

    const radio = screen.getByRole('radio', { name: 'Comfortable' });
    await user.click(radio);
    expect(onRadio).toHaveBeenCalledWith('comfortable');
  });

  it('Select opens portaled listbox and selects option', async () => {
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <Select defaultValue="arcane" onValueChange={onValueChange}>
        <SelectTrigger aria-label="Theme profile">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            <SelectLabel>Profiles</SelectLabel>
            <SelectItem value="arcane">Arcane</SelectItem>
            <SelectItem value="slate">Obsidian Slate</SelectItem>
          </SelectGroup>
        </SelectContent>
      </Select>
    );

    const trigger = screen.getByRole('combobox', { name: 'Theme profile' });
    expect(trigger).toHaveAttribute('data-slot', 'select-trigger');
    await user.click(trigger);
    const slateOpt = await screen.findByRole('option', { name: 'Obsidian Slate' });
    await user.click(slateOpt);
    expect(onValueChange).toHaveBeenCalledWith('slate');
  });

  it('Dialog, AlertDialog, and Sheet render translucent -overlay slots on the z-index ladder', async () => {
    const user = userEvent.setup();
    const { unmount: unmountDialog } = render(
      <Dialog open>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Modal Title</DialogTitle>
            <DialogDescription>Modal Body</DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button size="xs">Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );

    const dialogOverlay = document.querySelector('[data-slot="dialog-overlay"]');
    expect(dialogOverlay).not.toBeNull();
    expect(dialogOverlay?.className).toContain('bg-black/55');
    expect(dialogOverlay?.className).toContain('z-(--z-dialog)');
    unmountDialog();

    const { unmount: unmountSheet } = render(
      <Sheet open>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>Drawer Title</SheetTitle>
            <SheetDescription>Drawer Body</SheetDescription>
          </SheetHeader>
        </SheetContent>
      </Sheet>
    );

    const sheetOverlay = document.querySelector('[data-slot="sheet-overlay"]');
    expect(sheetOverlay).not.toBeNull();
    expect(sheetOverlay?.className).toContain('z-(--z-drawer)');
    unmountSheet();

    render(
      <AlertDialog>
        <AlertDialogTrigger asChild>
          <Button size="xs">Confirm Reset</Button>
        </AlertDialogTrigger>
        <AlertDialogContent>
          <AlertDialogTitle>Confirm Action</AlertDialogTitle>
          <AlertDialogDescription>Irreversible reset.</AlertDialogDescription>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction>Proceed</AlertDialogAction>
        </AlertDialogContent>
      </AlertDialog>
    );

    await user.click(screen.getByRole('button', { name: 'Confirm Reset' }));
    expect(await screen.findByRole('alertdialog')).toBeInTheDocument();
    expect(document.querySelector('[data-slot="alert-dialog-overlay"]')).not.toBeNull();
  });

  it('Popover, DropdownMenu, Tooltip, Tabs, Accordion, Collapsible, and ScrollArea work end-to-end', async () => {
    const user = userEvent.setup();
    const onMenuSelect = vi.fn();

    render(
      <TooltipProvider>
        <Popover open>
          <div>Legacy Sibling Popover Body</div>
        </Popover>

        <PopoverRoot>
          <PopoverTrigger asChild>
            <Button size="xs">Open Radix Popover</Button>
          </PopoverTrigger>
          <PopoverContent>Explicit Popover Body</PopoverContent>
        </PopoverRoot>

        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="xs">Open Menu</Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent>
            <DropdownMenuLabel>Actions</DropdownMenuLabel>
            <DropdownMenuSeparator />
            <DropdownMenuItem onSelect={onMenuSelect}>Calibrate</DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>

        <Tooltip content="Helpful hint">
          <Button size="xs">Hover Target</Button>
        </Tooltip>

        <Tabs defaultValue="a">
          <TabsList>
            <TabsTrigger value="a">Tab A</TabsTrigger>
            <TabsTrigger value="b">Tab B</TabsTrigger>
          </TabsList>
          <TabsContent value="a">Panel A</TabsContent>
          <TabsContent value="b">Panel B</TabsContent>
        </Tabs>

        <Accordion type="single" collapsible>
          <AccordionItem value="item-1">
            <AccordionTrigger>Section One</AccordionTrigger>
            <AccordionContent>Expanded Details</AccordionContent>
          </AccordionItem>
        </Accordion>

        <Collapsible>
          <CollapsibleTrigger>More Options</CollapsibleTrigger>
          <CollapsibleContent>Hidden Settings</CollapsibleContent>
        </Collapsible>

        <ScrollArea ariaLabel="Log output" className="h-24">
          <div>Scrollable Line</div>
        </ScrollArea>
      </TooltipProvider>
    );

    expect(screen.getByText('Legacy Sibling Popover Body')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Open Radix Popover' }));
    expect(await screen.findByText('Explicit Popover Body')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Open Menu' }));
    await user.click(await screen.findByRole('menuitem', { name: 'Calibrate' }));
    expect(onMenuSelect).toHaveBeenCalledTimes(1);

    await user.click(screen.getByRole('tab', { name: 'Tab B' }));
    expect(screen.getByText('Panel B')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Section One' }));
    expect(screen.getByText('Expanded Details')).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'More Options' }));
    expect(screen.getByText('Hidden Settings')).toBeInTheDocument();

    const scrollRegion = screen.getByRole('region', { name: 'Log output' });
    expect(scrollRegion).toHaveAttribute('tabindex', '0');
  });

  it('Command filters items, Resizable renders handle grip, DataTable sorts columns, and Chart injects CSS vars', async () => {
    const user = userEvent.setup();
    const onCmdSelect = vi.fn();
    const chartConfig: ChartConfig = {
      tokens: { label: 'Tokens', color: 'var(--color-chart-1)' },
    };

    render(
      <div>
        <Command>
          <CommandInput aria-label="Command filter" placeholder="Filter..." />
          <CommandList>
            <CommandEmpty>Empty</CommandEmpty>
            <CommandGroup heading="Navigation">
              <CommandItem value="loquela" onSelect={onCmdSelect}>
                <span>Open Loquela</span>
                <CommandShortcut>G L</CommandShortcut>
              </CommandItem>
              <CommandItem value="settings" onSelect={onCmdSelect}>
                Open Settings
              </CommandItem>
            </CommandGroup>
            <CommandSeparator />
          </CommandList>
        </Command>

        <div className="h-32 w-80">
          <ResizablePanelGroup orientation="horizontal">
            <ResizablePanel defaultSize="50%">Left Pane</ResizablePanel>
            <ResizableHandle withHandle />
            <ResizablePanel defaultSize="50%">Right Pane</ResizablePanel>
          </ResizablePanelGroup>
        </div>

        <DataTable
          rows={[
            { id: '1', name: 'Zulu' },
            { id: '2', name: 'Alpha' },
          ]}
          columns={[{ key: 'name', header: 'Name', sortable: true }]}
          getRowId={(r) => r.id}
        />

        <ChartContainer config={chartConfig}>
          <div />
        </ChartContainer>
        <ChartTooltipContent
          active
          label="Frame 1"
          payload={[{ dataKey: 'tokens', name: 'tokens', value: 42 }]}
        />
      </div>
    );

    const cmdInput = screen.getByPlaceholderText('Filter...');
    fireEvent.change(cmdInput, { target: { value: 'loq' } });
    expect(cmdInput).toHaveValue('loq');
    expect(screen.getByText('Open Loquela')).toBeInTheDocument();
    expect(screen.queryByText('Open Settings')).not.toBeInTheDocument();
    await user.click(screen.getByText('Open Loquela'));
    expect(onCmdSelect).toHaveBeenCalledWith('loquela');

    expect(document.querySelector('[data-slot="resizable-grip"]')).not.toBeNull();
    expect(screen.getByText('Left Pane')).toBeInTheDocument();

    // DataTable TanStack v9 sorting: click Name header to sort ascending (Alpha before Zulu)
    const sortBtn = screen.getByRole('button', { name: 'Name' });
    await user.click(sortBtn);
    const cells = screen.getAllByRole('cell').map((c) => c.textContent);
    expect(cells).toEqual(['Alpha', 'Zulu']);

    const chartEl = document.querySelector('[data-slot="chart"]') as HTMLElement | null;
    expect(chartEl).not.toBeNull();
    expect(chartEl?.style.getPropertyValue('--color-tokens')).toBe('var(--color-chart-1)');
    expect(screen.getByText('42')).toBeInTheDocument();
  });
});
