import { Link } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';
import { ProductMockup } from './LandingPage.product';

export function LandingPageHero() {
  return (
    <section className="mx-auto grid max-w-[88rem] gap-12 px-6 pb-12 pt-14 min-[1320px]:grid-cols-[0.65fr_1fr] lg:px-10">
      <div className="self-center">
        <h1 className="max-w-xl text-5xl font-semibold leading-[1.08] tracking-normal text-zinc-950 md:text-6xl">
          Identity and files for teams that ship securely
        </h1>
        <p className="mt-7 max-w-md text-lg leading-8 text-zinc-600">
          nvbes gives growing teams one control plane for access, audit trails, billing, and private
          workspace storage.
        </p>
        <div className="mt-10 flex flex-wrap gap-4">
          <Button asChild className="h-12 bg-teal-700 px-8 text-base hover:bg-teal-800">
            <Link to="/register">Start workspace</Link>
          </Button>
          <Button
            asChild
            className="h-12 border-teal-700 px-8 text-base text-teal-800"
            variant="outline"
          >
            <a href="#docs">View docs</a>
          </Button>
        </div>
      </div>
      <ProductMockup />
    </section>
  );
}
