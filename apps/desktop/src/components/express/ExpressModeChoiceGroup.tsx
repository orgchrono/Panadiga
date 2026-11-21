import React from 'react';
import { Button } from '@/components/ui';

export interface ModeChoiceOption<T extends string> {
  readonly value: T;
  readonly label: string;
  readonly testId: string;
}

export interface ExpressModeChoiceGroupProps<T extends string> {
  readonly activeValue: T;
  readonly options: readonly ModeChoiceOption<T>[];
  readonly onSelect: (value: T) => void;
  readonly activeVariant?: 'default' | 'emerald';
}

export function ExpressModeChoiceGroup<T extends string>({
  activeValue,
  options,
  onSelect,
  activeVariant = 'default',
}: ExpressModeChoiceGroupProps<T>): React.ReactElement {
  return (
    <div className="flex flex-col gap-1.5">
      {options.length === 0
        ? null
        : options.map(option => (
            <Button
              key={option.value}
              type="button"
              size="sm"
              variant={activeValue === option.value ? activeVariant : 'outline'}
              data-testid={option.testId}
              onClick={() => onSelect(option.value)}
              className="justify-start text-xs"
            >
              {option.label}
            </Button>
          ))}
    </div>
  );
}
