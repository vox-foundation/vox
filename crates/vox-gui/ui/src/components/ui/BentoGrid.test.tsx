import React, { useEffect } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { voxTransport } from '../../transport';
import {
  BentoCard,
  BentoGrid,
  BentoScrollRegion,
  MasterDetailWorkbench,
  parseWorkbenchLayoutV4,
  serializeWorkbenchLayoutV4,
} from './BentoGrid';
import { EmbeddedSurfaceProvider } from '../dashboard/EmbeddedSurfaceContext';

describe('BentoGrid, BentoScrollRegion & MasterDetailWorkbench (AXIS-BENTO-02 & AXIS-BENTO-04)', () => {
  it('renders @container/surface grid presets and keyboard-focusable BentoScrollRegion', () => {
    render(
      <BentoGrid preset="dense">
        <BentoCard span="wide">
          <BentoScrollRegion ariaLabel="Catalog command rows">
            <p>Focusable scroll content</p>
          </BentoScrollRegion>
        </BentoCard>
      </BentoGrid>,
    );

    const grid = document.querySelector('[data-slot="bento-grid"]');
    expect(grid).not.toBeNull();
    expect(grid?.getAttribute('data-bento-preset')).toBe('dense');

    const region = screen.getByRole('region', { name: 'Catalog command rows' });
    expect(region).toHaveAttribute('tabindex', '0');
    expect(region).toHaveTextContent('Focusable scroll content');
  });

  it('validates .v4 workbench layout schema and rejects corrupt or legacy payloads', () => {
    expect(parseWorkbenchLayoutV4(serializeWorkbenchLayoutV4(62), 55)).toBe(62);
    expect(parseWorkbenchLayoutV4('{"version":3,"masterPct":70}', 55)).toBe(55);
    expect(parseWorkbenchLayoutV4('{"version":4,"masterPct":99}', 55)).toBe(55);
    expect(parseWorkbenchLayoutV4('not-json', 55)).toBe(55);
    expect(parseWorkbenchLayoutV4(null, 55)).toBe(55);
  });

  it('renders ResizablePanelGroup in wide desktop mode (>=960px)', () => {
    render(
      <MasterDetailWorkbench
        storageKey="vox.layout.test-wide.v4"
        forceMode="wide"
        detailTitle="Command Inspector"
        master={<div>Master Table</div>}
        detail={<div>Detail Drawer Body</div>}
      />,
    );

    const workbench = document.querySelector('[data-slot="master-detail-workbench"]');
    expect(workbench?.getAttribute('data-workbench-mode')).toBe('wide');
    expect(document.querySelector('[data-slot="resizable-panel-group"]')).not.toBeNull();
    expect(screen.getByText('Master Table')).toBeInTheDocument();
    expect(screen.getByText('Detail Drawer Body')).toBeInTheDocument();
  });

  it('falls back to Sheet drawer in compact (<960px) or embedded 320px dock mode', () => {
    const onOpenChange = vi.fn();
    render(
      <EmbeddedSurfaceProvider value={true}>
        <MasterDetailWorkbench
          storageKey="vox.layout.test-dock.v4"
          detailTitle="Approval Detail"
          master={<div>Compact Master List</div>}
          detail={<div>Embedded Sheet Inspector</div>}
          onCompactOpenChange={onOpenChange}
        />
      </EmbeddedSurfaceProvider>,
    );

    const workbench = document.querySelector('[data-slot="master-detail-workbench"]');
    expect(workbench?.getAttribute('data-workbench-mode')).toBe('compact');
    expect(document.querySelector('[data-slot="resizable-panel-group"]')).toBeNull();
    expect(screen.getByText('Compact Master List')).toBeInTheDocument();

    fireEvent.click(screen.getByTestId('workbench-open-detail-btn'));
    expect(onOpenChange).toHaveBeenCalledWith(true);
    expect(screen.getByText('Embedded Sheet Inspector')).toBeInTheDocument();
  });

  it('supports custom min/max bounds and hydrates remote preferences without remounting subtrees', async () => {
    expect(parseWorkbenchLayoutV4(serializeWorkbenchLayoutV4(82, 20, 85), 55, 20, 85)).toBe(82);
    expect(parseWorkbenchLayoutV4('{"version":4,"masterPct":90}', 55, 20, 85)).toBe(55);

    let resolvePref!: (val: string | null) => void;
    const getPrefSpy = vi
      .spyOn(voxTransport, 'getGuiPreference')
      .mockImplementation(() => new Promise((res) => { resolvePref = res; }));
    const setPrefSpy = vi.spyOn(voxTransport, 'setGuiPreference').mockResolvedValue();

    let masterMounts = 0;
    function MasterProbe() {
      useEffect(() => {
        masterMounts += 1;
      }, []);
      return <div data-testid="master-probe">Mounted</div>;
    }

    render(
      <MasterDetailWorkbench
        storageKey="vox.layout.test-no-remount.v4"
        forceMode="wide"
        minMasterPct={20}
        minDetailPct={15}
        detailTitle="Inspector"
        master={<MasterProbe />}
        detail={<div>Detail</div>}
      />,
    );

    expect(masterMounts).toBe(1);
    expect(setPrefSpy).not.toHaveBeenCalled();

    await act(async () => {
      resolvePref(JSON.stringify({ version: 4, masterPct: 80 }));
    });

    expect(masterMounts).toBe(1);
    expect(setPrefSpy).not.toHaveBeenCalled();

    getPrefSpy.mockRestore();
    setPrefSpy.mockRestore();
  });

  it('toggles BentoScrollRegion tabIndex based on scroll overflow when dimensions are non-zero', () => {
    const { rerender } = render(
      <BentoScrollRegion ariaLabel="Dynamic region">
        <div>Short</div>
      </BentoScrollRegion>,
    );
    const region = screen.getByRole('region', { name: 'Dynamic region' });
    Object.defineProperty(region, 'clientHeight', { configurable: true, value: 200 });
    Object.defineProperty(region, 'scrollHeight', { configurable: true, value: 80 });
    Object.defineProperty(region, 'clientWidth', { configurable: true, value: 300 });
    Object.defineProperty(region, 'scrollWidth', { configurable: true, value: 300 });

    rerender(
      <BentoScrollRegion ariaLabel="Dynamic region">
        <div>Short updated</div>
      </BentoScrollRegion>,
    );
    expect(region).not.toHaveAttribute('tabindex');

    Object.defineProperty(region, 'scrollHeight', { configurable: true, value: 420 });
    rerender(
      <BentoScrollRegion ariaLabel="Dynamic region">
        <div>Tall updated</div>
      </BentoScrollRegion>,
    );
    expect(region).toHaveAttribute('tabindex', '0');
  });
});
