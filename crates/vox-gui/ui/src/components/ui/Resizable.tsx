import React from 'react';
import {
  Group,
  Panel,
  Separator,
  type GroupProps,
  type PanelProps,
  type SeparatorProps,
} from 'react-resizable-panels';
import { GripVertical } from 'lucide-react';
import { cn } from '../../lib/cn';

const ResizablePanelGroup = ({ className, orientation = 'horizontal', ...props }: GroupProps) => (
  <Group
    data-slot="resizable-panel-group"
    orientation={orientation}
    className={cn(
      'flex h-full w-full min-h-0 min-w-0',
      orientation === 'vertical' ? 'flex-col' : 'flex-row',
      className
    )}
    {...props}
  />
);

const ResizablePanel = ({ className, ...props }: PanelProps) => (
  <Panel
    data-slot="resizable-panel"
    className={cn('min-h-0 min-w-0 overflow-hidden', className)}
    {...props}
  />
);

export interface ResizableHandleProps extends SeparatorProps {
  withHandle?: boolean;
}

const ResizableHandle = ({ withHandle = false, className, ...props }: ResizableHandleProps) => (
  <Separator
    data-slot="resizable-handle"
    className={cn(
      'relative flex shrink-0 items-center justify-center bg-border-subtle transition-colors hover:bg-primary/60 focus-visible:outline-solid focus-visible:outline-2 focus-visible:outline-offset-1 focus-visible:outline-ring',
      'aria-[orientation=vertical]:w-1.5 aria-[orientation=vertical]:cursor-col-resize aria-[orientation=horizontal]:h-1.5 aria-[orientation=horizontal]:w-full aria-[orientation=horizontal]:cursor-row-resize',
      className
    )}
    {...props}
  >
    {withHandle && (
      <div
        data-slot="resizable-grip"
        className="z-10 flex h-5 w-3.5 items-center justify-center rounded-xs border border-border-strong bg-bg-elevated text-text-muted shadow-xs"
      >
        <GripVertical className="size-2.5" strokeWidth={1.75} aria-hidden="true" />
      </div>
    )}
  </Separator>
);

export { ResizablePanelGroup, ResizablePanel, ResizableHandle };
