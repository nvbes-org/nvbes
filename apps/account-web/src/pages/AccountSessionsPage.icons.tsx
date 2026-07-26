import type { BrowserKey, OsKey } from './AccountSessionsPage.device';

// --- OS Brand Icons (FontAwesome 6 Brands) ---

export function AppleIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 384 512"
      fill="currentColor"
      aria-label="Apple"
    >
      <path d="M318.7 268.7c-.2-36.7 16.4-64.4 50-84.8-18.8-26.9-47.2-41.7-84.7-44.6-35.5-2.8-74.3 20.7-88.5 20.7-15 0-49.4-19.7-76.4-19.7C63.3 141.2 4 184.8 4 273.5q0 39.3 14.4 81.2c12.8 36.7 59 126.7 107.2 125.2 25.2-.6 43-17.9 75.8-17.9 31.8 0 48.3 17.9 76.4 17.9 48.6-.7 90.4-82.5 102.6-119.3-65.2-30.7-61.7-90-61.7-91.9zm-56.6-164.2c27.3-32.4 24.8-61.9 24-72.5-24.1 1.4-52 16.4-67.9 34.9-17.5 19.8-27.8 44.3-25.6 71.9 26.1 2 52.3-11.4 69.5-34.3z" />
    </svg>
  );
}

export function WindowsIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 448 512"
      fill="currentColor"
      aria-label="Windows"
    >
      <path d="M0 93.7l183.6-25.3v177.4H0V93.7zm0 324.6l183.6 25.3V268.4H0v149.9zm203.8 28L448 480V268.4H203.8v173.9zm0-392.3v174H448V32L203.8 54z" />
    </svg>
  );
}

export function LinuxIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 448 512"
      fill="currentColor"
      aria-label="Linux"
    >
      <path d="M220.8 0C98.7 0 0 98.7 0 220.8c0 48.2 15.4 92.8 41.5 129.2L3.9 473.5c-3.1 9.4 4.5 18.5 14.1 15.4l123.5-37.6c36.4 26.1 81 41.5 129.2 41.5 122.1 0 220.8-98.7 220.8-220.8S342.9 0 220.8 0zm-84.5 162.7c15.6 0 28.3 12.7 28.3 28.3s-12.7 28.3-28.3 28.3-28.3-12.7-28.3-28.3 12.7-28.3 28.3-28.3zm169 0c15.6 0 28.3 12.7 28.3 28.3s-12.7 28.3-28.3 28.3-28.3-12.7-28.3-28.3 12.7-28.3 28.3-28.3zM220.8 384c-53 0-96-43-96-96h192c0 53-43 96-96 96z" />
    </svg>
  );
}

export function AndroidIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 576 512"
      fill="currentColor"
      aria-label="Android"
    >
      <path d="M420.55 301.93a24 24 0 1 1 24-24 24 24 0 0 1-24 24m-265.1 0a24 24 0 1 1 24-24 24.06 24.06 0 0 1-24 24m273.7-144.48 47.9-83a10 10 0 0 0-3.66-13.66l-1.41-.81a10 10 0 0 0-13.66 3.66l-48.43 83.94C371.3 128.23 322.84 116 268 116s-103.3 12.23-141.95 31.54l-48.43-83.94a10 10 0 0 0-13.66-3.66l-1.41.81a10 10 0 0 0-3.66 13.66l47.9 83C42.87 205.8 0 274.65 0 354h536c0-79.35-42.87-148.2-106.85-196.55" />
    </svg>
  );
}

// --- Browser Brand Icons (FontAwesome 6 Brands) ---

export function ChromeIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 512 512"
      fill="currentColor"
      aria-label="Chrome"
    >
      <path d="M0 256C0 209.4 12.47 165.6 34.27 127.1L144.1 318.3C166 357.5 207.9 384 256 384c14.3 0 27.1-2.3 40.8-6.6l-76.3 132.2C95.9 492.3 0 385.3 0 256zm365.1 65.6C377.4 302.4 384 279.1 384 256c0-38.2-16.8-72.5-43.3-96H493.4c12 29.6 18.6 62.1 18.6 96 0 141.4-114.6 255.1-256 256l109.1-190.4zM477.8 128H256c-62.9 0-113.7 44.1-125.5 102.7L54.19 98.47C101 38.53 174 0 256 0c94.8 0 177.5 51.48 221.8 128zM168 256c0-48.6 39.4-88 88-88s88 39.4 88 88-39.4 88-88 88-88-39.4-88-88z" />
    </svg>
  );
}

