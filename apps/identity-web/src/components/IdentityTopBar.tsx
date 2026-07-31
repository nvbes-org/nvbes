import { Link } from '@tanstack/react-router';
import { Menu } from 'lucide-react';

import { IdentitySidebar } from '@/components/IdentitySidebar';
import { Button } from '@/components/ui/button';
import { Sheet, SheetContent, SheetTrigger } from '@/components/ui/sheet';

export function IdentityTopBar() {
  return (
    <header className="sticky top-0 z-40 flex h-14 shrink-0 items-center bg-background/95 px-4 backdrop-blur supports-[backdrop-filter]:bg-background/80">
      <div className="mx-auto flex w-full items-center gap-3">
        <div className="shrink-0 md:hidden">
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon-sm">
                <Menu className="size-4" />
                <span className="sr-only">Ouvrir le menu Identity</span>
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-64 p-0">
              <IdentitySidebar />
            </SheetContent>
          </Sheet>
        </div>
        <Link to="/security" className="flex min-w-0 items-center gap-2.5">
          <span className="truncate text-2xl font-light tracking-tight text-foreground">
            <span className="font-semibold">nvbes</span> Identity
          </span>
        </Link>
      </div>
    </header>
  );
}
