import { useCallback } from 'react';
import { useTranslation as useI18nextTranslation } from 'react-i18next';
import i18n from './i18n';
import { translations, type Locale, type TranslationKey } from './translations';

export type { Locale, TranslationKey };
export { translations };

function isLocale(value: string): value is Locale {
  return value === 'en-US' || value === 'pt-BR';
}

export const setGlobalLocale = (newLocale: Locale): void => {
  void i18n.changeLanguage(newLocale);
};

export const getGlobalLocale = (): Locale =>
  isLocale(i18n.language) ? i18n.language : 'en-US';

export const translate = (
  key: TranslationKey,
  options?: Record<string, unknown>,
): string => {
  if (options) {
    return i18n.t(key, options);
  }
  return i18n.t(key);
};

/**
 * Functional Programming Hook: Provides declarative, strongly-typed i18n translations
 * backed by i18next with automatic global synchronization across all mounted components.
 */
export function useTranslation() {
  const { t: i18nTranslate, i18n: instance } = useI18nextTranslation();

  const setLocale = useCallback((newLocale: Locale) => {
    setGlobalLocale(newLocale);
  }, []);

  const t = useCallback(
    (key: TranslationKey, options?: Record<string, unknown>): string => {
      if (options) {
        return i18nTranslate(key, options);
      }
      return i18nTranslate(key);
    },
    [i18nTranslate],
  );

  return {
    t,
    locale: isLocale(instance.language) ? instance.language : 'en-US',
    setLocale,
    i18n: instance,
  };
}
