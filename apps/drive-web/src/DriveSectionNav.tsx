import {
  Building2,
  CreditCard,
  FileKey2,
  FolderOpen,
  Link2,
  ShieldCheck,
  Trash2,
  UserCircle2,
  Users2,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export type DriveModuleId = 'drive' | 'sharing' | 'admin' | 'account';

export type DriveSectionId =
  | 'files'
  | 'trash'
  | 'shared-links'
  | 'members'
  | 'security'
  | 'billing'
  | 'api'
  | 'account';

type DriveSection = {
  id: DriveSectionId;
  moduleId: DriveModuleId;
  label: string;
  description: string;
  icon: LucideIcon;
};

export const DRIVE_SECTIONS: DriveSection[] = [
  {
    id: 'files',
    moduleId: 'drive',
    label: 'Fichiers',
    description: 'Bibliotheque active',
    icon: FolderOpen,
  },
  {
    id: 'trash',
    moduleId: 'drive',
    label: 'Corbeille',
    description: 'Elements supprimes',
    icon: Trash2,
  },
  {
    id: 'shared-links',
    moduleId: 'sharing',
    label: 'Liens partages',
    description: 'Acces externes',
    icon: Link2,
  },
  {
    id: 'members',
    moduleId: 'admin',
    label: 'Membres',
    description: 'Roles et invitations',
    icon: Users2,
  },
  {
    id: 'security',
    moduleId: 'admin',
    label: 'Securite',
    description: 'Journal et controles',
    icon: ShieldCheck,
  },
  {
    id: 'billing',
    moduleId: 'admin',
    label: 'Facturation',
    description: 'Plan et capacite',
    icon: CreditCard,
  },
  {
    id: 'api',
    moduleId: 'admin',
    label: 'API',
    description: 'Cles et integrations',
    icon: FileKey2,
  },
  {
    id: 'account',
    moduleId: 'account',
    label: 'Compte',
    description: 'Profil utilisateur',
    icon: UserCircle2,
  },
];

export function firstSectionForModule(moduleId: DriveModuleId): DriveSectionId {
  return DRIVE_SECTIONS.find((section) => section.moduleId === moduleId)?.id ?? 'files';
}

export function labelForSection(sectionId: DriveSectionId): string {
  return DRIVE_SECTIONS.find((section) => section.id === sectionId)?.label ?? 'Fichiers';
}

export function DriveSectionNav({
  activeModule,
  activeSection,
  onSectionChange,
}: {
  activeModule: DriveModuleId;
  activeSection: DriveSectionId;
  onSectionChange: (sectionId: DriveSectionId) => void;
}) {
  const visibleSections = DRIVE_SECTIONS.filter((section) => section.moduleId === activeModule);

  return (
    <aside className="border-border/70 bg-background/95 md:w-64 md:border-r">
      <div className="flex gap-2 overflow-x-auto p-3 md:flex-col md:p-4">
        {visibleSections.map((section) => {
          const Icon = section.icon;
          const isActive = section.id === activeSection;

          return (
            <Button
              key={section.id}
              type="button"
              variant={isActive ? 'secondary' : 'ghost'}
              className={cn(
                'h-auto min-w-44 justify-start gap-3 rounded-xl px-3 py-3 text-left md:min-w-0',
                isActive && 'bg-muted text-foreground shadow-sm',
              )}
              onClick={() => onSectionChange(section.id)}
            >
              <Icon className="size-4" aria-hidden="true" />
              <span className="grid gap-0.5">
                <span className="text-sm font-medium">{section.label}</span>
                <span className="text-xs font-normal text-muted-foreground">{section.description}</span>
              </span>
            </Button>
          );
        })}
      </div>
      <div className="hidden border-t p-4 text-xs text-muted-foreground md:block">
        <Building2 className="mb-2 size-4" aria-hidden="true" />
        Navigation contextualisee par espace.
      </div>
    </aside>
  );
}
