export const auditEvents = [
  ['Sam Dovis uploaded specs.pdf', '10:12 AM'],
  ['Maya Patel granted access to Vendors', '9:58 AM'],
  ['Alex Kim downloaded pricing.xlsx', '9:41 AM'],
  ['Sam Dovis deleted changelog.md', '9:15 AM'],
] as const;

export const metrics = [
  ['Active sessions', '24', 'Users online'],
  ['Workspaces', '8', '3.2 TB used'],
  ['Audit events', '1,284', 'Last 24 hours'],
  ['Billing', '$2,450.00', 'Next invoice May 1'],
] as const;

export const files = [
  ['design', '-', 'Apr 28, 10:12 AM'],
  ['docs', '-', 'Apr 28, 9:41 AM'],
  ['roadmap.md', '14 KB', 'Apr 28, 9:20 AM'],
  ['specs.pdf', '1.2 MB', 'Apr 28, 9:15 AM'],
  ['pricing.xlsx', '28 KB', 'Apr 27, 4:08 PM'],
] as const;

export const navItems = [
  'Overview',
  'Workspaces',
  'Drive',
  'Access',
  'Audit',
  'Billing',
  'Settings',
] as const;

export const policyGroups = ['Acme Corp members', 'Design team', 'Vendors'] as const;
