'use client';

import * as React from 'react';

import { cn } from '@/lib/classnames';
import { useSidebar } from './sidebar.context';
import { DesktopSidebar } from './sidebar.shell.desktop';
import { MobileSidebar } from './sidebar.shell.mobile';

export function Sidebar({
  side = 'left',
  variant = 'sidebar',
  collapsible = 'offcanvas',
  className,
  children,
  dir,
  ...props
}: React.ComponentProps<'div'> & {
  side?: 'left' | 'right';
  variant?: 'sidebar' | 'floating' | 'inset';
  collapsible?: 'offcanvas' | 'icon' | 'none';
}) {
  const { isMobile, state, openMobile, setOpenMobile } = useSidebar();

  if (collapsible === 'none') {
    return (
      <div
        data-slot="sidebar"
        className={cn(
          'flex h-full w-(--sidebar-width) flex-col bg-sidebar text-sidebar-foreground',
          className,
        )}
        {...props}
      >
        {children}
      </div>
    );
  }

  if (isMobile) {
    return (
      <MobileSidebar
        side={side}
        dir={dir}
        open={openMobile}
        onOpenChange={setOpenMobile}
        className={className}
        {...props}
      >
        {children}
      </MobileSidebar>
    );
  }

  return (
    <DesktopSidebar
      side={side}
      variant={variant}
      collapsible={collapsible}
      state={state}
      className={className}
      {...props}
    >
      {children}
    </DesktopSidebar>
  );
}

export function SidebarInset({ className, ...props }: React.ComponentProps<'main'>) {
  return (
    <main
      data-slot="sidebar-inset"
      className={cn(
        'relative flex w-full flex-1 flex-col bg-background md:peer-data-[variant=inset]:m-2 md:peer-data-[variant=inset]:ml-0 md:peer-data-[variant=inset]:rounded-xl md:peer-data-[variant=inset]:shadow-sm md:peer-data-[variant=inset]:peer-data-[state=collapsed]:ml-2',
        className,
      )}
      {...props}
    />
  );
}
