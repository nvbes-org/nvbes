export type DriveModuleId = 'drive' | 'sharing' | 'shared-with-me' | 'starred' | 'trash';

export type DriveEntryKind = 'folder' | 'document' | 'image' | 'video' | 'archive' | 'spreadsheet';

export type DriveEntryStatus = 'active' | 'trashed';

export type DriveShareStatus = 'private' | 'shared' | 'revoked' | 'expired';

export type DriveRole = 'owner' | 'admin' | 'member' | 'viewer';

export type DriveMemberStatus = 'active' | 'invited' | 'suspended';

export type DriveViewMode = 'grid' | 'list';

export type DriveSortKey = 'name' | 'updated' | 'size' | 'kind';

export type DriveEntry = {
  id: string;
  parentId: string | null;
  name: string;
  kind: DriveEntryKind;
  status: DriveEntryStatus;
  shareStatus: DriveShareStatus;
  sizeBytes: number;
  ownerId: string;
  createdAt: string;
  updatedAt: string;
  mimeType?: string;
  starred: boolean;
  sharedWithCount: number;
};

export type DriveShareLink = {
  id: string;
  entryId: string;
  token: string;
  label: string;
  status: DriveShareStatus;
  permission: 'view' | 'edit';
  createdAt: string;
  expiresAt: string | null;
  accessCount: number;
  lastAccessedAt: string | null;
};

export type DriveMember = {
  id: string;
  name: string;
  email: string;
  role: DriveRole;
  status: DriveMemberStatus;
  joinedAt: string | null;
  invitedAt: string | null;
};

export type DriveSecurityEvent = {
  id: string;
  actorId: string;
  action: string;
  target: string;
  severity: 'info' | 'warning' | 'critical';
  createdAt: string;
};

export type DriveBillingState = {
  plan: 'free' | 'team' | 'enterprise';
  status: 'trialing' | 'active' | 'past_due' | 'canceled';
  seatsUsed: number;
  seatsIncluded: number;
  storageUsedBytes: number;
  storageLimitBytes: number;
  renewalDate: string;
};

export type DriveApiKey = {
  id: string;
  label: string;
  prefix: string;
  status: 'active' | 'revoked';
  createdAt: string;
  lastUsedAt: string | null;
  revokedAt: string | null;
};

export type DriveDetailsSelection =
  | { type: 'entry'; id: string }
  | { type: 'member'; id: string }
  | null;

export type DriveWorkspaceState = {
  activeModuleId: DriveModuleId;
  currentFolderId: string | null;
  query: string;
  viewMode: DriveViewMode;
  sort: DriveSortKey;
  selectedEntryIds: string[];
  detailsSelection: DriveDetailsSelection;
  entries: DriveEntry[];
  shareLinks: DriveShareLink[];
  members: DriveMember[];
  securityEvents: DriveSecurityEvent[];
  billing: DriveBillingState;
  apiKeys: DriveApiKey[];
};
