import {
  BROWSER_ADAPTER_ALLOWLIST,
  BROWSER_GLOBALS,
  COLLECTION_MUTATION_METHODS,
  HARDWARE_HEX_RAWS,
  HARDWARE_MAGIC_NUMBERS,
  HARDWARE_WIRE_STRINGS,
  IPC_FUNCTIONS,
  JSX_PROSE_ATTRS,
  LAYER_ALLOW_MAP,
  MODULE_BOOT_ALLOWLIST,
  MUTATING_ARRAY_METHODS,
  TIMER_REF_ALLOWLIST,
  USER_FACING_CALLS,
} from './constants.mjs';
import {
  findDirectLiteralInErrorExpr,
  getCalleeName,
  getFileInfo,
  getRootObjectName,
  hasAsConstAnnotation,
  inspectProseInArg,
  isAllowedFactoryLetAssignment,
  isAllowedFactoryLetDeclaration,
  isLocalShadowed,
  isPropertyOrTypeIdentifier,
  looksLikeProseLiteral,
  resolveInternalLayer,
} from './astUtils.mjs';

const noUnsafeTypeCast = {
  meta: {
    type: 'problem',
    docs: {
      description: 'Disallow any, as unknown, double type assertions, and unsafe narrowing casts.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc) return {};
    return {
      TSAnyKeyword(node) {
        context.report({
          node,
          message:
            "Explicit 'any' type is forbidden; model domain contracts explicitly or validate 'unknown' with type guards.",
        });
      },
      TSTypeAssertion(node) {
        context.report({
          node,
          message:
            "Legacy angle-bracket type assertion '<T>expr' is forbidden; use runtime type guards or 'satisfies'.",
        });
      },
      TSAsExpression(node) {
        if (hasAsConstAnnotation(node)) {
          return;
        }
        const annType = node.typeAnnotation?.type;
        if (annType === 'TSAnyKeyword') {
          context.report({
            node,
            message: "Unsafe cast 'as any' is forbidden.",
          });
          return;
        }
        if (annType === 'TSUnknownKeyword' || node.expression?.type === 'TSAsExpression') {
          context.report({
            node,
            message:
              "Chained or widening cast ('as unknown' / 'as unknown as T') is forbidden; narrow with runtime type guards.",
          });
          return;
        }
        if (!info.isTest) {
          context.report({
            node,
            message:
              "Type assertion 'as Type' is forbidden in production; use 'satisfies', 'oneOfGuard', 'safeParseJson', or a type predicate.",
          });
        }
      },
    };
  },
};

const noLetOrMutation = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Enforce immutable FP style: allow let only inside closed non-UI create* factory closures; ban loops, mutations, ++/--, delete, and OOP classes.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc || info.isTest) return {};
    const allowCollectionMutations =
      info.layer === 'services' || TIMER_REF_ALLOWLIST.has(info.rel);

    const reportLoop = node =>
      context.report({
        node,
        message: `Imperative '${node.type}' loop is forbidden; prefer .map, .filter, .reduce, .some, .every, or recursion.`,
      });

    return {
      VariableDeclaration(node) {
        if (node.kind === 'var') {
          context.report({
            node,
            message: "Mutable 'var' declaration is forbidden; use immutable 'const' expressions.",
          });
          return;
        }
        if (node.kind === 'let' && !isAllowedFactoryLetDeclaration(node, info)) {
          context.report({
            node,
            message:
              "Mutable 'let' is forbidden outside closed non-UI create* closure factories; prefer immutable 'const' expressions.",
          });
        }
      },
      ForStatement: reportLoop,
      ForInStatement: reportLoop,
      ForOfStatement: reportLoop,
      WhileStatement: reportLoop,
      DoWhileStatement: reportLoop,
      UpdateExpression(node) {
        context.report({
          node,
          message: `In-place update operator '${node.operator}' is forbidden.`,
        });
      },
      AssignmentExpression(node) {
        if (isAllowedFactoryLetAssignment(node, info)) {
          return;
        }
        context.report({
          node,
          message: `Reassignment/mutation via '${node.operator}' is forbidden; derive a new immutable value instead.`,
        });
      },
      UnaryExpression(node) {
        if (node.operator === 'delete') {
          context.report({
            node,
            message: "Mutating 'delete' operator is forbidden; construct a new object without the key.",
          });
        }
      },
      ClassDeclaration(node) {
        context.report({
          node,
          message: 'OOP class declarations are forbidden; use pure functions, hooks, and immutable closures.',
        });
      },
      ClassExpression(node) {
        context.report({
          node,
          message: 'OOP class expressions are forbidden; use pure functions and closures.',
        });
      },
      CallExpression(node) {
        const callee = node.callee;
        if (!callee || callee.type !== 'MemberExpression' || callee.computed) {
          return;
        }
        const propName = callee.property?.type === 'Identifier' ? callee.property.name : null;
        if (!propName) return;

        if (MUTATING_ARRAY_METHODS.has(propName)) {
          context.report({
            node,
            message: `Mutating array method '.${propName}()' is forbidden; use immutable spread/slice/map/filter helpers.`,
          });
          return;
        }

        if (
          callee.object?.type === 'Identifier' &&
          callee.object.name === 'Object' &&
          propName === 'assign' &&
          node.arguments[0]?.type !== 'ObjectExpression'
        ) {
          context.report({
            node,
            message: "Mutating Object.assign(target, ...) is forbidden; use object spread '{ ...a, ...b }'.",
          });
          return;
        }

        if (!allowCollectionMutations && COLLECTION_MUTATION_METHODS.has(propName)) {
          context.report({
            node,
            message: `In-place collection mutation '.${propName}()' is forbidden outside transport services.`,
          });
        }
      },
    };
  },
};

