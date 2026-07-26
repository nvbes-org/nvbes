import { BoxIcon, CircleOffIcon, ShieldIcon, UsersIcon } from 'lucide-react';
import { AuthBrandWordmark } from './AuthBrandWordmark';

export function RegisterBrandPanel() {
  return (
    <div className="relative hidden w-[45%] overflow-hidden lg:block">
      <div className="absolute inset-0 bg-gradient-to-br from-primary/12 via-primary/6 to-background" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_30%_40%,hsl(var(--primary)/0.12),transparent_70%)]" />
      <div className="absolute inset-0 bg-[radial-gradient(ellipse_at_80%_80%,hsl(var(--primary)/0.06),transparent_60%)]" />
      <div
        className="absolute inset-0 opacity-[0.03]"
        style={{
          backgroundImage:
            "url(\"data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.5'/%3E%3C/svg%3E\")",
        }}
      />
      <div className="relative z-10 flex h-full flex-col justify-between p-12 xl:p-16">
        <div className="animate-fade-slide-up">
          <div className="mb-16 inline-flex size-10 items-center justify-center rounded-xl bg-primary/15 text-primary">
            <CircleOffIcon className="size-5 stroke-[2.5]" aria-hidden="true" />
          </div>
          <AuthBrandWordmark />
          <p className="mt-4 max-w-xs text-base leading-relaxed text-muted-foreground">
            Votre cloud, sans dispersion, prêt à grandir avec votre équipe.
          </p>
          <div className="mt-12 flex flex-col gap-4">
            <div className="flex items-start gap-3 animate-fade-slide-up [animation-delay:300ms]">
              <div className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md bg-primary/10">
                <ShieldIcon className="size-3.5 text-primary" />
              </div>
              <p className="text-sm text-muted-foreground/80">
                Reprenez la main sur vos accès sans ralentir le travail.
              </p>
            </div>
            <div className="flex items-start gap-3 animate-fade-slide-up [animation-delay:400ms]">
              <div className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md bg-primary/10">
                <UsersIcon className="size-3.5 text-primary" />
              </div>
              <p className="text-sm text-muted-foreground/80">
                Rassemblez vos équipes, vos fichiers et vos usages au même endroit.
              </p>
            </div>
            <div className="flex items-start gap-3 animate-fade-slide-up [animation-delay:500ms]">
              <div className="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-md bg-primary/10">
                <BoxIcon className="size-3.5 text-primary" />
              </div>
              <p className="text-sm text-muted-foreground/80">
                Construisez sur un socle clair, durable, et vraiment maîtrisé.
              </p>
            </div>
          </div>
        </div>
        <div className="animate-fade-slide-up [animation-delay:200ms]">
          <p className="text-xs text-muted-foreground/50">
            &copy; {new Date().getFullYear()} nvbes
          </p>
        </div>
      </div>
    </div>
  );
}
