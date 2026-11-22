import React from 'react';
import { Select } from '@/components/ui';
import { MIB_BYTES } from '@/constants/hardwareSpecs';
import type { DigaModelProfile } from '@/models';
import { SSOT_AM34_BUS_CONSTANTS, SSOT_MODEL_PROJECTIONS } from '@/models/generated/ssot';

const FALLBACK_MODELS: readonly DigaModelProfile[] = SSOT_MODEL_PROJECTIONS.filter(
  p => p.sectorBanks.length > 0 && p.modelId === 'DMR-EH55',
).map(p => {
  const totalBytes = p.sectorBanks.reduce((sum, b) => sum + b.count * b.sectorSizeBytes, 0);
  return {
    model_id: p.modelId,
    series: 'SSOT Profile',
    flash_size_mb: totalBytes / MIB_BYTES,
    flash_size_bytes: totalBytes,
    nvram_base_hex: p.nvramUncachedVaHex ?? SSOT_AM34_BUS_CONSTANTS.nvramCachedBaseHex,
    sector_geometry: p.derivedSectorGeometry ?? '',
    soc_name: 'AM34',
  };
});

export interface DigaModelSelectProps {
  readonly id?: string;
  readonly ariaLabel?: string;
  readonly testId: string;
  readonly modelCatalog: readonly DigaModelProfile[];
  readonly value: string;
  readonly onChange: (modelId: string) => void;
}

export const DigaModelSelect: React.FC<DigaModelSelectProps> = ({
  id,
  ariaLabel,
  testId,
  modelCatalog,
  value,
  onChange,
}) => {
  const models = modelCatalog.length > 0 ? modelCatalog : FALLBACK_MODELS;
  return (
    <Select
      id={id}
      aria-label={ariaLabel}
      data-testid={testId}
      value={value}
      onChange={e => onChange(e.target.value)}
    >
      {models.length === 0 ? (
        <option value="">—</option>
      ) : (
        models.map(item => (
          <option key={item.model_id} value={item.model_id}>
            {item.model_id} ({item.flash_size_mb} MB)
          </option>
        ))
      )}
    </Select>
  );
};
