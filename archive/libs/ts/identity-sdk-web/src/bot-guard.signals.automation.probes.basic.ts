export function probeWebdriver(): { webdriver: boolean; spoofed: boolean } {
  const hasWebdriver = !!navigator.webdriver;
  let spoofed = false;
  try {
    if (Object.getOwnPropertyDescriptor(navigator, 'webdriver') !== undefined) {
      spoofed = true;
    }
    const prototypeDescriptor = Object.getOwnPropertyDescriptor(Navigator.prototype, 'webdriver');
    if (prototypeDescriptor && typeof prototypeDescriptor.get !== 'function') {
      spoofed = true;
    }
  } catch {
    spoofed = true;
  }
  return { webdriver: hasWebdriver, spoofed };
}

export function probeChromeDriver(): boolean {
  try {
    for (const key of Object.getOwnPropertyNames(window)) {
      if (key.includes('cdc_') || key.startsWith('$cdc_')) return true;
    }
    for (const key of Object.getOwnPropertyNames(document)) {
      if (key.includes('cdc_') || key.startsWith('$cdc_')) return true;
    }
  } catch {
    return false;
  }
  return false;
}

export function probeGlobalTools(): boolean {
  try {
    const browserWindow = window as unknown as Record<string, unknown>;
    return (
      browserWindow._selenium !== undefined ||
      browserWindow.callSelenium !== undefined ||
      browserWindow._phantom !== undefined ||
      browserWindow.callPhantom !== undefined ||
      browserWindow.Cypress !== undefined ||
      browserWindow.__cypress !== undefined ||
      browserWindow.__playwright !== undefined ||
      browserWindow.__playwright_coverage !== undefined ||
      browserWindow.__playwright_evaluation !== undefined ||
      browserWindow.__nightmare !== undefined ||
      browserWindow.domAutomation !== undefined ||
      browserWindow.domAutomationController !== undefined
    );
  } catch {
    return false;
  }
}

export function probeChromeRuntime(): boolean {
  try {
    const browserWindow = window as unknown as { chrome?: { runtime?: unknown } };
    const isDesktopChrome =
      /Chrome/.test(navigator.userAgent) && !/Mobile/.test(navigator.userAgent);
    return Boolean(browserWindow.chrome && !browserWindow.chrome.runtime && isDesktopChrome);
  } catch {
    return false;
  }
}

export function probeNativeFunctionSpoofing(): boolean {
  const isSpoofed = (readSource: () => string | undefined): boolean => {
    try {
      const source = readSource();
      return source ? !source.includes('[native code]') || source.includes('return') : false;
    } catch {
      return true;
    }
  };

  try {
    return (
      isSpoofed(() => navigator.permissions?.query?.toString()) ||
      isSpoofed(() => HTMLCanvasElement.prototype.toDataURL.toString()) ||
      isSpoofed(() => Function.prototype.toString.toString())
    );
  } catch {
    return true;
  }
}

export function probePlugins(): boolean {
  try {
    if (!navigator.plugins) return false;
    return (
      Object.prototype.toString.call(navigator.plugins) !== '[object PluginArray]' ||
      Array.isArray(navigator.plugins)
    );
  } catch {
    return true;
  }
}
