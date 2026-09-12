import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { collectAutomationSignals } from './bot-guard.signals.automation';
import * as basic from './bot-guard.signals.automation.probes.basic';
import * as advanced from './bot-guard.signals.automation.probes.advanced';

vi.mock('./bot-guard.signals.automation.probes.basic', () => ({
  probeWebdriver: vi.fn(),
  probeChromeDriver: vi.fn(),
  probeGlobalTools: vi.fn(),
  probeChromeRuntime: vi.fn(),
  probeNativeFunctionSpoofing: vi.fn(),
  probePlugins: vi.fn(),
}));
vi.mock('./bot-guard.signals.automation.probes.advanced', () => ({
  probeIframeWebdriver: vi.fn(),
  probeUADataInconsistency: vi.fn(),
  probeChromeSpoofing: vi.fn(),
}));
const mappings = [
  [basic.probeChromeDriver, 'auto_chrome_driver_injected', '__bg_mock_chrome_driver_injected'],
  [basic.probeGlobalTools, 'auto_global_tools_detected', '__bg_mock_global_tools_detected'],
  [basic.probeChromeRuntime, 'auto_chrome_runtime_missing', '__bg_mock_chrome_runtime_missing'],
  [basic.probeNativeFunctionSpoofing, 'auto_native_function_spoofed', '__bg_mock_native_spoofed'],
  [basic.probePlugins, 'auto_plugins_inconsistent', '__bg_mock_plugins_inconsistent'],
  [advanced.probeIframeWebdriver, 'auto_iframe_webdriver', '__bg_mock_iframe_webdriver'],
  [advanced.probeUADataInconsistency, 'auto_ua_data_inconsistent', '__bg_mock_ua_inconsistent'],
  [advanced.probeChromeSpoofing, 'auto_chrome_spoofed', '__bg_mock_chrome_spoofed'],
] as const;
beforeEach(() => {
  vi.stubEnv('DEV', false);
  vi.stubGlobal('localStorage', undefined);
  vi.mocked(basic.probeWebdriver).mockReturnValue({ webdriver: false, spoofed: false });
  for (const [probe] of mappings) vi.mocked(probe).mockReturnValue(false);
});
afterEach(() => {
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

it('keeps independent webdriver and spoofing verdicts', () => {
  expect(Object.values(collectAutomationSignals()).every((value) => value === false)).toBe(true);
  vi.mocked(basic.probeWebdriver).mockReturnValue({ webdriver: true, spoofed: false });
  expect(collectAutomationSignals()).toMatchObject({
    auto_webdriver: true,
    auto_webdriver_spoofed: false,
  });
  vi.mocked(basic.probeWebdriver).mockReturnValue({ webdriver: false, spoofed: true });
  expect(collectAutomationSignals()).toMatchObject({
    auto_webdriver: false,
    auto_webdriver_spoofed: true,
  });
});
it.each(mappings)('maps probe to %s / %s', (probe, field) => {
  vi.mocked(probe).mockReturnValue(true);
  const result = collectAutomationSignals();
  expect(result[field]).toBe(true);
  expect(Object.values(result).filter(Boolean)).toHaveLength(1);
});
it.each([false, true])('allows all development overrides only with DEV=%s', (development) => {
  vi.stubEnv('DEV', development);
  const getItem = vi.fn(() => 'true');
  vi.stubGlobal('localStorage', { getItem });
  expect(Object.values(collectAutomationSignals())).toEqual(Array(10).fill(development));
  expect(getItem).toHaveBeenCalledTimes(development ? 9 : 0);
  for (const [probe, , key] of mappings) {
    expect(probe).toHaveBeenCalledTimes(development ? 0 : 1);
    if (development) expect(getItem).toHaveBeenCalledWith(key);
  }
});
it('tolerates missing debug storage and does not treat arbitrary strings as true', () => {
  vi.stubEnv('DEV', true);
  expect(Object.values(collectAutomationSignals()).every((value) => value === false)).toBe(true);
  vi.stubGlobal('localStorage', { getItem: () => 'false' });
  expect(Object.values(collectAutomationSignals()).every((value) => value === false)).toBe(true);
});
