export type DeleteMode = "trash" | "permanent";

export interface DriveFile {
  id: string;
  name: string;
  mime_type: string;
  account: string;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
  deleteMode?: DeleteMode;
}

export interface SharedDriveFile {
  id: string;
  name: string;
  mime_type: string;
  account: string;
  owner: string;
  permission_id: string;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
}

export interface MailDriveFile {
  id: string;
  name: string;
  mime_type: string;
  permission_id?: string;
  owner: string;
  reason: string;
  removable: boolean;
  resource_key?: string;
}

export interface MailCleanupCandidate {
  id: string;
  thread_id: string;
  account: string;
  subject: string;
  from: string;
  date: string;
  snippet: string;
  drive_links: MailDriveFile[];
  removable_permissions: number;
  ignored_files: number;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
  deleteMode?: DeleteMode;
}

export interface ContactItem {
  id: string;
  resource_name: string;
  display_name: string;
  email: string;
  phone: string;
  account: string;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
}

export interface ContactGroupItem {
  id: string;
  resource_name: string;
  name: string;
  member_count: number;
  account: string;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
}

export interface TaskCleanupItem {
  id: string;
  task_list_id: string;
  title: string;
  account: string;
  is_default: boolean;
  task_count: number;
  cleanStatus?: "idle" | "cleaning" | "done" | "failed";
}

export interface AuthorizedAccount {
  email: string;
  expires_at?: number;
  scopes: string[];
}

export interface TokenImportFailure {
  line: number;
  preview: string;
  error: string;
}

export interface TokenImportReport {
  imported: AuthorizedAccount[];
  failures: TokenImportFailure[];
}

export interface FileActionStatusPayload {
  id: string;
  account: string;
  status: "cleaning" | "done" | "failed";
}