const noGlobalStateOrSideEffects = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Forbid declare global, module-level side effects, global mutable state, and leaky DOM/window access.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc || info.isTest) return {};
    const allowBrowserRead =
      info.layer === 'services' || BROWSER_ADAPTER_ALLOWLIST.has(info.rel);
    const allowTopLevelBoot = MODULE_BOOT_ALLOWLIST.has(info.rel);

    return {
      TSModuleDeclaration(node) {
        context.report({
          node,
          message:
            'Ambient global/module declarations (`declare global` / `declare module`) are forbidden; use typed bridge accessors.',
        });
      },
      Program(program) {
        if (!allowTopLevelBoot) {
          for (const stmt of program.body ?? []) {
            if (stmt.type === 'ExpressionStatement' && !stmt.directive) {
              context.report({
                node: stmt,
                message: 'Top-level side-effect statement is forbidden at module scope.',
              });
            }
          }
        }
      },
      NewExpression(node) {
        if (
          node.callee?.type === 'Identifier' &&
          node.callee.name === 'Map' &&
          info.layer !== 'services' &&
          node.parent?.type === 'VariableDeclarator' &&
          node.parent.parent?.parent?.type === 'Program'
        ) {
          context.report({
            node,
            message: 'Top-level mutable Map state is forbidden outside services/.',
          });
        }
      },
      CallExpression(node) {
        if (
          node.callee?.type === 'MemberExpression' &&
          node.callee.object?.type === 'Identifier' &&
          node.callee.object.name === 'Reflect' &&
          node.callee.property?.type === 'Identifier' &&
          (node.callee.property.name === 'set' || node.callee.property.name === 'deleteProperty')
        ) {
          const targetName = node.arguments[0]?.type === 'Identifier' ? node.arguments[0].name : null;
          if (targetName && BROWSER_GLOBALS.has(targetName)) {
            context.report({
              node,
              message: `Mutating global '${targetName}' in production is forbidden.`,
            });
          }
        }
      },
      Identifier(node) {
        if (!BROWSER_GLOBALS.has(node.name)) return;
        if (isPropertyOrTypeIdentifier(node) || isLocalShadowed(context, node)) return;
        if (node.parent?.type === 'AssignmentExpression' && getRootObjectName(node.parent.left) === node.name) {
          context.report({
            node,
            message: `Assigning to global '${node.name}' is forbidden.`,
          });
          return;
        }
        if (!allowBrowserRead) {
          context.report({
            node,
            message: `Direct browser global '${node.name}' access is forbidden outside services/store storage/platform/pointer hook adapters.`,
          });
        }
      },
    };
  },
};

