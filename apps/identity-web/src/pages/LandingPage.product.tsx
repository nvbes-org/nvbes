import { ProductMockupContent } from './LandingPage.product.content';
import { ProductMockupSidebar } from './LandingPage.product.sidebar';

export function ProductMockup() {
  return (
    <div className="min-w-0 overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-[0_24px_70px_rgba(15,23,42,0.12)]">
      <div className="grid min-h-[620px] lg:grid-cols-[176px_1fr]">
        <ProductMockupSidebar />
        <ProductMockupContent />
      </div>
    </div>
  );
}
