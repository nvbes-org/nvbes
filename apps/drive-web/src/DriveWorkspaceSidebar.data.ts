import {
  Clock3,
  CreditCard,
  FileText,
  KeyRound,
  Link2,
  Shield,
  Star,
  Trash2,
  User,
  Users,
} from 'lucide-react';

export const mainNavigation = [
  { label: 'Mes fichiers', icon: FileText, active: true },
  { label: 'Recents', icon: Clock3 },
  { label: 'Favoris', icon: Star },
  { label: 'Partages', icon: Link2 },
  { label: 'Corbeille', icon: Trash2 },
];

export const settingsNavigation = [
  { label: 'Membres', icon: Users },
  { label: 'Facturation', icon: CreditCard },
  { label: 'Securite', icon: Shield },
  { label: 'API', icon: KeyRound },
  { label: 'Compte', icon: User },
];