const mvvmStrictBoundaries = {
  meta: {
    type: 'problem',
    docs: {
      description: 'Enforce strict directed MVVM & Clean Architecture import boundaries.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc || info.isTest) return {};
    const allowedLayers = LAYER_ALLOW_MAP[info.layer] ?? LAYER_ALLOW_MAP.root;

    const validateSpecifier = (reportNode, specifier) => {
      if (!specifier) return;
      if (specifier.startsWith('@tauri-apps/')) {
        if (info.layer !== 'services') {
          context.report({
            node: reportNode,
            message: `MVVM violation: '@tauri-apps/*' imports are allowed ONLY in src/services/ (found in '${info.layer}').`,
          });
        }
        return;
      }
      if (
        specifier === 'react' ||
        specifier.startsWith('react/') ||
        specifier === 'react-dom' ||
        specifier.startsWith('react-dom/')
      ) {
        if (
          info.layer === 'models' ||
          info.layer === 'types' ||
          info.layer === 'constants' ||
          info.layer === 'utils' ||
          info.layer === 'lib' ||
          info.layer === 'config' ||
          info.layer === 'services' ||
          (info.layer === 'store' && info.rel !== 'store/hooks.ts')
        ) {
          context.report({
            node: reportNode,
            message: `MVVM violation: '${info.layer}' must stay UI-framework-free (cannot import '${specifier}').`,
          });
        }
        return;
      }
      if (specifier === 'react-redux' || specifier.startsWith('@reduxjs/')) {
        if (info.layer !== 'store' && info.layer !== 'root') {
          context.report({
            node: reportNode,
            message: `MVVM violation: direct Redux package '${specifier}' is encapsulated in src/store/ (use @/store/hooks instead).`,
          });
        }
        return;
      }

      const resolved = resolveInternalLayer(info.rel, specifier);
      if (resolved.externalRepoAsset) {
        if (info.layer !== 'constants' || !specifier.endsWith('.json')) {
          context.report({
            node: reportNode,
            message: `Import escaping src/ ('${specifier}') is restricted to JSON SSOT catalogs in src/constants/.`,
          });
        }
        return;
      }
      if (!resolved.internal || !resolved.targetLayer) return;

      if (resolved.targetPath === 'services/ipc' && info.layer !== 'services') {
        context.report({
          node: reportNode,
          message:
            "MVVM violation: direct '@/services/ipc' access is restricted to src/services/; import a domain service wrapper instead.",
        });
        return;
      }

      if (!allowedLayers.has(resolved.targetLayer)) {
        context.report({
          node: reportNode,
          message: `MVVM boundary violation: layer '${info.layer}' cannot import from layer '${resolved.targetLayer}' ('${specifier}').`,
        });
      }
    };

    return {
      ImportDeclaration(node) {
        validateSpecifier(node, node.source?.value);
      },
      ExportNamedDeclaration(node) {
        if (node.source) validateSpecifier(node, node.source.value);
      },
      ExportAllDeclaration(node) {
        validateSpecifier(node, node.source?.value);
      },
      ImportExpression(node) {
        if (node.source?.type === 'Literal' && typeof node.source.value === 'string') {
          validateSpecifier(node, node.source.value);
        }
      },
    };
  },
};

const zeroHardcode = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Ban hardcoded error strings in throw, toast/log prose in logic, hardware magic values, raw IPC, console.*, and t() fallbacks.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc || info.isTest) return {};
    const allowHardwareLiterals = info.layer === 'constants' || info.layer === 'models';
    const allowInternalSchemaThrows = info.rel.startsWith('constants/hardwareSpecs/');

    return {
      ThrowStatement(node) {
        if (allowInternalSchemaThrows) return;
        const literalHit = findDirectLiteralInErrorExpr(node.argument);
        if (literalHit) {
          context.report({
            node: literalHit,
            message:
              "Hardcoded string in 'throw' is forbidden; use translate('errors....') from '@/i18n/useTranslation'.",
          });
        }
      },
      CallExpression(node) {
        const calleeName = getCalleeName(node.callee);

        if (
          node.callee?.type === 'MemberExpression' &&
          !node.callee.computed &&
          node.callee.object?.type === 'Identifier' &&
          node.callee.object.name === 'console' &&
          !isLocalShadowed(context, node.callee.object)
        ) {
          context.report({
            node,
            message: "Direct 'console.*' call is forbidden; use structured telemetry via '@/utils/logger'.",
          });
          return;
        }

        if (calleeName && IPC_FUNCTIONS.has(calleeName) && info.layer !== 'services') {
          context.report({
            node,
            message: `Direct IPC transport call '${calleeName}()' is forbidden outside src/services/.`,
          });
          return;
        }

        if (
          (calleeName === 't' || calleeName === 'translate') &&
          node.arguments.length >= 2 &&
          node.arguments[1]?.type === 'Literal' &&
          typeof node.arguments[1].value === 'string'
        ) {
          context.report({
            node: node.arguments[1],
            message:
              'Inline fallback string in translation call is forbidden; define the key under src/locales/<lang>/<domain>.json.',
          });
          return;
        }

        if (
          calleeName === 'Error' &&
          node.parent?.type !== 'ThrowStatement' &&
          !allowInternalSchemaThrows
        ) {
          const hit = findDirectLiteralInErrorExpr(node);
          if (hit) {
            context.report({
              node: hit,
              message: "Hardcoded Error(...) literal is forbidden; use translate('errors....').",
            });
          }
        }

        if (
          calleeName &&
          USER_FACING_CALLS.has(calleeName) &&
          (info.layer === 'services' ||
            info.layer === 'viewmodels' ||
            info.layer === 'store' ||
            info.layer === 'context')
        ) {
          for (const arg of node.arguments ?? []) {
            const hit = inspectProseInArg(arg);
            if (hit) {
              context.report({
                node: hit,
                message: `Hardcoded user-facing text in '${calleeName}()' is forbidden; use i18n translate()/t().`,
              });
            }
          }
        }
      },
      NewExpression(node) {
        if (allowInternalSchemaThrows) return;
        if (node.callee?.type === 'Identifier' && node.callee.name === 'Error') {
          const hit = findDirectLiteralInErrorExpr(node);
          if (hit) {
            context.report({
              node: hit,
              message:
                "Hardcoded string in 'new Error()' is forbidden; use translate('errors....') from '@/i18n/useTranslation'.",
            });
          }
        }
      },
      ReturnStatement(node) {
        if (info.layer !== 'services' && info.layer !== 'viewmodels') return;
        if (node.argument?.type === 'ObjectExpression') {
          const hit = inspectProseInArg(node.argument);
          if (hit) {
            context.report({
              node: hit,
              message:
                'Hardcoded user-facing message/error string in service or viewmodel return value is forbidden; use translate().',
            });
          }
        }
      },
      Literal(node) {
        if (allowHardwareLiterals || info.layer === 'locales') return;
        const rawLower = typeof node.raw === 'string' ? node.raw.toLowerCase() : '';
        if (
          typeof node.value === 'number' &&
          (HARDWARE_MAGIC_NUMBERS.has(node.value) || HARDWARE_HEX_RAWS.has(rawLower))
        ) {
          context.report({
            node,
            message: `Hardcoded hardware constant (${node.raw ?? node.value}) is forbidden outside src/constants/; import the canonical SSOT constant.`,
          });
          return;
        }
        if (typeof node.value === 'string' && HARDWARE_WIRE_STRINGS.has(node.value)) {
          context.report({
            node,
            message: `Hardcoded protocol/address constant '${node.value}' is forbidden outside src/constants/.`,
          });
        }
      },
      JSXText(node) {
        if (info.layer !== 'views' && info.layer !== 'components' && info.layer !== 'root') return;
        const trimmed = String(node.value ?? '').replace(/\s+/g, ' ').trim();
        if (looksLikeProseLiteral(trimmed) && trimmed !== 'DIGA SUITE') {
          context.report({
            node,
            message: `Hardcoded JSX text '${trimmed.slice(0, 48)}' is forbidden; move user-facing text to src/locales/<lang>/<domain>.json via t().`,
          });
        }
      },
      JSXAttribute(node) {
        if (info.layer !== 'views' && info.layer !== 'components' && info.layer !== 'root') return;
        const attrName = node.name?.type === 'JSXIdentifier' ? node.name.name : null;
        if (!attrName || !JSX_PROSE_ATTRS.has(attrName)) return;
        if (
          node.value?.type === 'Literal' &&
          typeof node.value.value === 'string' &&
          looksLikeProseLiteral(node.value.value)
        ) {
          context.report({
            node: node.value,
            message: `Hardcoded JSX prop '${attrName}="${node.value.value.slice(0, 40)}"' is forbidden; use t('...').`,
          });
        }
      },
    };
  },
};

