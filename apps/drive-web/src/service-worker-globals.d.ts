export {};

declare global {
  interface ExtendableEvent extends Event {
    waitUntil(promise: Promise<unknown>): void;
  }

  interface ExtendableMessageEvent extends ExtendableEvent {
    readonly data?: any;
    readonly ports?: MessagePort[];
  }
}
