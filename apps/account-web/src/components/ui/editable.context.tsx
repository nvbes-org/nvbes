'use client';

import * as React from 'react';

export type EditableDirection = 'ltr' | 'rtl';

export interface EditableDivProps extends React.ComponentProps<'div'> {
  asChild?: boolean;
}

export interface EditableStoreState {
  value: string;
  editing: boolean;
}

export interface EditableStore {
  subscribe: (callback: () => void) => () => void;
  getState: () => EditableStoreState;
  setState: <Key extends keyof EditableStoreState>(
    key: Key,
    value: EditableStoreState[Key],
  ) => void;
  notify: () => void;
}

export interface EditableContextValue {
  rootId: string;
  inputId: string;
  labelId: string;
  defaultValue: string;
  onCancel: () => void;
  onEdit: () => void;
  onSubmit: (value: string) => void;
  onEnterKeyDown?: (event: KeyboardEvent) => void;
  onEscapeKeyDown?: (event: KeyboardEvent) => void;
  dir?: EditableDirection;
  maxLength?: number;
  placeholder?: string;
  triggerMode: 'click' | 'dblclick' | 'focus';
  autosize: boolean;
  disabled?: boolean;
  readOnly?: boolean;
  required?: boolean;
  invalid?: boolean;
}

export const EditableStoreContext = React.createContext<EditableStore | null>(null);
export const EditableContext = React.createContext<EditableContextValue | null>(null);

export function useEditableStoreContext(consumerName: string): EditableStore {
  const context = React.useContext(EditableStoreContext);
  if (!context) {
    throw new Error(`\`${consumerName}\` must be used within \`Editable\``);
  }
  return context;
}

export function useEditableStore<Selected>(
  selector: (state: EditableStoreState) => Selected,
  providedStore?: EditableStore | null,
): Selected {
  const contextStore = React.useContext(EditableStoreContext);
  const store = providedStore ?? contextStore;
  if (!store) {
    throw new Error('`useEditable` must be used within `Editable`');
  }

  const getSnapshot = React.useCallback(() => selector(store.getState()), [store, selector]);
  return React.useSyncExternalStore(store.subscribe, getSnapshot, getSnapshot);
}

export function useEditableContext(consumerName: string): EditableContextValue {
  const context = React.useContext(EditableContext);
  if (!context) {
    throw new Error(`\`${consumerName}\` must be used within \`Editable\``);
  }
  return context;
}
