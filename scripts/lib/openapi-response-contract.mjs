function responseContractForStatus(responses, status) {
  const exact = responses?.[String(status)];
  if (exact) {
    return exact;
  }

  const responseClass = `${Math.floor(status / 100)}XX`;
  const ranged = Object.entries(responses ?? {}).find(
    ([key]) => key.toUpperCase() === responseClass,
  );
  return ranged?.[1] ?? responses?.default;
}

function normalizedMediaType(value) {
  return value?.split(';', 1)[0]?.trim().toLowerCase();
}

function mediaTypeIsDeclared(actual, declared) {
  if (declared === actual || declared === '*/*') {
    return true;
  }
  if (declared.endsWith('/*')) {
    return actual.startsWith(declared.slice(0, -1));
  }
  if (declared === 'application/*+json') {
    return actual.startsWith('application/') && actual.endsWith('+json');
  }
  return false;
}

export function assertOpenApiProbeResponse({
  response,
  operation,
  operationLabel,
  forbidRedirects,
  authenticatedProbe,
}) {
  if (forbidRedirects && response.status >= 300 && response.status < 400) {
    throw new Error(`${operationLabel} returned forbidden redirect ${response.status}`);
  }
  if (response.status >= 500) {
    throw new Error(`${operationLabel} returned ${response.status}`);
  }
  if (authenticatedProbe && [401, 403].includes(response.status)) {
    throw new Error(
      `${operationLabel} rejected the seeded authenticated identity with ${response.status}`,
    );
  }

  const contract = responseContractForStatus(operation.responses, response.status);
  if (!contract) {
    throw new Error(`${operationLabel} returned undeclared OpenAPI status ${response.status}`);
  }

  const declaredMediaTypes = Object.keys(contract.content ?? {}).map((value) =>
    value.toLowerCase(),
  );
  if (declaredMediaTypes.length === 0 || [204, 304].includes(response.status)) {
    return;
  }

  const actualMediaType = normalizedMediaType(response.headers.get('content-type'));
  if (
    !actualMediaType ||
    !declaredMediaTypes.some((declared) => mediaTypeIsDeclared(actualMediaType, declared))
  ) {
    throw new Error(
      `${operationLabel} returned content-type ${actualMediaType ?? '<missing>'}; ` +
        `declared: ${declaredMediaTypes.join(', ')}`,
    );
  }
}
