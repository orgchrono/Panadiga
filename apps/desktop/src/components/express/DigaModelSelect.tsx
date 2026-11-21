import React from 'react';
import { Select } from '@/components/ui';
import type { DigaModelProfile } from '@/models';

const FALLBACK_MODELS: readonly DigaModelProfile[] = [
  {
    model_id: 'DMR-EH55',
    series: 'Série EH (HDD+DVD)',
    flash_size_mb: 8,
    flash_size_bytes: 0,
    nvram_base_hex: '0x807F0000',
    sector_geometry: 'SA134..SA141',
    soc_name: 'AM34',
  },
];

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
