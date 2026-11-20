import { useCallback, useState } from 'react';
import { useToast } from '@/context/ToastContext';
import { translate } from '@/i18n/useTranslation';
import { copyToClipboard } from '@/services/browserActions';

export function usePreflightSelection() {
  const [checkedPreflightIds, setCheckedPreflightIds] = useState<readonly string[]>([]);

  const togglePreflight = useCallback((id: string) => {
    setCheckedPreflightIds(prev =>
      prev.includes(id) ? prev.filter(item => item !== id) : [...prev, id],
    );
  }, []);

  const resetPreflight = useCallback(() => {
    setCheckedPreflightIds([]);
  }, []);

  return { checkedPreflightIds, togglePreflight, resetPreflight };
}

export function useDigestCopy() {
  const { showToast } = useToast();
  const [copiedHash, setCopiedHash] = useState(false);

  const copyDigest = useCallback(
    (digest: string) => {
      void copyToClipboard(digest);
      setCopiedHash(true);
      showToast({ type: 'info', title: translate('errors.pipelines.copiedSha') });
    },
    [showToast],
  );

  return { copiedHash, copyDigest };
}
