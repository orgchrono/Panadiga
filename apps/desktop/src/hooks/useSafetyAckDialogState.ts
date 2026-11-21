/**
 * @file hooks/useSafetyAckDialogState.ts
 * @brief Shared acknowledgment checkbox & modal open/close handler for Safety Briefings.
 */

import { useCallback, useId, useState } from 'react';

export function useSafetyAckDialogState(
  onConfirmAction: () => void,
  onCloseAction: () => void,
) {
  const checkboxId = useId();
  const [checked, setChecked] = useState(false);

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      if (!nextOpen) {
        setChecked(false);
        onCloseAction();
      }
    },
    [onCloseAction],
  );

  const handleConfirm = useCallback(() => {
    setChecked(false);
    onConfirmAction();
  }, [onConfirmAction]);

  return { checkboxId, checked, setChecked, handleOpenChange, handleConfirm };
}
