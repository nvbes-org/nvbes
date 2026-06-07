import { LandingPageHeader } from './LandingPage.header';
import { LandingPageHero } from './LandingPage.hero';
import {
  LandingPageCapabilities,
  LandingPageFooter,
  LandingPagePricing,
  LandingPageSecurity,
  LandingPageWorkflow,
} from './LandingPage.sections';

export default function LandingPage() {
  return (
    <main className="min-h-screen bg-white text-zinc-950">
      <LandingPageHeader />
      <LandingPageHero />
      <LandingPageCapabilities />
      <LandingPageWorkflow />
      <LandingPageSecurity />
      <LandingPagePricing />
      <LandingPageFooter />
    </main>
  );
}
