'use client';

import * as SlotPrimitive from '@radix-ui/react-slot';
import * as React from 'react';
import { useAsRef } from '@/hooks/use-as-ref';
import { cn } from '@/lib/utils';
import { useEditableContext, useEditableStore } from './editable.context';

export interface EditableTriggerProps extends React.ComponentProps<'button'> {
  asChild?: boolean;
  forceMount?: boolean;
}

export function EditableTrigger(props: EditableTriggerProps) {
  const { asChild, forceMount = false, ref, ...triggerProps } = props;
  const context = useEditableContext('EditableTrigger');
  const editing = useEditableStore((state) => state.editing);
  const onTrigger = React.useCallback(() => {
    if (!context.disabled && !context.readOnly) context.onEdit();
  }, [context.disabled, context.readOnly, context.onEdit]);

  if (!forceMount && (editing || context.readOnly)) return null;
  const TriggerPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'button';

  return (
    <TriggerPrimitive
      type="button"
      aria-controls={context.rootId}
      aria-disabled={context.disabled || context.readOnly}
      data-disabled={context.disabled ? '' : undefined}
      data-readonly={context.readOnly ? '' : undefined}
      data-slot="editable-trigger"
      {...triggerProps}
      ref={ref}
      onClick={context.triggerMode === 'click' ? onTrigger : undefined}
      onDoubleClick={context.triggerMode === 'dblclick' ? onTrigger : undefined}
    />
  );
}

export interface EditableToolbarProps extends React.ComponentProps<'div'> {
  asChild?: boolean;
  orientation?: 'horizontal' | 'vertical';
}

export function EditableToolbar(props: EditableToolbarProps) {
  const { asChild, className, orientation = 'horizontal', ref, ...toolbarProps } = props;
  const context = useEditableContext('EditableToolbar');
  const ToolbarPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'div';

  return (
    <ToolbarPrimitive
      role="toolbar"
      aria-controls={context.rootId}
      aria-orientation={orientation}
      data-slot="editable-toolbar"
      dir={context.dir}
      {...toolbarProps}
      ref={ref}
      className={cn('flex items-center gap-2', orientation === 'vertical' && 'flex-col', className)}
    />
  );
}

export interface EditableCancelProps extends React.ComponentProps<'button'> {
  asChild?: boolean;
}

export function EditableCancel(props: EditableCancelProps) {
  const { onClick: onClickProp, asChild, ref, ...cancelProps } = props;
  const context = useEditableContext('EditableCancel');
  const editing = useEditableStore((state) => state.editing);
  const propsRef = useAsRef({ onClick: onClickProp });
  const onClick = React.useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      if (context.disabled || context.readOnly) return;
      propsRef.current.onClick?.(event);
      if (!event.defaultPrevented) context.onCancel();
    },
    [propsRef, context.onCancel, context.disabled, context.readOnly],
  );

  if (!editing && !context.readOnly) return null;
  const CancelPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'button';

  return (
    <CancelPrimitive
      type="button"
      aria-controls={context.rootId}
      data-slot="editable-cancel"
      {...cancelProps}
      onClick={onClick}
      ref={ref}
    />
  );
}

export interface EditableSubmitProps extends React.ComponentProps<'button'> {
  asChild?: boolean;
}

export function EditableSubmit(props: EditableSubmitProps) {
  const { onClick: onClickProp, asChild, ref, ...submitProps } = props;
  const context = useEditableContext('EditableSubmit');
  const value = useEditableStore((state) => state.value);
  const editing = useEditableStore((state) => state.editing);
  const propsRef = useAsRef({ onClick: onClickProp });
  const onClick = React.useCallback(
    (event: React.MouseEvent<HTMLButtonElement>) => {
      if (context.disabled || context.readOnly) return;
      propsRef.current.onClick?.(event);
      if (!event.defaultPrevented) context.onSubmit(value);
    },
    [propsRef, context.onSubmit, value, context.disabled, context.readOnly],
  );

  if (!editing && !context.readOnly) return null;
  const SubmitPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'button';

  return (
    <SubmitPrimitive
      type="button"
      aria-controls={context.rootId}
      data-slot="editable-submit"
      {...submitProps}
      ref={ref}
      onClick={onClick}
    />
  );
}
