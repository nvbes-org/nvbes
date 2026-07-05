import type { ComponentType } from 'react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

type DetailTabId = 'access' | 'audit' | 'billing' | 'security' | 'usage';

type DetailLink = {
  hash: string;
  label: string;
};

type DetailInsight = {
  label: string;
  tone?: 'danger' | 'default';
  value: string;
};

export type DetailTab = {
  action: DetailLink;
  description: string;
  icon: ComponentType<{ className?: string }>;
  id: DetailTabId;
  insights: DetailInsight[];
  secondaryAction?: DetailLink;
  title: string;
};

export function EntityDetailTabs({ tabs }: { tabs: DetailTab[] }) {
  const [activeTab, setActiveTab] = useState<DetailTabId>(tabs[0]?.id ?? 'audit');
  const selectedTab = tabs.find((tab) => tab.id === activeTab) ?? tabs[0];

  if (!selectedTab) return null;

  return (
    <div className="rounded-md border" data-testid="entity-detail-tabs">
      <div
        aria-label="Entity detail sections"
        className="border-border flex gap-1 overflow-x-auto border-b p-2"
        role="tablist"
      >
        {tabs.map((tab) => (
          <button
            aria-selected={tab.id === selectedTab.id}
            className={cn(
              'text-muted-foreground hover:bg-muted/70 hover:text-foreground flex h-9 shrink-0 items-center gap-2 rounded-md px-3 text-xs font-medium',
              tab.id === selectedTab.id && 'bg-muted text-foreground',
            )}
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            role="tab"
            type="button"
          >
            <tab.icon className="size-3.5" />
            {tab.title}
          </button>
        ))}
      </div>
      <div className="grid gap-4 p-3 lg:grid-cols-[1fr_auto]">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-medium">
            <selectedTab.icon className="text-muted-foreground size-4" />
            {selectedTab.title}
          </div>
          <p className="text-muted-foreground mt-1 text-sm">{selectedTab.description}</p>
          <div className="mt-3 grid gap-2 sm:grid-cols-3">
            {selectedTab.insights.map((insight) => (
              <div className="bg-muted/30 rounded-md border p-3" key={insight.label}>
                <p className="text-muted-foreground text-xs">{insight.label}</p>
                <p
                  className={cn(
                    'mt-1 text-sm font-semibold',
                    insight.tone === 'danger' && 'text-destructive',
                  )}
                >
                  {insight.value}
                </p>
              </div>
            ))}
          </div>
        </div>
        <div className="flex flex-wrap items-start gap-2 lg:flex-col">
          <Button onClick={() => openHash(selectedTab.action.hash)} size="sm" type="button">
            {selectedTab.action.label}
          </Button>
          {selectedTab.secondaryAction ? (
            <Button
              onClick={() => openHash(selectedTab.secondaryAction?.hash ?? '')}
              size="sm"
              type="button"
              variant="outline"
            >
              {selectedTab.secondaryAction.label}
            </Button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function openHash(hash: string) {
  window.location.hash = hash;
}
