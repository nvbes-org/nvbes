'use client';

import * as DirectionPrimitive from '@radix-ui/react-direction';
import * as SlotPrimitive from '@radix-ui/react-slot';
import * as React from 'react';
import { VisuallyHiddenInput } from '@/components/visually-hidden-input';
import { useAsRef } from '@/hooks/use-as-ref';
import { useIsomorphicLayoutEffect } from '@/hooks/use-isomorphic-layout-effect';
import { useLazyRef } from '@/hooks/use-lazy-ref';
import { useComposedRefs } from '@/lib/compose-refs';
import { cn } from '@/lib/utils';
import {
  EditableContext,
  type EditableContextValue,
  type EditableDirection,
  type EditableDivProps,
  type EditableStore,
  EditableStoreContext,
  type EditableStoreState,
  useEditableStore,
} from './editable.context';

type RootElement = React.ComponentRef<'div'>;

export interface EditableProps extends Omit<EditableDivProps, 'onSubmit'> {
  id?: string;
  defaultValue?: string;
  value?: string;
  onValueChange?: (value: string) => void;
  defaultEditing?: boolean;
  editing?: boolean;
  onEditingChange?: (editing: boolean) => void;
  onCancel?: () => void;
  onEdit?: () => void;
  onSubmit?: (value: string) => void;
  onEscapeKeyDown?: (event: KeyboardEvent) => void;
  onEnterKeyDown?: (event: KeyboardEvent) => void;
  dir?: EditableDirection;
  maxLength?: number;
  name?: string;
  placeholder?: string;
  triggerMode?: EditableContextValue['triggerMode'];
  autosize?: boolean;
  disabled?: boolean;
  readOnly?: boolean;
  required?: boolean;
  invalid?: boolean;
}

export function Editable(props: EditableProps) {
  const {
    value: valueProp,
    defaultValue = '',
    defaultEditing,
    editing: editingProp,
    onValueChange,
    onEditingChange,
    onCancel: onCancelProp,
    onEdit: onEditProp,
    onSubmit: onSubmitProp,
    onEscapeKeyDown,
    onEnterKeyDown,
    dir: dirProp,
    maxLength,
    name,
    placeholder,
    triggerMode = 'click',
    asChild,
    autosize = false,
    disabled,
    required,
    readOnly,
    invalid,
    className,
    id,
    ref,
    ...rootProps
  } = props;

  const instanceId = React.useId();
  const rootId = id ?? instanceId;
  const inputId = React.useId();
  const labelId = React.useId();
  const dir = DirectionPrimitive.useDirection(dirProp);
  const previousValueRef = React.useRef(defaultValue);
  const [formTrigger, setFormTrigger] = React.useState<RootElement | null>(null);
  const composedRef = useComposedRefs(ref, (node) => setFormTrigger(node));
  const isFormControl = formTrigger ? !!formTrigger.closest('form') : true;

  const listenersRef = useLazyRef(() => new Set<() => void>());
  const stateRef = useLazyRef<EditableStoreState>(() => ({
    value: valueProp ?? defaultValue,
    editing: editingProp ?? defaultEditing ?? false,
  }));
  const propsRef = useAsRef({
    onValueChange,
    onEditingChange,
    onCancel: onCancelProp,
    onEdit: onEditProp,
    onSubmit: onSubmitProp,
    onEscapeKeyDown,
    onEnterKeyDown,
  });

  const store = React.useMemo<EditableStore>(
    () => ({
      subscribe: (callback) => {
        listenersRef.current.add(callback);
        return () => listenersRef.current.delete(callback);
      },
      getState: () => stateRef.current,
      setState: (key, value) => {
        if (Object.is(stateRef.current[key], value)) return;

        if (key === 'value' && typeof value === 'string') {
          stateRef.current.value = value;
          propsRef.current.onValueChange?.(value);
        } else if (key === 'editing' && typeof value === 'boolean') {
          stateRef.current.editing = value;
          propsRef.current.onEditingChange?.(value);
        }
        store.notify();
      },
      notify: () => {
        for (const callback of listenersRef.current) callback();
      },
    }),
    [listenersRef, stateRef, propsRef],
  );

  const value = useEditableStore((state) => state.value, store);

  useIsomorphicLayoutEffect(() => {
    if (valueProp !== undefined) store.setState('value', valueProp);
  }, [valueProp]);

  useIsomorphicLayoutEffect(() => {
    if (editingProp !== undefined) store.setState('editing', editingProp);
  }, [editingProp]);

  const onCancel = React.useCallback(() => {
    store.setState('value', previousValueRef.current);
    store.setState('editing', false);
    propsRef.current.onCancel?.();
  }, [store, propsRef]);

  const onEdit = React.useCallback(() => {
    previousValueRef.current = store.getState().value;
    store.setState('editing', true);
    propsRef.current.onEdit?.();
  }, [store, propsRef]);

  const onSubmit = React.useCallback(
    (newValue: string) => {
      store.setState('value', newValue);
      store.setState('editing', false);
      propsRef.current.onSubmit?.(newValue);
    },
    [store, propsRef],
  );

  const contextValue = React.useMemo<EditableContextValue>(
    () => ({
      rootId,
      inputId,
      labelId,
      defaultValue,
      onSubmit,
      onEdit,
      onCancel,
      onEscapeKeyDown,
      onEnterKeyDown,
      dir,
      maxLength,
      placeholder,
      triggerMode,
      autosize,
      disabled,
      readOnly,
      required,
      invalid,
    }),
    [
      rootId,
      inputId,
      labelId,
      defaultValue,
      onSubmit,
      onCancel,
      onEdit,
      onEscapeKeyDown,
      onEnterKeyDown,
      dir,
      maxLength,
      placeholder,
      triggerMode,
      autosize,
      disabled,
      required,
      readOnly,
      invalid,
    ],
  );

  const RootPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'div';

  return (
    <EditableStoreContext.Provider value={store}>
      <EditableContext.Provider value={contextValue}>
        <RootPrimitive
          data-slot="editable"
          {...rootProps}
          id={id}
          ref={composedRef}
          className={cn('flex min-w-0 flex-col gap-2', className)}
        />
        {isFormControl && (
          <VisuallyHiddenInput
            type="hidden"
            control={formTrigger}
            name={name}
            value={value}
            disabled={disabled}
            readOnly={readOnly}
            required={required}
          />
        )}
      </EditableContext.Provider>
    </EditableStoreContext.Provider>
  );
}
