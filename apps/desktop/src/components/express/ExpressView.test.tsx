import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Sidebar } from '@/components/Sidebar';
import { ToastProvider } from '@/context/ToastContext';
import { WorkbenchModeProvider } from '@/context/WorkbenchModeContext';
import { setGlobalLocale } from '@/i18n/useTranslation';
import { getDigaModelCatalog } from '@/services/picoService';
import { dispatchIrSignal } from '@/services/irTransportService';
import { store } from '@/store';
import { resetWorkbenchPreferences } from '@/store/slices/workbenchSlice';
import { ExpressView } from '@/views/ExpressView';

describe('ExpressView Dual-Pipeline Missions, JIT Safety Sheet & Engineering Sidebar', () => {
  beforeEach(() => {
    window.localStorage.clear();
    setGlobalLocale('en-US');
    store.dispatch(resetWorkbenchPreferences());
  });

  it('executes Mission 1 (FAT16B 0x06 SD Card + SDXC warning + RecorderDisplayGuide + Technical Drawer)', async () => {
    const onEnterEngineering = vi.fn();
    render(
      <WorkbenchModeProvider>
        <ToastProvider>
          <ExpressView onEnterEngineering={onEnterEngineering} />
        </ToastProvider>
      </WorkbenchModeProvider>,
    );

    expect(screen.getByTestId('express-shell')).toBeTruthy();
    expect(screen.getByTestId('express-mission-sd')).toBeTruthy();
    expect(screen.getByText('FAT16B (0x06) • 32 MiB • LBA 63')).toBeTruthy();

    // Switch to Physical SD and pick 64 GiB SDXC drive to verify advisory & CONFIRM token gate
    fireEvent.click(screen.getByTestId('express-sd-mode-physical'));
    const driveSelect = await screen.findByTestId('express-sd-drive-select');
    fireEvent.change(driveSelect, { target: { value: '\\\\.\\PhysicalDrive4' } });
    expect(await screen.findByText(/SDXC >= 64 GiB Compatibility Warning/i)).toBeTruthy();

    const flashBtn = screen.getByTestId('express-sd-flash-btn');
    expect(flashBtn).toHaveProperty('disabled', true);

    fireEvent.change(screen.getByTestId('express-sd-confirm-input'), {
      target: { value: 'CONFIRM-PhysicalDrive4' },
    });
    expect(flashBtn).toHaveProperty('disabled', false);
    fireEvent.click(flashBtn);

    expect(await screen.findByTestId('express-sd-success')).toBeTruthy();
    const vfdGuide = screen.getByTestId('recorder-display-guide');
    expect(vfdGuide.textContent).toContain('SD');
    expect(vfdGuide.textContent).toContain('READ');
    expect(vfdGuide.textContent).toContain('W 01..W 99');
    expect(vfdGuide.textContent).toContain('COMPLETE');
    expect(vfdGuide.textContent).toContain('F99');
    expect(vfdGuide.textContent).toContain('SW-OFF');

    // Expand collapsed-by-default technical drawer & trigger deep-link
    expect(screen.queryByTestId('express-sd-tech-content')).toBeNull();
    fireEvent.click(screen.getByTestId('express-sd-tech-toggle'));
    expect(screen.getByTestId('express-sd-tech-content').textContent).toContain('0x06');

    fireEvent.click(screen.getByTestId('express-sd-tech-open-engineering'));
    expect(onEnterEngineering).toHaveBeenCalledWith('unbrick');
  });

  it('enforces JIT PicoConnectionSafetySheet once per session on Mission 2 (PANAMON1 U99) and runs Mission 3 (NOR 8 MiB)', async () => {
    const onEnterEngineering = vi.fn();
    render(
      <WorkbenchModeProvider>
        <ToastProvider>
          <ExpressView onEnterEngineering={onEnterEngineering} />
        </ToastProvider>
      </WorkbenchModeProvider>,
    );

    // Mission 2: U99 Quick Rescue via Pico PANAMON1
    fireEvent.mouseDown(screen.getByTestId('express-tab-u99'));
    fireEvent.click(screen.getByTestId('express-tab-u99'));
    expect(await screen.findByTestId('express-mission-u99')).toBeTruthy();

    fireEvent.click(screen.getByTestId('express-u99-run-btn'));
    const sheet = await screen.findByTestId('pico-connection-safety-sheet');
    expect(sheet.textContent).toContain('3.3V LVTTL');
    expect(sheet.textContent).toContain('NSW5V');
    expect(sheet.textContent).toContain('Open-Drain');
    expect(sheet.textContent).toContain('Never Power Pico from P7501');

    const confirmBtn = screen.getByTestId('pico-safety-sheet-confirm-btn');
    expect(confirmBtn).toHaveProperty('disabled', true);
    fireEvent.click(screen.getByTestId('pico-safety-sheet-checkbox'));
    expect(confirmBtn).toHaveProperty('disabled', false);
    fireEvent.click(confirmBtn);

    expect(await screen.findByTestId('express-u99-success')).toBeTruthy();
    expect(screen.getByTestId('express-u99-success').textContent).toContain('SA134..SA141');

    // Mission 3: Safe 8 MiB NOR Backup & Repair (capture via Pico should NOT re-open safety sheet)
    fireEvent.mouseDown(screen.getByTestId('express-tab-nor'));
    fireEvent.click(screen.getByTestId('express-tab-nor'));
    expect(await screen.findByTestId('express-mission-nor')).toBeTruthy();

    fireEvent.click(screen.getByTestId('express-nor-source-pico'));
    fireEvent.click(screen.getByTestId('express-nor-inspect-btn'));
    await waitFor(() => {
      expect(screen.queryByTestId('pico-connection-safety-sheet')).toBeNull();
      expect(screen.getByText('1. Bootloader IPL (SA0..SA10)')).toBeTruthy();
      expect(screen.getByText('2. BGO FTL Table (SA26..SA70)')).toBeTruthy();
      expect(screen.getByText('3. NVRAM U99 (SA134..SA141)')).toBeTruthy();
    });

    fireEvent.click(screen.getByTestId('express-nor-repair-btn'));
    expect(await screen.findByTestId('express-nor-success')).toBeTruthy();
  });

  it('executes Strict 6-Step Vault Pipeline: resolves relational JSON hardware profile, segregates raw/ vs frm/, builds ISO 9660 (1x), and gates HDD write on Mn103Vm PASS', async () => {
    render(
      <WorkbenchModeProvider>
        <ToastProvider>
          <ExpressView onEnterEngineering={vi.fn()} />
        </ToastProvider>
      </WorkbenchModeProvider>,
    );

    expect(screen.getByTestId('strict-workflow-stepper')).toBeTruthy();
    expect(screen.getByTestId('strict-step-1')).toBeTruthy();
    expect(screen.getByTestId('strict-step-6')).toBeTruthy();

    fireEvent.mouseDown(screen.getByTestId('express-tab-vault'));
    fireEvent.click(screen.getByTestId('express-tab-vault'));
    expect(await screen.findByTestId('express-mission-vault')).toBeTruthy();
    expect(await screen.findByTestId('express-vault-resolved-spec')).toBeTruthy();
    expect(screen.getByTestId('express-vault-resolved-spec').textContent).toContain('S29JL064H70TFI00');

    // Write button is strictly locked BEFORE running Mn103Vm gate
    expect(screen.getByTestId('express-vault-write-btn')).toHaveProperty('disabled', true);

    fireEvent.click(screen.getByTestId('express-vault-init-btn'));
    expect(await screen.findByText('sess_dmr_eh55')).toBeTruthy();

    // Ingest .bin -> raw/ and .FRM -> frm/, plus block .FRM forced into raw/
    fireEvent.change(screen.getByTestId('express-vault-ingest-input'), {
      target: { value: 'ic5001_nor_8mb.bin' },
    });
    fireEvent.click(screen.getByTestId('express-vault-ingest-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('express-vault-bucket-raw').textContent).toContain('ic5001_nor_8mb.bin');
    });

    fireEvent.change(screen.getByTestId('express-vault-ingest-input'), {
      target: { value: 'PANA_DVD.FRM' },
    });
    fireEvent.click(screen.getByTestId('express-vault-ingest-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('express-vault-bucket-frm').textContent).toContain('PANA_DVD.FRM');
    });

    fireEvent.change(screen.getByTestId('express-vault-ingest-bucket'), { target: { value: 'raw' } });
    fireEvent.click(screen.getByTestId('express-vault-ingest-btn'));
    expect((await screen.findByTestId('express-vault-ingest-error')).textContent).toContain('NEVER raw/');

    // Synthesize ISO 9660 Level 1 (1x DAO) & verify Mn103Vm gate unlocks write button
    fireEvent.click(screen.getByTestId('express-dvd-create-btn'));
    expect((await screen.findByTestId('express-dvd-success')).textContent).toContain('1x_or_drive_minimum_dao');

    fireEvent.click(screen.getByTestId('express-vault-run-gate-btn'));
    await waitFor(() => {
      expect(screen.getByTestId('express-vault-gate-status').textContent).toContain('PASS');
    });

    fireEvent.change(screen.getByTestId('express-vault-hdd-confirm-input'), {
      target: { value: 'CONFIRM-PhysicalDrive2' },
    });
    const writeBtn = screen.getByTestId('express-vault-write-btn');
    expect(writeBtn).toHaveProperty('disabled', false);
    fireEvent.click(writeBtn);
    expect(await screen.findByTestId('express-vault-write-success')).toBeTruthy();
  });

  it('groups Engineering Sidebar into 5 domain groups and verifies Task 1 mock catalog & IR transport fixes', async () => {
    const onSelectView = vi.fn();
    render(
      <Sidebar
        currentView="dashboard"
        onSelectView={onSelectView}
        vddVolts={3.29}
        hardwareConnected={true}
        indexedManualsCount={30}
      />,
    );

    expect(screen.getByTestId('nav-group-storage').textContent).toContain('Storage & MEIHFS');
    expect(screen.getByTestId('nav-group-nvram').textContent).toContain('NOR & NVRAM Surgery');
    expect(screen.getByTestId('nav-group-sdCrypto').textContent).toContain('SD & Crypto Pipeline');
    expect(screen.getByTestId('nav-group-picoProbe').textContent).toContain('P7501 Hardware Probe');
    expect(screen.getByTestId('nav-group-forensicsDocs').textContent).toContain('Forensics & Docs');

    const catalog = await getDigaModelCatalog();
    expect(catalog.find(m => m.model_id === 'DMR-ES10')?.sector_geometry).toContain('Read-Only OTP');
    expect(catalog.find(m => m.model_id === 'DMR-EH55')?.sector_geometry).toContain('SA134..SA141');
    expect(catalog.find(m => m.model_id === 'DMR-E85H')?.sector_geometry).toContain('UNKNOWN');

    const irRes = await dispatchIrSignal({
      protocol: 'KASEIKYO',
      transport: 'PICO_TAURI',
      device: 0xb0,
      subdevice: 0x00,
      obc: 0x3d,
      respectRemoteMode: false,
      remoteUnit: 1,
      picoGpioPin: 16,
      unitOverride: 'AUTO',
      repeatCount: 1,
    });
    expect(irRes.ok).toBe(true);
    expect(irRes.backendUsed).toContain('Pico IPC');
  });
});
