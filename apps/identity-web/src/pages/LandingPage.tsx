import { LandingPageHeader } from './LandingPage.header';
import { LandingPageHero } from './LandingPage.hero';
import {
  ShadcnblocksAnnouncement,
  TailarkStatsSection,
  TailwindadminProgressPanel,
} from './LandingPage.registry';
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
      <ShadcnblocksAnnouncement />
      <LandingPageHeader />
      <LandingPageHero />
      <TailarkStatsSection />
      <LandingPageCapabilities />
      <TailwindadminProgressPanel />
      <LandingPageWorkflow />
      <LandingPageSecurity />
      <LandingPagePricing />
      <LandingPageFooter />
    </main>
  );
}
