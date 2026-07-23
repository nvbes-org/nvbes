export function parseUserAgent(ua: string): { browser: string; os: string } {
  let browser = 'Navigateur inconnu';
  let os = 'OS inconnu';

  if (ua.includes('Firefox')) browser = 'Firefox';
  else if (ua.includes('Edg')) browser = 'Edge';
  else if (ua.includes('Chrome')) browser = 'Chrome';
  else if (ua.includes('Safari')) browser = 'Safari';

  if (ua.includes('Windows')) os = 'Windows';
  else if (ua.includes('Mac OS') || ua.includes('Macintosh')) os = 'macOS';
  else if (ua.includes('Linux')) os = 'Linux';
  else if (ua.includes('Android')) os = 'Android';
  else if (ua.includes('iPhone') || ua.includes('iPad')) os = 'iOS';

  return { browser, os };
}

export function deviceTrustLabel(level: string | null): string {
  if (level === 'trusted') return 'Appareil fiable';
  if (level === 'recognized') return 'Appareil reconnu';
  if (level === 'restricted') return 'À vérifier';
  return 'Nouvel appareil';
}
