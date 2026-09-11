/**
 * Bot Guard — Fingerprint signals collector
 *
 * Collects deterministic browser fingerprint signals:
 *   - Canvas rendering hash (GPU/driver differences reveal headless)
 *   - WebGL vendor/renderer string (SwiftShader → headless)
 *   - Font availability count (headless has minimal font set)
 *   - Screen resolution, colour depth, pixel ratio
 *   - AudioContext oscilloscope hash (hardware-dependent)
 *   - Platform string
 */

export interface FingerprintSignals {
  canvas_hash: string;
  webgl_vendor: string | null;
  webgl_renderer: string | null;
  font_count: number;
  screen_width: number;
  screen_height: number;
  color_depth: number;
  pixel_ratio: number;
  platform: string;
  audio_hash: string;
}

// ---------------------------------------------------------------------------
// Canvas fingerprint
// ---------------------------------------------------------------------------

async function computeCanvasHash(): Promise<string> {
  try {
    const canvas = document.createElement('canvas');
    canvas.width = 220;
    canvas.height = 30;
    const ctx = canvas.getContext('2d');
    if (!ctx) return 'unavailable';

    // A deterministic draw that is GPU-dependent (anti-aliasing, subpixel rendering).
    ctx.fillStyle = '#f0f0f0';
    ctx.fillRect(0, 0, 220, 30);

    ctx.font = "14px 'Arial'";
    ctx.fillStyle = '#1a1a2e';
    ctx.fillText('nvbes\u2122 \u03c0\u221a\u2211 \u00e9\u00e0\u00fc', 10, 20);

    ctx.strokeStyle = 'rgba(100, 200, 50, 0.7)';
    ctx.lineWidth = 0.5;
    ctx.beginPath();
    ctx.arc(180, 15, 10, 0, Math.PI * 2);
    ctx.stroke();

    const dataUrl = canvas.toDataURL('image/png');
    const encoded = new TextEncoder().encode(dataUrl);
    const hashBuffer = await crypto.subtle.digest('SHA-256', encoded);
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  } catch {
    return 'error';
  }
}

// ---------------------------------------------------------------------------
// WebGL vendor / renderer
// ---------------------------------------------------------------------------

function probeWebGL(): { vendor: string | null; renderer: string | null } {
  try {
    const canvas = document.createElement('canvas');
    const gl =
      canvas.getContext('webgl') ??
      (canvas.getContext('experimental-webgl') as WebGLRenderingContext | null);
    if (!gl) return { vendor: null, renderer: null };

    const ext = gl.getExtension('WEBGL_debug_renderer_info');
    if (!ext) return { vendor: null, renderer: null };

    return {
      vendor: gl.getParameter(ext.UNMASKED_VENDOR_WEBGL) ?? null,
      renderer: gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) ?? null,
    };
  } catch {
    return { vendor: null, renderer: null };
  }
}

// ---------------------------------------------------------------------------
// Font enumeration
// The test set covers web-safe and common OS fonts.
// Headless Chrome typically has ≤ 3 available from this list.
// ---------------------------------------------------------------------------

const TEST_FONTS = [
  'Arial',
  'Helvetica',
  'Times New Roman',
  'Courier New',
  'Verdana',
  'Georgia',
  'Palatino',
  'Garamond',
  'Trebuchet MS',
  'Arial Black',
  'Impact',
  'Comic Sans MS',
  'Lucida Sans Unicode',
  'Tahoma',
  'Geneva',
  'Courier',
  'Monaco',
  'Andale Mono',
  'Webdings',
  'Symbol',
] as const;

async function countAvailableFonts(): Promise<number> {
  try {
    await document.fonts.ready;
    return TEST_FONTS.filter((font) => document.fonts.check(`12px "${font}"`)).length;
  } catch {
    // Fallback: measure text width against a baseline monospace font.
    // Fonts that differ from the baseline are available.
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) return 0;

    const baseline = 'monospace';
    const testStr = 'mmmmmmmmmmlli';
    ctx.font = `12px ${baseline}`;
    const baseWidth = ctx.measureText(testStr).width;

    return TEST_FONTS.filter((font) => {
      ctx.font = `12px "${font}", ${baseline}`;
      return ctx.measureText(testStr).width !== baseWidth;
    }).length;
  }
}

// ---------------------------------------------------------------------------
// Audio context fingerprint
// Hardware-dependent: sample rate, oscillator/dynamics compressor output
// ---------------------------------------------------------------------------

async function computeAudioHash(): Promise<string> {
  try {
    const ctx = new (window.OfflineAudioContext || window.AudioContext)(1, 44100, 44100);
    const oscillator = ctx.createOscillator();
    const compressor = ctx.createDynamicsCompressor();
    oscillator.type = 'triangle';
    oscillator.frequency.setValueAtTime(1000, ctx.currentTime);
    compressor.threshold.setValueAtTime(-50, ctx.currentTime);
    compressor.knee.setValueAtTime(40, ctx.currentTime);
    compressor.ratio.setValueAtTime(12, ctx.currentTime);
    compressor.attack.setValueAtTime(0, ctx.currentTime);
    compressor.release.setValueAtTime(0.25, ctx.currentTime);
    oscillator.connect(compressor);
    compressor.connect(ctx.destination);
    oscillator.start();
    const buffer = await ctx.startRendering();
    const raw = buffer.getChannelData(0);
    // Sum absolute values of first 4500 samples as a simple hash
    let hash = 0;
    for (let i = 0; i < 4500 && i < raw.length; i++) {
      hash += Math.abs(raw[i]);
    }
    return hash.toFixed(6);
  } catch {
    return 'unavailable';
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function collectFingerprintSignals(): Promise<FingerprintSignals> {
  const [canvasHash, fontCount, audioHash] = await Promise.all([
    computeCanvasHash(),
    countAvailableFonts(),
    computeAudioHash(),
  ]);

  const webgl = probeWebGL();

  const mockAllowed = import.meta.env.DEV && typeof localStorage !== 'undefined';

  const mockCanvasHash = (mockAllowed && localStorage.getItem('__bg_mock_canvas_hash')) || null;
  const mockWebglVendor = (mockAllowed && localStorage.getItem('__bg_mock_webgl_vendor')) || null;
  const mockWebglRenderer =
    (mockAllowed && localStorage.getItem('__bg_mock_webgl_renderer')) || null;
  const mockFontCount = (mockAllowed && localStorage.getItem('__bg_mock_font_count')) || null;

  return {
    canvas_hash: mockCanvasHash || canvasHash,
    webgl_vendor: mockWebglVendor !== null ? mockWebglVendor : webgl.vendor,
    webgl_renderer: mockWebglRenderer !== null ? mockWebglRenderer : webgl.renderer,
    font_count: mockFontCount ? parseInt(mockFontCount, 10) : fontCount,
    screen_width: screen.width,
    screen_height: screen.height,
    color_depth: screen.colorDepth,
    pixel_ratio: window.devicePixelRatio,
    platform: navigator.platform ?? 'unknown',
    audio_hash: audioHash,
  };
}
