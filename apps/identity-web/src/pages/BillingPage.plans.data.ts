export interface Plan {
  code: string;
  name: string;
  price: string;
  period: string;
  features: string[];
  featured?: boolean;
}

export const availablePlans: Plan[] = [
  {
    code: 'solo_pro',
    name: 'Solo Pro',
    price: '15',
    period: '/mois',
    features: ['1 utilisateur', '100 Go de stockage', 'Support par email', 'API Access'],
  },
  {
    code: 'team',
    name: 'Team',
    price: '39',
    period: '/mois',
    features: [
      "Jusqu'a 10 utilisateurs",
      '500 Go de stockage',
      'Support prioritaire',
      'SSO & SAML',
      'Audit logs',
    ],
    featured: true,
  },
  {
    code: 'team_plus',
    name: 'Team Plus',
    price: '79',
    period: '/mois',
    features: [
      'Utilisateurs illimites',
      '2 To de stockage',
      'Support dedie 24/7',
      'SSO, SAML, SCIM',
      'Audit logs avances',
      'SLA 99.9%',
    ],
  },
];
