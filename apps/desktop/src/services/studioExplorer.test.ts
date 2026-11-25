import { describe, expect, it } from 'vitest';
import { DIGAOS_CONFIG } from '@/models/generated/digaos';
import { rgb565ToCssHex } from './ssotStudioService';

describe('DigaOS SSOT Studio & Silicon Explorer helpers', () => {
  it('converts RGB565 panel swatches into deterministic CSS hex values', () => {
    expect(rgb565ToCssHex('0x0000')).toBe('#000000');
    expect(rgb565ToCssHex('0xFFFF')).toBe('#ffffff');
    expect(rgb565ToCssHex('0xF800')).toBe('#ff0000');
    expect(rgb565ToCssHex('0x07E0')).toBe('#00ff00');
    expect(rgb565ToCssHex('0x001F')).toBe('#0000ff');
  });

  it('ships complete declarative DigaOS menus, themes, power profiles and multi-SoC targets', () => {
    expect(DIGAOS_CONFIG.prompt).toBe('DIGAOS>');
    expect(DIGAOS_CONFIG.menus.length).toBeGreaterThanOrEqual(4);
    expect(DIGAOS_CONFIG.themes.length).toBeGreaterThanOrEqual(3);
    expect(DIGAOS_CONFIG.power_profiles.length).toBeGreaterThanOrEqual(2);
    expect(DIGAOS_CONFIG.soc_targets.some((s) => s.soc_core_id === 'mn10300_am34' && s.mmio_verified)).toBe(true);
    expect(
      DIGAOS_CONFIG.soc_targets.filter((s) => !s.mmio_verified).every((s) => !s.is_primary_executable),
    ).toBe(true);
  });
});
