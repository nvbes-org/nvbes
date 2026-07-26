#!/usr/bin/env node
import { existsSync, lstatSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, normalize } from 'node:path';

const registryPath = 'docs/security/c-toolchain-hardening-controls.json';
const expectedSchemaVersion = 1;
const errors = [];
const skippedDirs = new Set([
  '.git',
  '.nx',
  'node_modules',
  'target',
  'dist',
  'build',
  '.turbo',
  'coverage',
]);
const cFamilyPattern = /\.(c|cc|cpp|cxx|h|hh|hpp|m|mm|s|S)$/u;
const nativeBuildKeywords = [
  'bindgen',
  'pkg_config',
  'pkg-config',
  'cc::Build',
  'cmake::',
  'make ',
  'ninja',
  'CFLAGS',
  'CXXFLAGS',
  'LDFLAGS',
];
const requiredReleaseCflags = [
  '-O2',
  '-g2',
  '-DNDEBUG',
  '-UDEBUG',
  '-Wall',
  '-Wextra',
  '-Wformat=2',
  '-Wformat-security',
  '-Werror=format-security',
  '-D_FORTIFY_SOURCE=3',
  '-fstack-protector-strong',
  '-fPIE',
  '-fno-omit-frame-pointer',
  '-fvisibility=hidden',
];
const requiredLinuxLdflags = ['-pie', '-Wl,-z,relro', '-Wl,-z,now', '-Wl,-z,noexecstack'];

function readJson(path) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return undefined;
  }

  try {
    return JSON.parse(readFileSync(path, 'utf8'));
  } catch (error) {
    errors.push(`${path}: invalid JSON (${error.message})`);
    return undefined;
  }
}

function requireArray(value, path) {
  if (Array.isArray(value)) return value;
  errors.push(`${path}: must be an array`);
  return [];
}

function requireString(value, path) {
  if (typeof value === 'string' && value.trim().length > 0) return value;
  errors.push(`${path}: must be a non-empty string`);
  return '';
}

function assertId(value, path, pattern) {
  const id = requireString(value, path);
  if (id && !pattern.test(id)) {
    errors.push(`${path}: invalid ID format (${id})`);
  }
  return id;
}

function requireUnique(id, seen, path) {
  if (!id) return;
  if (seen.has(id)) {
    errors.push(`${path}: duplicate ID ${id}`);
    return;
  }
  seen.add(id);
}

function requireIncludes(path, includes, context) {
  if (!existsSync(path)) {
    errors.push(`${context}.path: ${path} is missing`);
    return 0;
  }

  const text = readFileSync(path, 'utf8');
  let count = 0;

  for (const include of requireArray(includes, `${context}.includes`)) {
    const needle = requireString(include, `${context}.includes[]`);
    if (!needle) continue;
    count += 1;
    if (!text.includes(needle)) {
      errors.push(`${context}: ${path} does not include ${JSON.stringify(needle)}`);
    }
  }

  return count;
}

function requireFlags(actual, expected, path) {
  const values = new Set(requireArray(actual, path));
  for (const flag of expected) {
    if (!values.has(flag)) {
      errors.push(`${path}: missing ${flag}`);
    }
  }
}

function walk(dir, files = []) {
  for (const entry of readdirSync(dir)) {
    if (skippedDirs.has(entry)) continue;
    const path = join(dir, entry);
    let stats;
    try {
      stats = lstatSync(path);
    } catch (error) {
      if (error?.code === 'ENOENT') continue;
      throw error;
    }
    if (stats.isSymbolicLink()) continue;
    if (stats.isDirectory()) {
      walk(path, files);
      continue;
    }
    files.push(normalize(path));
  }
  return files;
}

function cargoBuildScriptPath(manifestPath, text) {
  const match = text.match(/^\s*build\s*=\s*"([^"]+)"/mu);
  if (!match) return undefined;
  return normalize(join(dirname(manifestPath), match[1]));
}

function nativeSurfaceKey(path, kind) {
  return `${kind}:${path}`;
}

