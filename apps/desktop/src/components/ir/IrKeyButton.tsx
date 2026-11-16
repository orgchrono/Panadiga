import React from 'react';
import { Pencil, ShieldAlert } from 'lucide-react';
import type { DynamicRemoteKey } from '@/models/ir';
import { Button } from '@/components/ui';
import { KEY_TONE_CLASSES } from './irToneStyles';
import { cn } from '@/lib/utils';

interface IrKeyButtonProps {
  keyItem: DynamicRemoteKey;
  disabled: boolean;
  editMode?: boolean;
  onTrigger: (key: DynamicRemoteKey) => void;
  onEdit?: (key: DynamicRemoteKey) => void;
}

export const IrKeyButton: React.FC<IrKeyButtonProps> = ({
  keyItem,
  disabled,
  editMode = false,
  onTrigger,
  onEdit,
}) => {
  const handleClick = () => {
    if (editMode && onEdit) {
      onEdit(keyItem);
      return;
    }
    onTrigger(keyItem);
  };

  return (
    <Button
      type="button"
      variant="outline"
      disabled={disabled && !editMode}
      onClick={handleClick}
      data-testid={`ir-key-${keyItem.id}`}
      className={cn(
        'h-auto min-h-[48px] px-2.5 py-2 flex flex-col items-start justify-between gap-1 text-left border transition-colors',
        KEY_TONE_CLASSES[keyItem.tone] ?? KEY_TONE_CLASSES.NEUTRAL,
        editMode && 'ring-1 ring-amber-400/60',
      )}
    >
      <div className="w-full flex items-center justify-between gap-1">
        <span className="text-xs font-semibold leading-tight break-words line-clamp-2">
          {keyItem.label}
        </span>
        {editMode ? (
          <Pencil className="w-3 h-3 shrink-0 text-amber-300" />
        ) : (
          keyItem.confirm_required && <ShieldAlert className="w-3 h-3 shrink-0 text-rose-400" />
        )}
      </div>
      <span className="text-[10px] font-mono opacity-75 tracking-tight truncate w-full">
        {keyItem.wire_preview}
      </span>
    </Button>
  );
};
