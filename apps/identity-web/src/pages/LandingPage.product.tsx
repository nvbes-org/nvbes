import { Card, CardContent } from '@/components/ui/card';
import { ProductMockupContent } from './LandingPage.product.content';
import { ProductMockupSidebar } from './LandingPage.product.sidebar';

export function ProductMockup() {
  return (
    <Card className="min-w-0 overflow-hidden border-zinc-200 bg-white p-0 shadow-[0_24px_70px_rgba(15,23,42,0.12)]">
      <CardContent className="grid min-h-[620px] p-0 lg:grid-cols-[176px_1fr]">
        <ProductMockupSidebar />
        <ProductMockupContent />
      </CardContent>
    </Card>
  );
}
