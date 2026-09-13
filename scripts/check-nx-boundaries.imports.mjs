import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, extname, isAbsolute, relative, resolve } from 'node:path';
import ts from 'typescript';

const sourceExtensions = new Set(['.cjs', '.js', '.jsx', '.mjs', '.ts', '.tsx']);
const skippedDirectories = new Set(['coverage', 'dist', 'node_modules', 'target']);

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}

function sourceFiles(root, results = []) {
  if (!existsSync(root)) return results;
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    if (skippedDirectories.has(entry.name)) continue;
    const path = resolve(root, entry.name);
    if (entry.isDirectory()) sourceFiles(path, results);
    else if (entry.isFile() && sourceExtensions.has(extname(entry.name))) results.push(path);
  }
  return results;
}

function literalText(node) {
  return ts.isStringLiteralLike(node) ? node.text : undefined;
}

export function sourceImportSpecifiers(source, fileName = 'source.ts') {
  const file = ts.createSourceFile(fileName, source, ts.ScriptTarget.Latest, false);
  const imports = new Set();
  function visit(node) {
    if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier) {
      const specifier = literalText(node.moduleSpecifier);
      if (specifier) imports.add(specifier);
    } else if (
      ts.isImportEqualsDeclaration(node) &&
      ts.isExternalModuleReference(node.moduleReference) &&
      node.moduleReference.expression
    ) {
      const specifier = literalText(node.moduleReference.expression);
      if (specifier) imports.add(specifier);
    } else if (
      ts.isCallExpression(node) &&
      node.arguments.length === 1 &&
      (node.expression.kind === ts.SyntaxKind.ImportKeyword ||
        (ts.isIdentifier(node.expression) && node.expression.text === 'require'))
    ) {
      const specifier = literalText(node.arguments[0]);
      if (specifier) imports.add(specifier);
    }
    ts.forEachChild(node, visit);
  }
  visit(file);
  return [...imports];
}

function owningProject(path, projectsByRoot) {
  return projectsByRoot.find(({ root }) => path === root || path.startsWith(`${root}/`));
}

function packageProject(specifier, packageToProject) {
  for (const [packageName, project] of packageToProject) {
    if (specifier === packageName || specifier.startsWith(`${packageName}/`)) return project;
  }
  return undefined;
}

export function checkSourceImportBoundaries(
  projectMeta,
  packageToProject,
  addErrors,
  errors,
  workspaceRoot = process.cwd(),
) {
  const projectsByRoot = [...projectMeta]
    .map(([name, meta]) => ({ name, root: normalizePath(resolve(workspaceRoot, meta.root)) }))
    .sort((left, right) => right.root.length - left.root.length);

  for (const [source, meta] of projectMeta) {
    const workspaceTool = meta.tags.has('type:workspace');
    for (const file of sourceFiles(resolve(workspaceRoot, meta.root))) {
      for (const specifier of sourceImportSpecifiers(readFileSync(file, 'utf8'), file)) {
        let target;
        if (specifier.startsWith('.')) {
          const importedPath = normalizePath(resolve(dirname(file), specifier));
          const owner = owningProject(importedPath, projectsByRoot);
          target = owner?.name;
          if (target && target !== source && !workspaceTool) {
            errors.push(
              `${normalizePath(relative(workspaceRoot, file))}: cross-project relative import ${specifier} bypasses the ${target} public package boundary`,
            );
          }
        } else if (!isAbsolute(specifier)) {
          target = packageProject(specifier, packageToProject);
        }
        if (target && target !== source) addErrors(source, target, projectMeta, errors);
      }
    }
  }
}
