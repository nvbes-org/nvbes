const RETURN_TO_PARAM = 'return_to';

export function readLoginReturnTo(searchParams: URLSearchParams): string | null {
  return normalizeLoginReturnTo(
    searchParams.get(RETURN_TO_PARAM),
    window.location.origin,
    configuredReturnOrigins(window.location.origin),
  );
}

export function normalizeLoginReturnTo(
  value: string | null,
  currentOrigin: string,
  allowedOrigins: ReadonlySet<string>,
): string | null {
  if (!value) {
    return null;
  }

  try {
    const url = new URL(value, currentOrigin);
    if (!allowedOrigins.has(url.origin)) {
      return null;
    }

    if (url.origin === currentOrigin && value.startsWith('/')) {
      return `${url.pathname}${url.search}${url.hash}`;
    }

    return url.toString();
  } catch {
    return null;
  }
}

function configuredReturnOrigins(currentOrigin: string): ReadonlySet<string> {
  const origins = new Set<string>([currentOrigin]);
  const configuredValues = [
    import.meta.env.VITE_CONSOLE_WEB_BASE_URL,
    import.meta.env.VITE_IDENTITY_ALLOWED_RETURN_ORIGINS,
  ];

  for (const value of configuredValues) {
    for (const candidate of splitOriginConfig(value)) {
      const origin = toOrigin(candidate);
      if (origin) {
        origins.add(origin);
      }
    }
  }

  if (import.meta.env.DEV) {
    origins.add('http://localhost:5175');
  }

  return origins;
}

function splitOriginConfig(value: string | undefined): string[] {
  return value?.split(/[\s,]+/u).filter(Boolean) ?? [];
}

function toOrigin(value: string): string | null {
  try {
    return new URL(value).origin;
  } catch {
    return null;
  }
}
