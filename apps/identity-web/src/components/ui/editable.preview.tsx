'use client';

import * as SlotPrimitive from '@radix-ui/react-slot';
import * as React from 'react';
import { useAsRef } from '@/hooks/use-as-ref';
import { cn } from '@/lib/utils';
import { useEditableContext, useEditableStore } from './editable.context';

type PreviewElement = React.ComponentRef<'div'>;

export interface EditableLabelProps extends React.ComponentProps<'label'> {
  asChild?: boolean;
}

export function EditableLabel(props: EditableLabelProps) {
  const { asChild, className, children, ref, ...labelProps } = props;
  const context = useEditableContext('EditableLabel');
  const LabelPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'label';

  return (
    <LabelPrimitive
      data-disabled={context.disabled ? '' : undefined}
      data-invalid={context.invalid ? '' : undefined}
      data-required={context.required ? '' : undefined}
      data-slot="editable-label"
      {...labelProps}
      ref={ref}
      id={context.labelId}
      htmlFor={context.inputId}
      className={cn(
        "font-medium text-sm leading-none peer-disabled:cursor-not-allowed peer-disabled:opacity-70 data-required:after:ml-0.5 data-required:after:text-destructive data-required:after:content-['*']",
        className,
      )}
    >
      {children}
    </LabelPrimitive>
  );
}

export interface EditableAreaProps extends React.ComponentProps<'div'> {
  asChild?: boolean;
}

export function EditableArea(props: EditableAreaProps) {
  const { asChild, className, ref, ...areaProps } = props;
  const context = useEditableContext('EditableArea');
  const editing = useEditableStore((state) => state.editing);
  const AreaPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'div';

  return (
    <AreaPrimitive
      role="group"
      data-disabled={context.disabled ? '' : undefined}
      data-editing={editing ? '' : undefined}
      data-slot="editable-area"
      dir={context.dir}
      {...areaProps}
      ref={ref}
      className={cn(
        'relative inline-block min-w-0 data-disabled:cursor-not-allowed data-disabled:opacity-50',
        className,
      )}
    />
  );
}

export interface EditablePreviewProps extends React.ComponentProps<'div'> {
  asChild?: boolean;
}

export function EditablePreview(props: EditablePreviewProps) {
  const {
    onClick: onClickProp,
    onDoubleClick: onDoubleClickProp,
    onFocus: onFocusProp,
    onKeyDown: onKeyDownProp,
    asChild,
    className,
    ref,
    ...previewProps
  } = props;
  const context = useEditableContext('EditablePreview');
  const value = useEditableStore((state) => state.value);
  const editing = useEditableStore((state) => state.editing);
  const propsRef = useAsRef({
    onClick: onClickProp,
    onDoubleClick: onDoubleClickProp,
    onFocus: onFocusProp,
    onKeyDown: onKeyDownProp,
  });

  const onTrigger = React.useCallback(() => {
    if (!context.disabled && !context.readOnly) context.onEdit();
  }, [context.onEdit, context.disabled, context.readOnly]);

  const onClick = React.useCallback(
    (event: React.MouseEvent<PreviewElement>) => {
      propsRef.current.onClick?.(event);
      if (!event.defaultPrevented && context.triggerMode === 'click') onTrigger();
    },
    [propsRef, onTrigger, context.triggerMode],
  );

  const onDoubleClick = React.useCallback(
    (event: React.MouseEvent<PreviewElement>) => {
      propsRef.current.onDoubleClick?.(event);
      if (!event.defaultPrevented && context.triggerMode === 'dblclick') onTrigger();
    },
    [propsRef, onTrigger, context.triggerMode],
  );

  const onFocus = React.useCallback(
    (event: React.FocusEvent<PreviewElement>) => {
      propsRef.current.onFocus?.(event);
      if (!event.defaultPrevented && context.triggerMode === 'focus') onTrigger();
    },
    [propsRef, onTrigger, context.triggerMode],
  );

  const onKeyDown = React.useCallback(
    (event: React.KeyboardEvent<PreviewElement>) => {
      propsRef.current.onKeyDown?.(event);
      if (event.defaultPrevented || event.key !== 'Enter') return;

      const nativeEvent = event.nativeEvent;
      context.onEnterKeyDown?.(nativeEvent);
      if (!nativeEvent.defaultPrevented) onTrigger();
    },
    [propsRef, onTrigger, context.onEnterKeyDown],
  );

  if (editing || context.readOnly) return null;
  const PreviewPrimitive: React.ElementType = asChild ? SlotPrimitive.Slot : 'div';

  return (
    <PreviewPrimitive
      role="button"
      aria-disabled={context.disabled || context.readOnly}
      data-empty={!value ? '' : undefined}
      data-disabled={context.disabled ? '' : undefined}
      data-readonly={context.readOnly ? '' : undefined}
      data-slot="editable-preview"
      tabIndex={context.disabled || context.readOnly ? undefined : 0}
      {...previewProps}
      ref={ref}
      onClick={onClick}
      onDoubleClick={onDoubleClick}
      onFocus={onFocus}
      onKeyDown={onKeyDown}
      className={cn(
        'cursor-text truncate rounded-sm border border-transparent py-1 text-base focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-ring data-disabled:cursor-not-allowed data-readonly:cursor-default data-empty:text-muted-foreground data-disabled:opacity-50 md:text-sm',
        className,
      )}
    >
      {value || context.placeholder}
    </PreviewPrimitive>
  );
}
