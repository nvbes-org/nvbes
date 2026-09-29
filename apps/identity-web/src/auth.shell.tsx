import type { ReactNode, Ref } from 'react';
import { AuthPageShell } from './components/AuthPageShell';
import { AuthBrandWordmark } from './components/AuthBrandWordmark';

/** Visual shell restored from archive/apps/identity-web, with active flow content. */
export function AuthShell({
  title,
  description,
  headingRef,
  children,
}: {
  title: string;
  description?: ReactNode;
  headingRef?: Ref<HTMLHeadingElement>;
  children: ReactNode;
}) {
  return (
    <AuthPageShell
      brand={
        <div className="flex h-full flex-col">
          <AuthBrandWordmark as="div" className="text-3xl" />
          <div className="mt-10 max-w-sm lg:mt-14">
            <h1
              ref={headingRef}
              tabIndex={-1}
              className="font-heading text-4xl leading-tight font-medium tracking-tight text-balance outline-none"
            >
              {title}
            </h1>
            {description && (
              <p className="mt-4 text-base leading-7 text-muted-foreground text-pretty">
                {description}
              </p>
            )}
          </div>
        </div>
      }
      footerLinks={null}
    >
      <div className="flex min-w-0 flex-col gap-6">{children}</div>
    </AuthPageShell>
  );
}
