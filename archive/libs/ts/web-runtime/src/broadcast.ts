type ChannelMessage =
  | { type: 'logout'; reason?: string }
  | { type: 'workspace_change'; workspaceId: string }
  | { type: 'token_refreshed' };

type Listener = (message: ChannelMessage) => void;

const CHANNEL_NAME = 'nvbes';

let channel: BroadcastChannel | null = null;
const listeners = new Set<Listener>();

function getChannel(): BroadcastChannel {
  if (!channel) {
    channel = new BroadcastChannel(CHANNEL_NAME);
    channel.onmessage = (event: MessageEvent<ChannelMessage>) => {
      for (const listener of listeners) {
        listener(event.data);
      }
    };
  }
  return channel;
}

export function broadcast(message: ChannelMessage) {
  if (typeof BroadcastChannel === 'undefined') return;
  getChannel().postMessage(message);
}

export function onBroadcast(callback: Listener): () => void {
  listeners.add(callback);
  if (typeof BroadcastChannel !== 'undefined') {
    getChannel();
  }
  return () => {
    listeners.delete(callback);
  };
}

export function broadcastLogout(reason?: string) {
  broadcast({ type: 'logout', reason });
}

export function broadcastWorkspaceChange(workspaceId: string) {
  broadcast({ type: 'workspace_change', workspaceId });
}

export function broadcastTokenRefreshed() {
  broadcast({ type: 'token_refreshed' });
}
