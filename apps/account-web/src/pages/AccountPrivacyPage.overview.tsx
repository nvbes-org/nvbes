import { Cookie, FileCheck2, ShieldCheck } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Item, ItemContent, ItemGroup, ItemMedia, ItemTitle } from '@/components/ui/item';
import type { PrivacyOverview } from './AccountPrivacyPage.presentation';

export function PrivacyOverviewCard({ overview }: { overview: PrivacyOverview }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h2>Votre protection en un coup d'œil</h2>
        </CardTitle>
        <CardDescription>
          Les réglages appliqués aujourd'hui sur ce navigateur et les choix enregistrés sur votre
          compte.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <ItemGroup className="grid gap-3 md:grid-cols-3">
          <Item variant="muted">
            <ItemMedia variant="icon">
              <Cookie />
            </ItemMedia>
            <ItemContent>
              <ItemTitle>Cookies optionnels</ItemTitle>
              <Badge variant="secondary">
                {overview.optionalEnabledCount} sur {overview.optionalTotal} actifs
              </Badge>
            </ItemContent>
          </Item>
          <Item variant="muted">
            <ItemMedia variant="icon">
              <FileCheck2 />
            </ItemMedia>
            <ItemContent>
              <ItemTitle>Choix enregistrés</ItemTitle>
              <Badge variant="secondary">
                {overview.recordedChoiceCount} preuve
                {overview.recordedChoiceCount === 1 ? '' : 's'}
              </Badge>
            </ItemContent>
          </Item>
          <Item variant="muted">
            <ItemMedia variant="icon">
              <ShieldCheck />
            </ItemMedia>
            <ItemContent>
              <ItemTitle>Signal navigateur</ItemTitle>
              <Badge variant={overview.gpcProtected ? 'default' : 'outline'}>
                {overview.gpcProtected ? 'GPC respecté' : 'Protection standard'}
              </Badge>
            </ItemContent>
          </Item>
        </ItemGroup>
      </CardContent>
    </Card>
  );
}
