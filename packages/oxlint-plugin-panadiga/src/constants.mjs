export const MUTATING_ARRAY_METHODS = new Set([
  'push',
  'pop',
  'shift',
  'unshift',
  'splice',
  'sort',
  'reverse',
  'fill',
  'copyWithin',
]);

export const COLLECTION_MUTATION_METHODS = new Set(['add', 'clear', 'delete', 'set']);

export const BROWSER_GLOBALS = new Set([
  'window',
  'globalThis',
  'document',
  'localStorage',
  'sessionStorage',
  'navigator',
  'location',
  'history',
]);

export const BROWSER_ADAPTER_ALLOWLIST = new Set([
  'store/storage.ts',
  'config/platform.ts',
  'hooks/usePointerDrag.ts',
  'main.tsx',
]);

export const MODULE_BOOT_ALLOWLIST = new Set(['main.tsx', 'i18n/i18n.ts']);

export const TIMER_REF_ALLOWLIST = new Set([
  'hooks/useManagedTimeouts.ts',
  'hooks/useAbortSlot.ts',
  'context/ToastContext.tsx',
]);

export const IPC_FUNCTIONS = new Set([
  'invokeCommand',
  'emitLocalEvent',
  'listenEvent',
  'invoke',
  'listen',
]);

export const USER_FACING_CALLS = new Set([
  'showToast',
  'enqueueToast',
  'addLog',
  'onLog',
  'setError',
  'setPackError',
  'setUnpackError',
  'setFormError',
]);

export const HARDWARE_MAGIC_NUMBERS = new Set([
  115200,
  3333,
  512,
  2048,
  8192,
  16384,
  65536,
  262144,
  8388608,
  0x80000000,
  0x807f0000,
  0x807fffff,
  0x7f0000,
  0x7fffff,
  0x800000,
  0x84000000,
  0x44000000,
  0x40000000,
]);

export const HARDWARE_HEX_RAWS = new Set(['0x2e8a', '0x0003']);

export const HARDWARE_WIRE_STRINGS = new Set(['127.0.0.1:3333', 'DUMP 0x', 'PID:0003']);

export const JSX_PROSE_ATTRS = new Set([
  'title',
  'subtitle',
  'description',
  'placeholder',
  'label',
  'badgeLabel',
  'aria-label',
  'ariaLabel',
  'emptyMessage',
]);

export const LAYER_ALLOW_MAP = Object.freeze({
  models: new Set(['models', 'types', 'constants']),
  types: new Set(['models', 'types', 'constants']),
  constants: new Set(['models', 'types', 'constants', 'locales']),
  locales: new Set(['locales']),
  utils: new Set(['models', 'types', 'constants', 'utils', 'lib']),
  lib: new Set(['models', 'types', 'constants', 'utils', 'lib']),
  config: new Set(['models', 'types', 'constants', 'utils', 'config']),
  i18n: new Set(['models', 'types', 'constants', 'locales', 'utils', 'i18n']),
  services: new Set(['models', 'types', 'constants', 'config', 'utils', 'lib', 'i18n', 'services']),
  store: new Set(['models', 'types', 'constants', 'config', 'utils', 'i18n', 'services', 'store']),
  hooks: new Set(['models', 'types', 'constants', 'config', 'utils', 'i18n', 'hooks']),
  context: new Set(['models', 'types', 'constants', 'config', 'utils', 'i18n', 'hooks', 'services', 'store', 'context']),
  viewmodels: new Set([
    'models',
    'types',
    'constants',
    'config',
    'utils',
    'i18n',
    'hooks',
    'services',
    'store',
    'context',
    'viewmodels',
  ]),
  components: new Set([
    'models',
    'types',
    'constants',
    'config',
    'utils',
    'lib',
    'i18n',
    'hooks',
    'components',
  ]),
  views: new Set([
    'models',
    'types',
    'constants',
    'config',
    'utils',
    'lib',
    'i18n',
    'hooks',
    'context',
    'viewmodels',
    'components',
    'views',
  ]),
  root: new Set([
    'models',
    'types',
    'constants',
    'config',
    'utils',
    'lib',
    'i18n',
    'hooks',
    'context',
    'store',
    'viewmodels',
    'components',
    'views',
    'root',
  ]),
});

export const KNOWN_LAYERS = new Set(Object.keys(LAYER_ALLOW_MAP));
