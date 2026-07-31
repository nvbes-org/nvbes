'use client';

import * as SlotPrimitive from '@radix-ui/react-slot';
import * as React from 'react';
import { useAsRef } from '@/hooks/use-as-ref';
import { useIsomorphicLayoutEffect } from '@/hooks/use-isomorphic-layout-effect';
import { useComposedRefs } from '@/lib/compose-refs';
import { cn } from '@/lib/utils';
import { useEditableContext, useEditableStore, useEditableStoreContext } from './editable.context';

type InputElement = React.ComponentRef<'input'>;

export interface EditableInputProps extends React.ComponentProps<'input'> {
  asChild?: boolean;
  maxLength?: number;
}

export function EditableInput(props: EditableInputProps) {
  const {
    onBlur: onBlurProp,
    onChange: onChangeProp,
    onKeyDown: onKeyDownProp,
    asChild,
    className,
    disabled,
    readOnly,
    required,
    maxLength,
    ref,
    ...inputProps
  } = props;
  const context = useEditableContext('EditableInput');
  const store = useEditableStoreContext('EditableInput');
  const value = useEditableStore((state) => state.value);
  const editing = useEditableStore((state) => state.editing);
  const inputRef = React.useRef<InputElement>(null);
  const composedRef = useComposedRefs(ref, inputRef);
  const propsRef = useAsRef({
    onBlur: onBlurProp,
    onChange: onChangeProp,
    onKeyDown: onKeyDownProp,
  });
  const isDisabled = disabled || context.disabled;
  const isReadOnly = readOnly || context.readOnly;
  const isRequired = required || context.required;

  const onAutosize = React.useCallback(
    (target: InputElement) => {
      if (!context.autosize) return;
      if (target instanceof HTMLTextAreaElement) {
        target.style.height = '0';
        target.style.height = `${target.scrollHeight}px`;
      } else {
        target.style.width = '0';
        target.style.width = `${target.scrollWidth + 4}px`;
      }
    },
    [context.autosize],
  );

  const onBlur = React.useCallback(
    (event: React.FocusEvent<InputElement>) => {
      if (isDisabled || isReadOnly) return;
      propsRef.current.onBlur?.(event);
      if (event.defaultPrevented) return;

      const relatedTarget = event.relatedTarget;
      const isAction =
        relatedTarget instanceof HTMLElement &&
        (relatedTarget.closest('[data-slot="editable-trigger"]') ||
          relatedTarget.closest('[data-slot="editable-cancel"]'));
      if (!isAction) context.onSubmit(value);
    },
    [value, context.onSubmit, propsRef, isDisabled, isReadOnly],
  );

  const onChange = React.useCallback(
    (event: React.ChangeEvent<InputElement>) => {
      if (isDisabled || isReadOnly) return;
      propsRef.current.onChange?.(event);
      if (event.defaultPrevented) return;

      store.setState('value', event.target.value);
      onAutosize(event.target);
    },
    [store, propsRef, onAutosize, isDisabled, isReadOnly],
  );

  const onKeyDown = React.useCallback(
    (event: React.KeyboardEvent<InputElement>) => {
      if (isDisabled || isReadOnly) return;
      propsRef.current.onKeyDown?.(event);
      if (event.defaultPrevented) return;

      if (event.key === 'Escape') {
        const nativeEvent = event.nativeEvent;
        context.onEscapeKeyDown?.(nativeEvent);
        if (!nativeEvent.defaultPrevented) context.onCancel();
      } else if (event.key === 'Enter') {
        context.onSubmit(value);
      }
    },
    [
      value,
      context.onSubmit,
      context.onCancel,
      context.onEscapeKeyDown,
      propsRef,
      isDisabled,
      isReadOnly,
    ],
  );

  useIsomorphicLayoutEffect(() => {
    if (!editing || isDisabled || isReadOnly || !inputRef.current) return;
    const frameId = window.requestAnimationFrame(() => {
      if (!inputRef.current) return;
      inputRef.current.focus();
      inputRef.current.select();
      onAutosize(inputRef.current);
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [editing, onAutosize, isDisabled, isReadOnly]);

  if (!editing && !isReadOnly) return null;
  const InputPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'input';

  return (
    <InputPrimitive
      aria-required={isRequired}
      aria-invalid={context.invalid}
      data-slot="editable-input"
      dir={context.dir}
      disabled={isDisabled}
      readOnly={isReadOnly}
      required={isRequired}
      {...inputProps}
      id={context.inputId}
      aria-labelledby={context.labelId}
      ref={composedRef}
      maxLength={maxLength}
      placeholder={context.placeholder}
      value={value}
      onBlur={onBlur}
      onChange={onChange}
      onKeyDown={onKeyDown}
      className={cn(
        'flex rounded-sm border border-input bg-transparent py-1 text-base shadow-xs transition-colors file:border-0 file:bg-transparent file:font-medium file:text-foreground file:text-sm placeholder:text-muted-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
        context.autosize ? 'w-auto' : 'w-full',
        className,
      )}
    />
  );
}
