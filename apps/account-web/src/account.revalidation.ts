import type { AccountController } from './account.controller';

interface PageActivity {
  windowEvents: EventTarget;
  documentEvents: EventTarget;
  active(): boolean;
}

/** One periodic check per minute of visible, online use; no background keepalive. */
export function watchAccountSession(
  controller: Pick<AccountController, 'snapshot' | 'subscribe' | 'revalidate'>,
  activity: PageActivity = {
    windowEvents: window,
    documentEvents: document,
    active: () => document.visibilityState === 'visible' && navigator.onLine,
  },
): () => void {
  let stopped = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  const cancel = () => {
    if (timer !== undefined) clearTimeout(timer);
    timer = undefined;
  };
  const check = () => {
    cancel();
    if (!stopped && activity.active() && controller.snapshot().stage === 'profile')
      void controller.revalidate();
  };
  const schedule = () => {
    cancel();
    if (!stopped && activity.active() && controller.snapshot().stage === 'profile')
      timer = setTimeout(check, 60_000);
  };
  const unsubscribe = controller.subscribe(schedule);
  for (const event of ['focus', 'online', 'offline'])
    activity.windowEvents.addEventListener(event, check);
  activity.documentEvents.addEventListener('visibilitychange', check);
  schedule();
  return () => {
    stopped = true;
    cancel();
    unsubscribe();
    for (const event of ['focus', 'online', 'offline'])
      activity.windowEvents.removeEventListener(event, check);
    activity.documentEvents.removeEventListener('visibilitychange', check);
  };
}
