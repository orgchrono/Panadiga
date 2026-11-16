import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { mergedLocales } from './translations';

export const defaultNS = 'translation';
export const resources = {
  'en-US': { translation: mergedLocales['en-US'] },
  'pt-BR': { translation: mergedLocales['pt-BR'] },
} as const;

i18n
  .use(initReactI18next)
  .init({
    resources,
    lng: 'en-US',
    fallbackLng: 'en-US',
    interpolation: {
      escapeValue: false,
    },
    react: {
      useSuspense: false,
    },
  });

export default i18n;
