import path from 'node:path';
import { KNOWN_LAYERS } from './constants.mjs';

const LOOP_NODE_TYPES = new Set([
  'ForStatement',
  'ForInStatement',
  'ForOfStatement',
  'WhileStatement',
  'DoWhileStatement',
]);

const FUNCTION_NODE_TYPES = new Set([
  'FunctionDeclaration',
  'FunctionExpression',
  'ArrowFunctionExpression',
]);

export function getFileInfo(context) {
  const raw = String(
    context.physicalFilename || context.filename || context.getFilename?.() || '',
  ).replace(/\\/g, '/');
  const marker = '/src/';
  const idx = raw.lastIndexOf(marker);
  if (idx === -1) {
    return { inSrc: false, rel: raw, layer: 'external', isTest: false };
  }
  const rel = raw.slice(idx + marker.length);
  const firstSegment = rel.includes('/') ? rel.split('/')[0] : 'root';
  const layer = KNOWN_LAYERS.has(firstSegment) ? firstSegment : 'root';
  const isTest = rel.startsWith('test/') || /\.test\.(ts|tsx)$/.test(rel);
  return { inSrc: true, rel, layer, isTest };
}

export function resolveInternalLayer(currentRel, specifier) {
  if (specifier.startsWith('@/')) {
    const sub = specifier.slice(2);
    const seg = sub.includes('/') ? sub.split('/')[0] : sub;
    const layer = KNOWN_LAYERS.has(seg) ? seg : 'root';
    return { internal: true, externalRepoAsset: false, targetPath: sub, targetLayer: layer };
  }
  if (specifier.startsWith('.')) {
    const dir = path.posix.dirname(currentRel);
    const joined = path.posix.normalize(path.posix.join(dir, specifier));
    if (joined.startsWith('..')) {
      return { internal: false, externalRepoAsset: true, targetPath: joined, targetLayer: null };
    }
    const seg = joined.includes('/') ? joined.split('/')[0] : joined;
    const layer = KNOWN_LAYERS.has(seg) ? seg : 'root';
    return { internal: true, externalRepoAsset: false, targetPath: joined, targetLayer: layer };
  }
  return { internal: false, externalRepoAsset: false, targetPath: specifier, targetLayer: null };
}

export function isLocalShadowed(context, node) {
  try {
    let scope = context.sourceCode?.getScope?.(node) ?? null;
    while (scope && scope.type !== 'global') {
      if (scope.variables?.some(v => v.name === node.name && v.defs?.length > 0)) {
        return true;
      }
      scope = scope.upper ?? null;
    }
  } catch {
    return false;
  }
  return false;
}

export function getCalleeName(callee) {
  if (!callee) return null;
  if (callee.type === 'Identifier') return callee.name;
  if (callee.type === 'MemberExpression' && !callee.computed && callee.property?.type === 'Identifier') {
    return callee.property.name;
  }
  return null;
}

export function getRootObjectName(expr) {
  if (!expr) return null;
  if (expr.type === 'Identifier') return expr.name;
  if (expr.type === 'MemberExpression') return getRootObjectName(expr.object);
  return null;
}

export function isPropertyOrTypeIdentifier(node) {
  const parent = node.parent;
  if (!parent) return false;
  if (
    (parent.type === 'MemberExpression' || parent.type === 'StaticMemberExpression') &&
    parent.property === node &&
    !parent.computed
  ) {
    return true;
  }
  if (
    (parent.type === 'Property' || parent.type === 'ObjectProperty' || parent.type === 'BindingProperty') &&
    parent.key === node &&
    !parent.computed
  ) {
    return true;
  }
  if (
    parent.type === 'ImportSpecifier' ||
    parent.type === 'ImportDefaultSpecifier' ||
    parent.type === 'ImportNamespaceSpecifier' ||
    parent.type === 'ExportSpecifier' ||
    parent.type === 'TSPropertySignature' ||
    parent.type === 'TSMethodSignature' ||
    parent.type === 'TSTypeReference' ||
    parent.type === 'TSTypeQuery' ||
    parent.type === 'TSInterfaceDeclaration' ||
    parent.type === 'TSTypeAliasDeclaration' ||
    (parent.type === 'VariableDeclarator' && parent.id === node)
  ) {
    return true;
  }
  if (
    (parent.type === 'FunctionDeclaration' ||
      parent.type === 'FunctionExpression' ||
      parent.type === 'ArrowFunctionExpression') &&
    (parent.id === node || parent.params?.includes(node))
  ) {
    return true;
  }
  return false;
}

function getFactoryClosureName(fnNode) {
  if (!fnNode) return null;
  if (fnNode.type === 'FunctionDeclaration' && fnNode.id?.type === 'Identifier') {
    return fnNode.id.name;
  }
  if (
    (fnNode.type === 'FunctionExpression' || fnNode.type === 'ArrowFunctionExpression') &&
    fnNode.parent?.type === 'VariableDeclarator' &&
    fnNode.parent.id?.type === 'Identifier'
  ) {
    return fnNode.parent.id.name;
  }
  return null;
}

