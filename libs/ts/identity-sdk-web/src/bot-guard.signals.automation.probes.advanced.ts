interface UserAgentDataBrand {
  brand: string;
  version: string;
}

interface NavigatorUAData {
  brands: UserAgentDataBrand[];
  mobile: boolean;
  platform: string;
}

export function probeIframeWebdriver(): boolean {
  try {
    const iframe = document.createElement('iframe');
    iframe.style.display = 'none';
    iframe.srcdoc = '';
    document.head.appendChild(iframe);
    const leakedWebdriver = Boolean(iframe.contentWindow?.navigator.webdriver);
    document.head.removeChild(iframe);
    return leakedWebdriver;
  } catch {
    return false;
  }
}

export function probeUADataInconsistency(): boolean {
  try {
    const userAgentData = (navigator as unknown as { userAgentData?: NavigatorUAData })
      .userAgentData;
    if (!userAgentData?.platform) return false;

    const userAgent = navigator.userAgent.toLowerCase();
    const platform = userAgentData.platform.toLowerCase();
    if (
      platform.includes('win') &&
      !userAgent.includes('windows') &&
      !userAgent.includes('win64') &&
      !userAgent.includes('wow64')
    ) {
      return true;
    }
    if (
      platform.includes('mac') &&
      !userAgent.includes('macintosh') &&
      !userAgent.includes('mac os')
    ) {
      return true;
    }
    if (
      platform.includes('linux') &&
      !userAgent.includes('linux') &&
      !userAgent.includes('android')
    ) {
      return true;
    }
    return userAgentData.mobile !== /mobi|android|iphone|ipad/i.test(userAgent);
  } catch {
    return false;
  }
}

export function probeChromeSpoofing(): boolean {
  try {
    const userAgent = navigator.userAgent;
    const isChrome =
      /Chrome|HeadlessChrome/.test(userAgent) &&
      !/Edge|Edg|OPR|Firefox|Safari\/[0-9.]+$/.test(userAgent);
    if (!isChrome || /Mobile|Android|iPhone|iPad/i.test(userAgent)) return false;

    const browserWindow = window as unknown as { chrome?: Record<string, unknown> };
    if (browserWindow.chrome === undefined || !browserWindow.chrome.runtime) return true;

    const descriptor = Object.getOwnPropertyDescriptor(window, 'chrome');
    return Boolean(descriptor && (descriptor.get !== undefined || descriptor.set !== undefined));
  } catch {
    return true;
  }
}
