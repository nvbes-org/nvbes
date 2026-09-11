// @vitest-environment happy-dom
import { act, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vite-plus/test';
import { ErrorBoundary } from './ErrorBoundary';
import { configureErrorReporting } from './report-error';

let root: Root;
let host: HTMLDivElement;
beforeEach(() => {
  vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', true);
  host = document.createElement('div');
  document.body.append(host);
  root = createRoot(host, { onCaughtError: vi.fn() });
  configureErrorReporting({ reporter: null });
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  configureErrorReporting({ reporter: null });
  vi.unstubAllGlobals();
});
function Broken({ error }: { error: Error }): ReactNode {
  throw error;
}

it('renders healthy children without reports', async () => {
  const report = vi.fn();
  await act(async () =>
    root.render(
      <ErrorBoundary onReport={report}>
        <p>Ready</p>
      </ErrorBoundary>,
    ),
  );
  expect(host.textContent).toBe('Ready');
  expect(report).not.toHaveBeenCalled();
});

it('catches an error, reports the feature once and recovers on retry', async () => {
  const error = new Error('Unavailable');
  const captureException = vi.fn();
  const onError = vi.fn();
  configureErrorReporting({ reporter: { captureException } });
  await act(async () =>
    root.render(
      <ErrorBoundary name="profile" onError={onError}>
        <Broken error={error} />
      </ErrorBoundary>,
    ),
  );
  expect(host.querySelector('h1')?.textContent).toBe('Une erreur est survenue');
  expect(host.textContent).toContain('Unavailable');
  expect(onError).toHaveBeenCalledWith(
    error,
    expect.objectContaining({ componentStack: expect.any(String) }),
  );
  expect(captureException).toHaveBeenCalledExactlyOnceWith(error, {
    tags: { feature: 'profile', source: 'error_boundary' },
  });
  expect(
    [...host.querySelectorAll('button')].find((button) => button.textContent === 'Signalé')
      ?.disabled,
  ).toBe(true);
  await act(async () =>
    root.render(
      <ErrorBoundary name="profile">
        <p>Recovered</p>
      </ErrorBoundary>,
    ),
  );
  await act(async () => host.querySelector('button')?.click());
  expect(host.textContent).toBe('Recovered');
  expect(captureException).toHaveBeenCalledTimes(1);
});

it('uses the supplied fallback and respects hidden report controls', async () => {
  await act(async () =>
    root.render(
      <ErrorBoundary fallback={<p>Custom fallback</p>}>
        <Broken error={new Error('failure')} />
      </ErrorBoundary>,
    ),
  );
  expect(host.textContent).toBe('Custom fallback');
  await act(async () =>
    root.render(
      <ErrorBoundary key="new" showReport={false}>
        <Broken error={new Error('failure')} />
      </ErrorBoundary>,
    ),
  );
  expect(host.querySelectorAll('button')).toHaveLength(1);
});

it('provides custom reset and report handlers with the current reporting state', async () => {
  const onReport = vi.fn();
  const error = new Error('failure');
  await act(async () =>
    root.render(
      <ErrorBoundary
        showReport
        onReport={onReport}
        renderFallback={({ error: caught, onReset, onReport: report, canReport, reported }) => (
          <div>
            <output>{`${caught.message}:${canReport}:${reported}`}</output>
            <button onClick={report}>Report</button>
            <button onClick={onReset}>Reset</button>
          </div>
        )}
      >
        <Broken error={error} />
      </ErrorBoundary>,
    ),
  );
  expect(host.querySelector('output')?.textContent).toBe('failure:true:true');
  await act(async () => host.querySelector('button')?.click());
  expect(onReport).toHaveBeenCalledTimes(2);
  expect(onReport).toHaveBeenLastCalledWith(error);
});