export function FirefoxIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 512 512"
      fill="currentColor"
      aria-label="Firefox"
    >
      <path d="M504.6 178.6a260.6 260.6 0 0 0-46.7-68.5c-43.2-46.4-101.5-74-165.7-78-15.3-1-30.8.2-45.9 3.5-31.6 6.9-61 21.6-85.3 42.6C122.9 44.9 66 112 66 189c0 23.3 5.4 46.1 15.6 66.8-21.7-18.7-36.2-44.7-40.8-73.4-1.2-7.5-8.5-12.4-15.8-10.4C9 176.4 0 190.7 0 206c0 134.8 109.2 244 244 244 125.7 0 229.8-94.9 242.4-217.7 1.4-13.7 2.3-27.6 1.8-41.5-.3-4.2-3.6-7.5-7.8-7.5-1.9 0-3.8.7-5.2 2.1-10.6 10.9-22.7 20-35.8 27.2zm-260.6 211.4c-77.3 0-140-62.7-140-140 0-38.7 15.7-73.7 41.1-99 15.8 28.5 44 48.7 77.2 53.6-18.8 19.3-30.3 45.6-30.3 74.4 0 57.4 46.6 104 104 104s104-46.6 104-104c0-21.4-6.5-41.3-17.7-57.8 2.2 6.5 3.7 13.3 3.7 20.4 0 37.6-30.4 68-68 68s-68-30.4-68-68c0-26.6 15.3-49.6 37.5-60.8 15.6-7.9 33.3-12.2 51.5-12.2 61.9 0 112 50.1 112 112 0 61.9-50.1 112-112 112z" />
    </svg>
  );
}

export function SafariIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 512 512"
      fill="currentColor"
      aria-label="Safari"
    >
      <path d="M274.69 274.69l-37.38-37.38L166 346zM256 8C119 8 8 119 8 256s111 248 248 248 248-111 248-248S393 8 256 8zm155.85 174.79 14.78-6.13a8 8 0 0 1 10.45 4.34 8 8 0 0 1-4.33 10.46L418 197.57a8 8 0 0 1-10.45-4.33 8 8 0 0 1 4.3-10.45zM314.43 94l6.12-14.78A8 8 0 0 1 331 74.92a8 8 0 0 1 4.33 10.45l-6.13 14.78a8 8 0 0 1-10.45 4.33 8 8 0 0 1-4.32-10.45zM256 60a8 8 0 0 1 8 8v16a8 8 0 0 1-16 0V68a8 8 0 0 1 8-8zm-75 14.92a8 8 0 0 1 10.46 4.33L197.57 94a8 8 0 1 1-14.78 6.12l-6.13-14.78A8 8 0 0 1 181 74.92zm-63.59 42.49a8 8 0 0 1 11.31 0L140 128.72A8 8 0 0 1 128.69 140l-11.28-11.31a8 8 0 0 1 0-11.28zM60 256a8 8 0 0 1 8-8h16a8 8 0 0 1 0 16H68a8 8 0 0 1-8-8zm40.15 73.21-14.78 6.13A8 8 0 0 1 74.92 331a8 8 0 0 1 4.33-10.46L94 314.43a8 8 0 0 1 10.45 4.33 8 8 0 0 1-4.3 10.45zm4.33-136a8 8 0 0 1-10.48 4.36l-14.78-6.12A8 8 0 0 1 74.92 181a8 8 0 0 1 10.45-4.33l14.78 6.13a8 8 0 0 1 4.33 10.44zm93.09 224.79-6.12 14.78a8 8 0 0 1-14.79-6.12l6.13-14.78a8 8 0 1 1 14.78 6.12zM264 444a8 8 0 0 1-16 0v-16a8 8 0 0 1 16 0zm67-6.92a8 8 0 0 1-10.46-4.33L314.43 418a8 8 0 0 1 14.78-6.12l6.13 14.78A8 8 0 0 1 331 437.08zm63.59-42.49a8 8 0 0 1-11.31 0L372 383.28A8 8 0 0 1 383.31 372l11.28 11.31a8 8 0 0 1 0 11.28zm-108.34-108.34L110.34 401.66l115.41-175.91 175.91-115.41zM437.08 331a8 8 0 0 1-10.45 4.33l-14.78-6.13a8 8 0 0 1-4.33-10.45 8 8 0 0 1 10.48-4.32l14.78 6.12a8 8 0 0 1 4.3 10.45zM444 264h-16a8 8 0 0 1 0-16h16a8 8 0 0 1 0 16z" />
    </svg>
  );
}