function isClosedFactoryFunction(fnNode, info) {
  if (!info || info.rel.endsWith('.tsx')) return false;
  if (info.layer === 'components' || info.layer === 'views' || info.layer === 'viewmodels') {
    return false;
  }
  const name = getFactoryClosureName(fnNode);
  return typeof name === 'string' && /^create[A-Z]/.test(name);
}

function findImmediateEnclosingFunction(node) {
  let cur = node?.parent ?? null;
  while (cur) {
    if (LOOP_NODE_TYPES.has(cur.type)) {
      return { loopFound: true, fnNode: null };
    }
    if (FUNCTION_NODE_TYPES.has(cur.type)) {
      return { loopFound: false, fnNode: cur };
    }
    cur = cur.parent ?? null;
  }
  return { loopFound: false, fnNode: null };
}

export function isAllowedFactoryLetDeclaration(node, info) {
  if (node?.kind !== 'let') return false;
  const { loopFound, fnNode } = findImmediateEnclosingFunction(node);
  if (loopFound || !fnNode) return false;
  return isClosedFactoryFunction(fnNode, info);
}

function blockContainsLetBinding(fnNode, varName) {
  const stmts = fnNode?.body?.type === 'BlockStatement' ? fnNode.body.body : null;
  if (!Array.isArray(stmts)) return false;
  return stmts.some(
    stmt =>
      stmt.type === 'VariableDeclaration' &&
      stmt.kind === 'let' &&
      (stmt.declarations ?? []).some(
        decl => decl.id?.type === 'Identifier' && decl.id.name === varName,
      ),
  );
}

export function isAllowedFactoryLetAssignment(node, info) {
  if (node?.operator !== '=' || node.left?.type !== 'Identifier') return false;
  const targetName = node.left.name;
  let cur = node.parent ?? null;
  while (cur) {
    if (LOOP_NODE_TYPES.has(cur.type)) return false;
    if (FUNCTION_NODE_TYPES.has(cur.type)) {
      if ((cur.params ?? []).some(p => p.type === 'Identifier' && p.name === targetName)) {
        return false;
      }
      if (blockContainsLetBinding(cur, targetName)) {
        return isClosedFactoryFunction(cur, info);
      }
    }
    cur = cur.parent ?? null;
  }
  return false;
}

export function isAllowedErrorTranslationCall(node) {
  if (node?.type !== 'CallExpression') return false;
  const name = getCalleeName(node.callee);
  return name === 'translate' || name === 't' || name === 'String';
}

export function findDirectLiteralInErrorExpr(expr) {
  if (!expr) return null;
  if (expr.type === 'Literal' && typeof expr.value === 'string' && expr.value.trim().length > 0) {
    return expr;
  }
  if (expr.type === 'TemplateLiteral') {
    return expr;
  }
  if (isAllowedErrorTranslationCall(expr)) {
    return null;
  }
  if (expr.type === 'NewExpression') {
    for (const arg of expr.arguments ?? []) {
      const hit = findDirectLiteralInErrorExpr(arg);
      if (hit) return hit;
    }
    return null;
  }
  if (expr.type === 'BinaryExpression' || expr.type === 'LogicalExpression') {
    return findDirectLiteralInErrorExpr(expr.left) || findDirectLiteralInErrorExpr(expr.right);
  }
  if (expr.type === 'ConditionalExpression') {
    return (
      findDirectLiteralInErrorExpr(expr.consequent) ||
      findDirectLiteralInErrorExpr(expr.alternate)
    );
  }
  return null;
}

export function looksLikeProseLiteral(text) {
  return /[a-zA-ZÀ-ÿ]{2,}\s+[a-zA-ZÀ-ÿ]{2,}/.test(text);
}

export function inspectProseInArg(arg) {
  if (!arg) return null;
  if (isAllowedErrorTranslationCall(arg)) return null;
  if (arg.type === 'Literal' && typeof arg.value === 'string' && looksLikeProseLiteral(arg.value)) {
    return arg;
  }
  if (arg.type === 'TemplateLiteral') {
    const rawText = (arg.quasis ?? []).map(q => q.value?.raw ?? '').join(' ');
    if (looksLikeProseLiteral(rawText)) return arg;
  }
  if (arg.type === 'ObjectExpression') {
    for (const prop of arg.properties ?? []) {
      if (
        (prop.type === 'Property' || prop.type === 'ObjectProperty') &&
        prop.key?.type === 'Identifier' &&
        (prop.key.name === 'title' ||
          prop.key.name === 'message' ||
          prop.key.name === 'description' ||
          prop.key.name === 'error' ||
          prop.key.name === 'output' ||
          prop.key.name === 'errorTitle')
      ) {
        const hit = inspectProseInArg(prop.value);
        if (hit) return hit;
      }
    }
  }
  return null;
}

export function hasAsConstAnnotation(node) {
  const ann = node.typeAnnotation;
  return (
    ann?.type === 'TSTypeReference' &&
    ann.typeName?.type === 'Identifier' &&
    ann.typeName.name === 'const'
  );
}
