import { ArrowRight } from 'lucide-react';
import { Link } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { CheckList } from './LandingPage.ui';

export function LandingPagePricing() {
  return (
    <section className="mx-auto max-w-7xl px-6 pb-14 lg:px-10" id="pricing">
      <Card className="bg-teal-800 text-white">
        <CardContent className="grid gap-8 px-8 py-9 md:grid-cols-[1.4fr_0.6fr_1fr_0.8fr] md:items-center">
          <div>
            <h2 className="text-3xl font-semibold tracking-normal">
              Simple pricing that scales with your team
            </h2>
            <p className="mt-3 text-sm text-teal-50">Start free, upgrade when you&apos;re ready.</p>
          </div>
          <div>
            <div className="text-5xl font-medium">$20</div>
            <div className="mt-2 text-sm text-teal-50">
              Per user / month
              <br />
              Billed monthly
            </div>
          </div>
          <CheckList
            tone="dark"
            items={[
              'All platform features',
              '10 GB storage per user',
              'SSO, MFA, and audit logs',
              'Community support',
            ]}
          />
          <div className="space-y-4">
            <Button asChild className="h-12 w-full bg-white text-zinc-950 hover:bg-zinc-100">
              <Link to="/register">Start workspace</Link>
            </Button>
            <a
              className="flex items-center justify-center gap-2 text-sm font-medium text-white"
              href="mailto:sales@nvbes.com"
            >
              Contact sales <ArrowRight className="size-4" />
            </a>
          </div>
        </CardContent>
      </Card>
    </section>
  );
}
