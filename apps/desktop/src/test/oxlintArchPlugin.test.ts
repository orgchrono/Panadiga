import { describe, expect, it } from 'vitest';
import archPlugin from '@orgchrono/oxlint-plugin';

interface ReportedViolation {
  readonly message: string;
}

interface HarnessContext {
  readonly physicalFilename: string;
  readonly filename: string;
  readonly sourceCode: {
    readonly getAllComments: () => readonly { readonly value: string }[];
  };
  readonly report: (entry: { readonly message: string }) => void;
}

function createRuleHarness<V extends object>(
  rule: { readonly create: (ctx: HarnessContext) => V },
  physicalFilename: string,
  comments: readonly { readonly value: string }[] = [],
) {
  const reports: ReportedViolation[] = [];
  const visitors = rule.create({
    physicalFilename,
    filename: physicalFilename,
    sourceCode: {
      getAllComments: () => comments,
    },
    report: ({ message }: { readonly message: string }) => {
      reports.push({ message });
    },
  });
  return { visitors, reports };
}

describe('oxlint-panadiga-arch custom architectural plugin', () => {
  it('blocks unsafe type assertions and chained unknown casts while permitting as const', () => {
    const { visitors, reports } = createRuleHarness(
      archPlugin.rules['no-unsafe-type-cast'],
      'C:/repo/apps/desktop/src/viewmodels/useFoo.ts',
    );
    visitors.TSAsExpression?.({
      type: 'TSAsExpression',
      typeAnnotation: { type: 'TSUnknownKeyword' },
      expression: { type: 'Identifier', name: 'payload' },
    });
    visitors.TSAsExpression?.({
      type: 'TSAsExpression',
      typeAnnotation: {
        type: 'TSTypeReference',
        typeName: { type: 'Identifier', name: 'const' },
      },
      expression: { type: 'Identifier', name: 'MODES' },
    });

    expect(reports).toHaveLength(1);
    expect(reports[0].message).toContain('as unknown');
  });

  it('forbids let bindings and mutating array methods in FP layers', () => {
    const { visitors, reports } = createRuleHarness(
      archPlugin.rules['no-let-or-mutation'],
      'C:/repo/apps/desktop/src/components/Foo.tsx',
    );
    visitors.VariableDeclaration?.({ kind: 'let' });
    visitors.CallExpression?.({
      callee: {
        type: 'MemberExpression',
        computed: false,
        property: { type: 'Identifier', name: 'push' },
      },
    });

    expect(reports).toHaveLength(2);
  });

  it('permits let inside closed non-UI create* factory closures while blocking UI/top-level let', () => {
    const utilCheck = createRuleHarness(
      archPlugin.rules['no-let-or-mutation'],
      'C:/repo/apps/desktop/src/utils/counter.ts',
    );
    const letDecl = {
      type: 'VariableDeclaration',
      kind: 'let',
      declarations: [{ type: 'VariableDeclarator', id: { type: 'Identifier', name: 'count' } }],
    };
    const factoryFn = {
      type: 'FunctionDeclaration',
      id: { type: 'Identifier', name: 'createCounter' },
      params: [],
      body: { type: 'BlockStatement', body: [letDecl] },
    };
    const innerCallback = {
      type: 'ArrowFunctionExpression',
      params: [],
      parent: factoryFn,
    };
    const assignExpr = {
      type: 'AssignmentExpression',
      operator: '=',
      left: { type: 'Identifier', name: 'count' },
      parent: innerCallback,
    };
    const letDeclWithParent = { ...letDecl, parent: factoryFn };

    utilCheck.visitors.VariableDeclaration?.(letDeclWithParent);
    utilCheck.visitors.AssignmentExpression?.(assignExpr);
    expect(utilCheck.reports).toHaveLength(0);

    const componentCheck = createRuleHarness(
      archPlugin.rules['no-let-or-mutation'],
      'C:/repo/apps/desktop/src/components/BadWidget.tsx',
    );
    componentCheck.visitors.VariableDeclaration?.({ ...letDecl, parent: factoryFn });
    expect(componentCheck.reports).toHaveLength(1);
  });

  it('enforces strict MVVM unidirectional import DAG', () => {
    const componentCheck = createRuleHarness(
      archPlugin.rules['mvvm-strict-boundaries'],
      'C:/repo/apps/desktop/src/components/hdd/SampleCard.tsx',
    );
    componentCheck.visitors.ImportDeclaration?.({
      importKind: 'value',
      source: { value: '@/services/picoService' },
    });
    expect(componentCheck.reports).toHaveLength(1);
    expect(componentCheck.reports[0].message).toContain('MVVM boundary violation');

    const vmCheck = createRuleHarness(
      archPlugin.rules['mvvm-strict-boundaries'],
      'C:/repo/apps/desktop/src/viewmodels/useSampleVm.ts',
    );
    vmCheck.visitors.ImportDeclaration?.({
      importKind: 'value',
      source: { value: '@/services/picoService' },
    });
    expect(vmCheck.reports).toHaveLength(0);
  });

  it('flags hardcoded throw messages and fallback strings in translation calls', () => {
    const { visitors, reports } = createRuleHarness(
      archPlugin.rules['zero-hardcode'],
      'C:/repo/apps/desktop/src/viewmodels/nvram/nvramHelpers.ts',
    );
    visitors.ThrowStatement?.({
      argument: {
        type: 'NewExpression',
        arguments: [{ type: 'Literal', value: 'Hardcoded throw error text' }],
      },
    });
    visitors.CallExpression?.({
      callee: { type: 'Identifier', name: 't' },
      arguments: [
        { type: 'Literal', value: 'errors.demo' },
        { type: 'Literal', value: 'Inline fallback string' },
      ],
    });

    expect(reports).toHaveLength(2);
  });

  it('blocks browser globals outside adapters and rejects suppression/custom .d.ts files', () => {
    const globalCheck = createRuleHarness(
      archPlugin.rules['no-global-state-or-side-effects'],
      'C:/repo/apps/desktop/src/components/Foo.tsx',
    );
    globalCheck.visitors.Identifier?.({ name: 'window', parent: { type: 'MemberExpression', object: null } });
    expect(globalCheck.reports).toHaveLength(1);

    const disableToken = ['eslint', 'disable', 'next-line'].join('-');
    const suppressionCheck = createRuleHarness(
      archPlugin.rules['no-lint-suppression'],
      'C:/repo/apps/desktop/src/types/legacy.d.ts',
      [{ value: ` ${disableToken} no-explicit-any` }],
    );
    suppressionCheck.visitors.Program?.({ type: 'Program' });
    expect(suppressionCheck.reports).toHaveLength(2);
  });

  it('enforces explicit button type and CSS-variable-only inline styles in UI layers', () => {
    const { visitors, reports } = createRuleHarness(
      archPlugin.rules['ux-workbench-integrity'],
      'C:/repo/apps/desktop/src/components/SamplePanel.tsx',
    );

    visitors.JSXOpeningElement?.({
      type: 'JSXOpeningElement',
      name: { type: 'JSXIdentifier', name: 'button' },
      attributes: [],
    });
    visitors.JSXOpeningElement?.({
      type: 'JSXOpeningElement',
      name: { type: 'JSXIdentifier', name: 'button' },
      attributes: [
        {
          type: 'JSXAttribute',
          name: { type: 'JSXIdentifier', name: 'type' },
          value: { type: 'Literal', value: 'button' },
        },
      ],
    });
    const makeDivStyleNode = (key: Record<string, unknown>, value: string) => ({
      type: 'JSXOpeningElement',
      name: { type: 'JSXIdentifier', name: 'div' },
      attributes: [
        {
          type: 'JSXAttribute',
          name: { type: 'JSXIdentifier', name: 'style' },
          value: {
            type: 'JSXExpressionContainer',
            expression: {
              type: 'ObjectExpression',
              properties: [{ type: 'Property', computed: false, key, value: { type: 'Literal', value } }],
            },
          },
        },
      ],
    });
    visitors.JSXOpeningElement?.(makeDivStyleNode({ type: 'Identifier', name: 'color' }, 'red'));
    visitors.JSXOpeningElement?.(makeDivStyleNode({ type: 'Literal', value: '--split-left' }, '40%'));

    expect(reports).toHaveLength(2);
    expect(reports[0].message).toContain('<button>');
    expect(reports[1].message).toContain('CSS custom properties');
  });
});