export function EdgeIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 512 512"
      fill="currentColor"
      aria-label="Edge"
    >
      <path d="M470.1 262.1c.3 11-1.1 22.3-4.5 33.3-13.8 45.4-50.6 77-96 82.8-19 2.4-38.3 1.1-56.8-4-32.9-9.1-59.5-28.7-76.3-56.4-17-28-21.7-61.2-13.7-93.8 8.8-35.9 31.8-65.1 63.8-81.2 28.5-14.3 61.2-18.4 92.7-11.4 34.6 7.7 64 27.6 82.5 56 12 18.4 18.5 40 18.5 62.1v12.6H256c0 19.8 6.7 39 19.1 54.4 12.9 16 31.7 26.2 52.3 28.3 22 2.2 44-4.8 60.1-19.1 11.2-10 18.5-23.7 20.8-38.6H470.1zm-84-48.1c.2-15.6-5.8-30.8-16.7-41.8-11.4-11.5-26.9-17.7-42.9-17-17 .7-33 7.8-44.5 19.8-11.6 12.1-17.7 28.2-16.9 45h121z" />
    </svg>
  );
}

export function OperaIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 496 512"
      fill="currentColor"
      aria-label="Opera"
    >
      <path d="M248 8C111 8 0 119 0 256s111 248 248 248 248-111 248-248S385 8 248 8zm0 416c-44.1 0-80-75.2-80-168s35.9-168 80-168 80 75.2 80 168-35.9 168-80 168z" />
    </svg>
  );
}

export function BraveIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 512 512"
      fill="currentColor"
      aria-label="Brave"
    >
      <path d="M256 0L48 96v160c0 141.4 88.5 220 208 256 119.5-36 208-114.6 208-256V96L256 0zm0 64l144 64v128c0 102.4-62.4 162.2-144 190.2C174.4 418.2 112 358.4 112 256V128l144-64z" />
    </svg>
  );
}

export function GlobeIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      role="img"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-label="Navigateur"
    >
      <circle cx="12" cy="12" r="10" />
      <line x1="2" y1="12" x2="22" y2="12" />
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
    </svg>
  );
}

export function OsIcon({ osKey, className }: { osKey: OsKey; className?: string }) {
  switch (osKey) {
    case 'macos':
    case 'ios':
      return <AppleIcon className={className} />;
    case 'windows':
      return <WindowsIcon className={className} />;
    case 'linux':
      return <LinuxIcon className={className} />;
    case 'android':
      return <AndroidIcon className={className} />;
    default:
      return null;
  }
}

export function BrowserIcon({
  browserKey,
  className,
}: {
  browserKey: BrowserKey;
  className?: string;
}) {
  switch (browserKey) {
    case 'chrome':
      return <ChromeIcon className={className} />;
    case 'brave':
      return <BraveIcon className={className} />;
    case 'firefox':
      return <FirefoxIcon className={className} />;
    case 'safari':
      return <SafariIcon className={className} />;
    case 'edge':
      return <EdgeIcon className={className} />;
    case 'opera':
      return <OperaIcon className={className} />;
    default:
      return <GlobeIcon className={className} />;
  }
}