const noLintSuppression = {
  meta: {
    type: 'problem',
    docs: {
      description: 'Ban inline linter/TypeScript suppression comments and custom ambient .d.ts declarations.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (!info.inSrc) return {};
    return {
      Program(node) {
        if (info.rel.endsWith('.d.ts') && info.rel !== 'vite-env.d.ts') {
          context.report({
            node,
            message: `Ambient declaration file '${info.rel}' breaks modern ES module typing; remove .d.ts and use explicit module exports + type guards.`,
          });
        }
        const comments = context.sourceCode?.getAllComments?.() ?? [];
        for (const comment of comments) {
          if (
            /\b(eslint-disable|oxlint-disable|@ts-ignore|@ts-expect-error|@ts-nocheck)\b/i.test(
              comment.value,
            )
          ) {
            context.report({
              node: comment,
              message:
                'Inline lint/TypeScript suppression directives are forbidden; resolve the root architectural or type issue cleanly.',
            });
          }
        }
      },
    };
  },
};

const ALLOWED_BUTTON_TYPES = new Set(['button', 'submit', 'reset']);

function getButtonTypeValue(attr) {
  if (!attr?.value) return null;
  if (attr.value.type === 'Literal' && typeof attr.value.value === 'string') {
    return attr.value.value;
  }
  if (
    attr.value.type === 'JSXExpressionContainer' &&
    attr.value.expression?.type === 'Literal' &&
    typeof attr.value.expression.value === 'string'
  ) {
    return attr.value.expression.value;
  }
  return null;
}

const uxWorkbenchIntegrity = {
  meta: {
    type: 'problem',
    docs: {
      description:
        'Enforce explicit <button type="..."> and forbid inline non-CSS-variable style objects in UI layers.',
    },
    schema: [],
  },
  create(context) {
    const info = getFileInfo(context);
    if (
      !info.inSrc ||
      info.isTest ||
      (info.layer !== 'views' && info.layer !== 'components' && info.layer !== 'root')
    ) {
      return {};
    }
    return {
      JSXOpeningElement(node) {
        const attrs = node.attributes ?? [];
        if (node.name?.type === 'JSXIdentifier' && node.name.name === 'button') {
          const typeAttr = attrs.find(
            attr =>
              attr.type === 'JSXAttribute' &&
              attr.name?.type === 'JSXIdentifier' &&
              attr.name.name === 'type',
          );
          const typeVal = getButtonTypeValue(typeAttr);
          if (!typeVal || !ALLOWED_BUTTON_TYPES.has(typeVal)) {
            context.report({
              node: typeAttr ?? node,
              message:
                '<button> must declare an explicit literal type attribute ("button", "submit", or "reset").',
            });
          }
        }

        const styleAttr = attrs.find(
          attr =>
            attr.type === 'JSXAttribute' &&
            attr.name?.type === 'JSXIdentifier' &&
            attr.name.name === 'style',
        );
        if (
          styleAttr?.value?.type === 'JSXExpressionContainer' &&
          styleAttr.value.expression?.type === 'ObjectExpression'
        ) {
          for (const prop of styleAttr.value.expression.properties ?? []) {
            if (prop.type === 'SpreadElement') continue;
            const isCssVarKey =
              (prop.type === 'Property' || prop.type === 'ObjectProperty') &&
              !prop.computed &&
              prop.key?.type === 'Literal' &&
              typeof prop.key.value === 'string' &&
              prop.key.value.startsWith('--');
            if (!isCssVarKey) {
              context.report({
                node: prop,
                message:
                  'Inline style={{ ... }} objects may only set CSS custom properties (--*); use Tailwind CSS utility classes instead.',
              });
            }
          }
        }
      },
    };
  },
};

export default {
  meta: { name: 'panadiga' },
  rules: {
    'no-unsafe-type-cast': noUnsafeTypeCast,
    'no-let-or-mutation': noLetOrMutation,
    'no-global-state-or-side-effects': noGlobalStateOrSideEffects,
    'mvvm-strict-boundaries': mvvmStrictBoundaries,
    'zero-hardcode': zeroHardcode,
    'no-lint-suppression': noLintSuppression,
    'ux-workbench-integrity': uxWorkbenchIntegrity,
  },
};
