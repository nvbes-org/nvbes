import {
  probeChromeSpoofing,
  probeIframeWebdriver,
  probeUADataInconsistency,
} from './bot-guard.signals.automation.probes.advanced';
import {
  probeChromeDriver,
  probeChromeRuntime,
  probeGlobalTools,
  probeNativeFunctionSpoofing,
  probePlugins,
  probeWebdriver,
} from './bot-guard.signals.automation.probes.basic';
import type { AutomationSignals } from './bot-guard.signals.automation.types';

export type { AutomationSignals } from './bot-guard.signals.automation.types';

export function collectAutomationSignals(): AutomationSignals {
  const webdriver = probeWebdriver();
  const mocks = readDevelopmentMocks();

  return {
    auto_webdriver: mocks.webdriver || webdriver.webdriver,
    auto_webdriver_spoofed: mocks.webdriver || webdriver.spoofed,
    auto_chrome_driver_injected: mocks.chromeDriverInjected || probeChromeDriver(),
    auto_global_tools_detected: mocks.globalToolsDetected || probeGlobalTools(),
    auto_chrome_runtime_missing: mocks.chromeRuntimeMissing || probeChromeRuntime(),
    auto_native_function_spoofed: mocks.nativeSpoofed || probeNativeFunctionSpoofing(),
    auto_plugins_inconsistent: mocks.pluginsInconsistent || probePlugins(),
    auto_iframe_webdriver: mocks.iframeWebdriver || probeIframeWebdriver(),
    auto_ua_data_inconsistent: mocks.userAgentInconsistent || probeUADataInconsistency(),
    auto_chrome_spoofed: mocks.chromeSpoofed || probeChromeSpoofing(),
  };
}

function readDevelopmentMocks() {
  const isDevelopment = import.meta.env.DEV;
  const read = (key: string): boolean =>
    isDevelopment && typeof localStorage !== 'undefined' && localStorage.getItem(key) === 'true';

  return {
    webdriver: read('__bg_mock_webdriver'),
    iframeWebdriver: read('__bg_mock_iframe_webdriver'),
    chromeSpoofed: read('__bg_mock_chrome_spoofed'),
    userAgentInconsistent: read('__bg_mock_ua_inconsistent'),
    nativeSpoofed: read('__bg_mock_native_spoofed'),
    chromeRuntimeMissing: read('__bg_mock_chrome_runtime_missing'),
    pluginsInconsistent: read('__bg_mock_plugins_inconsistent'),
    globalToolsDetected: read('__bg_mock_global_tools_detected'),
    chromeDriverInjected: read('__bg_mock_chrome_driver_injected'),
  };
}
