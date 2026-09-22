import { existsSync, readFileSync } from 'node:fs';

/**
 * Shared helpers for docs/security/*.json control checkers.
 * Keeps per-domain scripts focused on V1 evidence assertions.
 */
export function createControlsCheckContext() {
  /** @type {string[]} */
  const errors = [];

  /**
   * @param {string} path
   * @returns {string}
   */
  function readText(path) {
    if (!existsSync(path)) {
      errors.push(`${path}: missing`);
      return '';
    }
    return readFileSync(path, 'utf8');
  }

  /**
   * @param {string} path
   * @returns {unknown}
   */
  function readJson(path) {
    const text = readText(path);
    if (!text) return undefined;
    try {
      return JSON.parse(text);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      errors.push(`${path}: invalid JSON (${message})`);
      return undefined;
    }
  }

  /**
   * @param {unknown} value
   * @param {string} path
   * @returns {unknown[]}
   */
  function requireArray(value, path) {
    if (Array.isArray(value)) return value;
    errors.push(`${path}: must be an array`);
    return [];
  }

  /**
   * @param {unknown} value
   * @param {string} path
   * @returns {string}
   */
  function requireString(value, path) {
    if (typeof value === 'string' && value.trim().length > 0) return value;
    errors.push(`${path}: must be a non-empty string`);
    return '';
  }

  /**
   * @param {string} path
   * @param {unknown} includes
   * @param {string} context
   * @returns {number}
   */
  function requireIncludes(path, includes, context) {
    const text = readText(path);
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

  /**
   * @param {string} path
   * @param {string[]} needles
   * @param {string} missingPrefix
   */
  function requireFileIncludes(path, needles, missingPrefix) {
    const text = readText(path);
    for (const needle of needles) {
      if (!text.includes(needle)) {
        errors.push(`${missingPrefix} missing ${needle}`);
      }
    }
    return text;
  }

  /**
   * @param {{
   *   registry: unknown,
   *   registryPath: string,
   *   requirementIdPattern: RegExp,
   *   controlIdPattern: RegExp,
   *   minRequirements: number,
   *   minControls: number,
   *   minEvidence: number,
   * }} options
   */
  function assertControlsRegistry(options) {
    const {
      registry,
      registryPath,
      requirementIdPattern,
      controlIdPattern,
      minRequirements,
      minControls,
      minEvidence,
    } = options;
    if (!registry || typeof registry !== 'object') return;

    const record = /** @type {Record<string, unknown>} */ (registry);
    if (record.schemaVersion !== 1) {
      errors.push(`${registryPath}: schemaVersion must be 1`);
    }
    requireString(record.source, `${registryPath}.source`);
    requireString(record.reviewCadence, `${registryPath}.reviewCadence`);

    const reqIds = new Set();
    const ctrlIds = new Set();
    let evidenceCount = 0;

    for (const [reqIndex, req] of requireArray(record.requirements, 'requirements').entries()) {
      const reqPath = `${registryPath}.requirements[${reqIndex}]`;
      const reqRecord = /** @type {Record<string, unknown>} */ (req);
      const reqId = requireString(reqRecord.id, `${reqPath}.id`);
      if (reqId && !requirementIdPattern.test(reqId)) {
        errors.push(`${reqPath}.id: invalid ID ${reqId}`);
      }
      if (reqIds.has(reqId)) errors.push(`${reqPath}.id: duplicate ${reqId}`);
      reqIds.add(reqId);
      requireString(reqRecord.name, `${reqPath}.name`);
      requireString(reqRecord.owasp, `${reqPath}.owasp`);

      for (const [ctrlIndex, ctrl] of requireArray(
        reqRecord.controls,
        `${reqPath}.controls`,
      ).entries()) {
        const ctrlPath = `${reqPath}.controls[${ctrlIndex}]`;
        const ctrlRecord = /** @type {Record<string, unknown>} */ (ctrl);
        const ctrlId = requireString(ctrlRecord.id, `${ctrlPath}.id`);
        if (ctrlId && !controlIdPattern.test(ctrlId)) {
          errors.push(`${ctrlPath}.id: invalid ID ${ctrlId}`);
        }
        if (ctrlIds.has(ctrlId)) errors.push(`${ctrlPath}.id: duplicate ${ctrlId}`);
        ctrlIds.add(ctrlId);
        requireString(ctrlRecord.name, `${ctrlPath}.name`);
        requireString(ctrlRecord.description, `${ctrlPath}.description`);

        for (const [evidenceIndex, evidence] of requireArray(
          ctrlRecord.evidence,
          `${ctrlPath}.evidence`,
        ).entries()) {
          const evidenceRecord = /** @type {Record<string, unknown>} */ (evidence);
          evidenceCount += requireIncludes(
            requireString(evidenceRecord.path, `${ctrlPath}.evidence[${evidenceIndex}].path`),
            evidenceRecord.includes,
            `${ctrlPath}.evidence[${evidenceIndex}]`,
          );
        }
      }
    }

    if (reqIds.size < minRequirements) {
      errors.push(`${registryPath}: expected at least ${minRequirements} requirements`);
    }
    if (ctrlIds.size < minControls) {
      errors.push(`${registryPath}: expected at least ${minControls} controls`);
    }
    if (evidenceCount < minEvidence) {
      errors.push(`${registryPath}: expected substantial evidence coverage`);
    }
  }

  /**
   * @param {{ scriptName: string, commandPath: string, label: string }} options
   */
  function assertPackageCheckScript(options) {
    const { scriptName, commandPath, label } = options;
    const pkg = readText('package.json');
    if (!pkg.includes(scriptName)) {
      errors.push(`package.json: ${scriptName} script missing`);
    }
    if (!pkg.includes(commandPath)) {
      errors.push(`package.json: ${label} checker command missing`);
    }
  }

  /**
   * @param {string} failureTitle
   * @param {string} okMessage
   */
  function finish(failureTitle, okMessage) {
    if (errors.length > 0) {
      console.error(`${failureTitle}:`);
      for (const error of errors) console.error(`- ${error}`);
      process.exit(1);
    }
    console.log(okMessage);
  }

  return {
    errors,
    readText,
    readJson,
    requireFileIncludes,
    assertControlsRegistry,
    assertPackageCheckScript,
    finish,
  };
}

/**
 * @param {{
 *   registryPath: string,
 *   requirementIdPattern: RegExp,
 *   controlIdPattern: RegExp,
 *   minRequirements?: number,
 *   minControls?: number,
 *   minEvidence?: number,
 *   packageScript: { scriptName: string, commandPath: string, label: string },
 *   runDomainAssertions: (ctx: ReturnType<typeof createControlsCheckContext>) => void,
 *   failureTitle: string,
 *   okMessage: string,
 * }} options
 */
export function runControlsRegistryCheck(options) {
  const ctx = createControlsCheckContext();
  ctx.assertControlsRegistry({
    registry: ctx.readJson(options.registryPath),
    registryPath: options.registryPath,
    requirementIdPattern: options.requirementIdPattern,
    controlIdPattern: options.controlIdPattern,
    minRequirements: options.minRequirements ?? 8,
    minControls: options.minControls ?? 8,
    minEvidence: options.minEvidence ?? 16,
  });
  options.runDomainAssertions(ctx);
  ctx.assertPackageCheckScript(options.packageScript);
  ctx.finish(options.failureTitle, options.okMessage);
}
