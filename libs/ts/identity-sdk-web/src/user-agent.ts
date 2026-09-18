import {
  detectBrowser,
  detectBrowserVersion,
  type BrowserDetectionOptions,
  type BrowserHints,
} from './user-agent.browser';
import type {
  BrowserName,
  DeviceName,
  OperatingSystemName,
  UserAgentDeviceType,
} from './user-agent.constants';
import {
  detectDevice,
  detectDeviceType,
  detectOS,
  type DeviceTypeOptions,
} from './user-agent.device';

export interface UserAgentDetectionOptions extends BrowserDetectionOptions, DeviceTypeOptions {
  hints?: BrowserHints;
  vendor?: string;
}

export interface UserAgentInfo {
  browser: BrowserName | null;
  browserVersion: number | null;
  device: DeviceName | null;
  deviceType: UserAgentDeviceType;
  os: OperatingSystemName | null;
  osVersion: string | null;
}

export function parseUserAgent(
  userAgent: string,
  options: UserAgentDetectionOptions = {},
): UserAgentInfo {
  const browser = detectBrowser(userAgent, options.vendor, options.hints, options);
  const [os, osVersion] = detectOS(userAgent);

  return {
    browser,
    browserVersion: detectBrowserVersion(userAgent, options.vendor, options.hints, options),
    device: detectDevice(userAgent),
    deviceType: detectDeviceType(userAgent, options),
    os,
    osVersion,
  };
}

export { detectBrowser, detectBrowserVersion } from './user-agent.browser';
export type { BrowserDetectionOptions, BrowserHints } from './user-agent.browser';
export type {
  BrowserName,
  DeviceName,
  OperatingSystemName,
  UserAgentDeviceType,
} from './user-agent.constants';
export { detectDevice, detectDeviceType, detectOS } from './user-agent.device';
export type { DeviceTypeOptions, OperatingSystem } from './user-agent.device';
