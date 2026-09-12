import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { collectFingerprintSignals } from './bot-guard.signals.fingerprint';

const ctx = {
  font: '',
  fillStyle: '',
  strokeStyle: '',
  lineWidth: 0,
  fillRect: vi.fn(),
  fillText: vi.fn(),
  beginPath: vi.fn(),
  arc: vi.fn(),
  stroke: vi.fn(),
  measureText: vi.fn(() => ({ width: 10 })),
};
const gl = {
  getExtension: vi.fn(() => ({ UNMASKED_VENDOR_WEBGL: 1, UNMASKED_RENDERER_WEBGL: 2 })),
  getParameter: vi.fn((parameter: number) => (parameter === 1 ? 'Test vendor' : 'Test renderer')),
};
const fonts = { ready: Promise.resolve(), check: vi.fn((font: string) => font === '12px "Arial"') };
const canvas = {
  width: 0,
  height: 0,
  getContext: vi.fn((kind: string): unknown => (kind === '2d' ? ctx : gl)),
  toDataURL: vi.fn(() => 'data:image/png;synthetic'),
};

beforeEach(() => {
  vi.stubEnv('DEV', false);
  vi.stubGlobal('localStorage', undefined);
  vi.stubGlobal('navigator', { platform: 'Test OS' });
  vi.stubGlobal('screen', { width: 1280, height: 720, colorDepth: 24 });
  vi.stubGlobal('window', { devicePixelRatio: 2 });
  vi.stubGlobal('document', { createElement: () => canvas, fonts });
  vi.stubGlobal('crypto', { subtle: { digest: async () => new Uint8Array([0, 15, 255]).buffer } });
});
afterEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  vi.unstubAllGlobals();
  vi.unstubAllEnvs();
});

it('collects deterministic drawing, font and graphics metadata without requiring audio', async () => {
  expect(await collectFingerprintSignals()).toEqual({
    canvas_hash: '000fff',
    webgl_vendor: 'Test vendor',
    webgl_renderer: 'Test renderer',
    font_count: 1,
    screen_width: 1280,
    screen_height: 720,
    color_depth: 24,
    pixel_ratio: 2,
    platform: 'Test OS',
    audio_hash: 'unavailable',
  });
  expect([canvas.width, canvas.height]).toEqual([220, 30]);
  expect(ctx.fillRect).toHaveBeenCalledExactlyOnceWith(0, 0, 220, 30);
  expect(ctx.fillText).toHaveBeenCalledExactlyOnceWith('nvbes™ π√∑ éàü', 10, 20);
  expect(ctx.arc).toHaveBeenCalledExactlyOnceWith(180, 15, 10, 0, Math.PI * 2);
  expect(canvas.toDataURL).toHaveBeenCalledExactlyOnceWith('image/png');
  expect(fonts.check).toHaveBeenCalledTimes(20);
  expect(gl.getExtension).toHaveBeenCalledExactlyOnceWith('WEBGL_debug_renderer_info');
});
it('uses the legacy WebGL context when the standard context is absent', async () => {
  canvas.getContext
    .mockImplementationOnce(() => ctx)
    .mockImplementationOnce(() => null)
    .mockImplementationOnce(() => gl);
  expect((await collectFingerprintSignals()).webgl_vendor).toBe('Test vendor');
  expect(canvas.getContext).toHaveBeenCalledWith('experimental-webgl');
});
it('returns neutral values for unavailable canvas, WebGL and platform', async () => {
  canvas.getContext
    .mockImplementationOnce(() => null)
    .mockImplementationOnce(() => null)
    .mockImplementationOnce(() => null);
  vi.stubGlobal('navigator', {});
  expect(await collectFingerprintSignals()).toMatchObject({
    canvas_hash: 'unavailable',
    webgl_vendor: null,
    webgl_renderer: null,
    platform: 'unknown',
  });
});
it('contains drawing and graphics driver errors', async () => {
  canvas.toDataURL.mockImplementationOnce(() => {
    throw new Error('tainted');
  });
  gl.getExtension.mockImplementationOnce(() => {
    throw new Error('denied');
  });
  expect(await collectFingerprintSignals()).toMatchObject({
    canvas_hash: 'error',
    webgl_vendor: null,
    webgl_renderer: null,
  });
});
it('measures fonts against monospace when font enumeration is unavailable', async () => {
  vi.stubGlobal('document', {
    createElement: () => canvas,
    get fonts() {
      throw new Error('unsupported');
    },
  });
  ctx.measureText.mockImplementation(() => ({
    width: ctx.font === '12px "Arial", monospace' ? 20 : 10,
  }));
  expect((await collectFingerprintSignals()).font_count).toBe(1);
  expect(ctx.measureText).toHaveBeenCalledWith('mmmmmmmmmmlli');
});
it.each([false, true])(
  'permits debug overrides only in development (DEV=%s)',
  async (development) => {
    vi.stubEnv('DEV', development);
    const values: Record<string, string> = {
      __bg_mock_canvas_hash: 'debug',
      __bg_mock_webgl_vendor: 'debug-vendor',
      __bg_mock_webgl_renderer: 'debug-renderer',
      __bg_mock_font_count: '12',
    };
    const getItem = vi.fn((key: string) => values[key]);
    vi.stubGlobal('localStorage', { getItem });
    const result = await collectFingerprintSignals();
    expect(result.canvas_hash).toBe(development ? 'debug' : '000fff');
    expect(result.font_count).toBe(development ? 12 : 1);
    expect(result.webgl_vendor).toBe(development ? 'debug-vendor' : 'Test vendor');
    expect(result.webgl_renderer).toBe(development ? 'debug-renderer' : 'Test renderer');
    expect(getItem).toHaveBeenCalledTimes(development ? 4 : 0);
  },
);
it.each([3, 4501])(
  'bounds the audio hash to the first 4500 samples (length=%s)',
  async (length) => {
    const parameter = () => ({ setValueAtTime: vi.fn() });
    const oscillator = { type: '', frequency: parameter(), connect: vi.fn(), start: vi.fn() };
    const compressor = {
      threshold: parameter(),
      knee: parameter(),
      ratio: parameter(),
      attack: parameter(),
      release: parameter(),
      connect: vi.fn(),
    };
    class Audio {
      currentTime = 0;
      destination = {};
      constructor(channels: number, samples: number, rate: number) {
        expect([channels, samples, rate]).toEqual([1, 44100, 44100]);
      }
      createOscillator() {
        return oscillator;
      }
      createDynamicsCompressor() {
        return compressor;
      }
      async startRendering() {
        return { getChannelData: () => new Float32Array(length).fill(-0.5) };
      }
    }
    vi.stubGlobal('window', { devicePixelRatio: 1, OfflineAudioContext: Audio });
    expect((await collectFingerprintSignals()).audio_hash).toBe(
      (Math.min(length, 4500) / 2).toFixed(6),
    );
    expect(oscillator.type).toBe('triangle');
    expect(oscillator.frequency.setValueAtTime).toHaveBeenCalledExactlyOnceWith(1000, 0);
    expect(oscillator.connect).toHaveBeenCalledExactlyOnceWith(compressor);
    expect(oscillator.start).toHaveBeenCalledTimes(1);
  },
);
