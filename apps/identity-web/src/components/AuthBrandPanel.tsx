import { AuthBrandWordmark } from '@/pages/AuthBrandWordmark';

export function AuthBrandPanel({ title, description }: { title: string; description: string }) {
  return (
    <div className="flex h-full flex-col">
      <AuthBrandWordmark as="div" className="text-3xl" />
      <div className="mt-10 max-w-sm lg:mt-14">
        <h1 className="font-heading text-4xl leading-tight font-medium tracking-tight text-balance">
          {title}
        </h1>
        <p className="mt-4 text-base leading-7 text-muted-foreground text-pretty">{description}</p>
      </div>
    </div>
  );
}