function detectNativeSurfaces() {
  const surfaces = [];

  for (const path of walk('.')) {
    const cleanPath = path.replace(/^\.\//u, '');
    if (cFamilyPattern.test(cleanPath)) {
      surfaces.push({ path: cleanPath, kind: 'c_family_file' });
      continue;
    }

    if (cleanPath.endsWith('Cargo.toml')) {
      const manifest = readFileSync(cleanPath, 'utf8');
      const buildScript = cargoBuildScriptPath(cleanPath, manifest);
      if (!buildScript || !existsSync(buildScript)) continue;
      const buildScriptText = readFileSync(buildScript, 'utf8');
      if (nativeBuildKeywords.some((keyword) => buildScriptText.includes(keyword))) {
        surfaces.push({
          path: buildScript.replace(/^\.\//u, ''),
          kind: 'cargo_native_build_script',
        });
      }
    }
  }

  return surfaces;
}

const registry = readJson(registryPath);

if (registry) {
  if (registry.schemaVersion !== expectedSchemaVersion) {
    errors.push(`${registryPath}: schemaVersion must be ${expectedSchemaVersion}`);
  }

  requireString(registry.source, `${registryPath}.source`);
  requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

  requireFlags(
    registry.profiles?.release?.requiredCflags,
    requiredReleaseCflags,
    'profiles.release.requiredCflags',
  );
  requireFlags(
    registry.profiles?.release?.requiredLinuxLdflags,
    requiredLinuxLdflags,
    'profiles.release.requiredLinuxLdflags',
  );

  const nativeSurfaces = requireArray(registry.nativeSurfaces, `${registryPath}.nativeSurfaces`);
  const registeredSurfaces = new Set();
  nativeSurfaces.forEach((surface, surfaceIndex) => {
    const context = `nativeSurfaces[${surfaceIndex}]`;
    const path = requireString(surface?.path, `${context}.path`);
    const kind = requireString(surface?.kind, `${context}.kind`);
    requireString(surface?.owner, `${context}.owner`);
    requireString(surface?.reason, `${context}.reason`);
    if (path && !existsSync(path)) {
      errors.push(`${context}.path: ${path} is missing`);
    }
    registeredSurfaces.add(nativeSurfaceKey(path, kind));
    if (kind === 'c_header') {
      registeredSurfaces.add(nativeSurfaceKey(path, 'c_family_file'));
    }
  });

  for (const surface of detectNativeSurfaces()) {
    if (!registeredSurfaces.has(nativeSurfaceKey(surface.path, surface.kind))) {
      errors.push(`${surface.path}: unregistered native surface (${surface.kind})`);
    }
  }

  const requirements = requireArray(registry.requirements, `${registryPath}.requirements`);
  const requirementIds = new Set();
  const controlIds = new Set();
  let controlCount = 0;
  let evidenceCount = 0;

  requirements.forEach((requirement, requirementIndex) => {
    const requirementContext = `requirements[${requirementIndex}]`;
    const requirementId = assertId(requirement?.id, `${requirementContext}.id`, /^CTH_REQ_\d{3}$/u);
    requireUnique(requirementId, requirementIds, 'requirements');
    requireString(requirement?.name, `${requirementContext}.name`);
    requireString(requirement?.owasp, `${requirementContext}.owasp`);

    const controls = requireArray(requirement?.controls, `${requirementContext}.controls`);
    if (controls.length === 0) {
      errors.push(`${requirementContext}: must include at least one control`);
    }

    controls.forEach((control, controlIndex) => {
      const controlContext = `${requirementContext}.controls[${controlIndex}]`;
      const controlId = assertId(control?.id, `${controlContext}.id`, /^CTH_CTRL_\d{3}$/u);
      requireUnique(controlId, controlIds, 'controls');
      requireString(control?.name, `${controlContext}.name`);
      requireString(control?.description, `${controlContext}.description`);
      controlCount += 1;

      const evidence = requireArray(control?.evidence, `${controlContext}.evidence`);
      if (evidence.length === 0) {
        errors.push(`${controlContext}: must include at least one evidence entry`);
      }

      evidence.forEach((entry, evidenceIndex) => {
        const evidenceContext = `${controlContext}.evidence[${evidenceIndex}]`;
        const path = requireString(entry?.path, `${evidenceContext}.path`);
        evidenceCount += requireIncludes(path, entry?.includes, evidenceContext);
      });
    });
  });

  if (errors.length === 0) {
    console.log(
      `C toolchain hardening controls: ok (${requirements.length} requirements, ${controlCount} controls, ${nativeSurfaces.length} native surfaces, ${evidenceCount} evidence strings)`,
    );
  }
}

if (errors.length > 0) {
  console.error('C toolchain hardening controls failed:');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  process.exit(1);
}
