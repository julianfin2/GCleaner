<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { 
  UserPlus, 
  ShieldAlert, 
  FileText, 
  LayoutDashboard,
  ListChecks,
  Mail,
  RefreshCw, 
  Share2,
  Users
} from "lucide-vue-next";
import AccountsView from "./components/AccountsView.vue";
import ConfirmDialog from "./components/ConfirmDialog.vue";
import ContactsCleanupView from "./components/ContactsCleanupView.vue";
import DeleteFilesView from "./components/DeleteFilesView.vue";
import LogsView from "./components/LogsView.vue";
import MailCleanupView from "./components/MailCleanupView.vue";
import OverviewView, { type OverviewRow } from "./components/OverviewView.vue";
import SharedRemoveView from "./components/SharedRemoveView.vue";
import TasksCleanupView from "./components/TasksCleanupView.vue";
import type {
  AuthorizedAccount,
  ContactGroupItem,
  ContactItem,
  DeleteMode,
  DriveFile,
  FileActionStatusPayload,
  MailCleanupCandidate,
  SharedDriveFile,
  TaskCleanupItem,
  TokenImportReport,
} from "./types";

// --- State ---
const currentTab = ref("accounts");
const authorizedAccountDetails = ref<AuthorizedAccount[]>([]);
const scanResults = ref<DriveFile[]>([]);
const sharedResults = ref<SharedDriveFile[]>([]);
const mailResults = ref<MailCleanupCandidate[]>([]);
const contactResults = ref<ContactItem[]>([]);
const contactGroupResults = ref<ContactGroupItem[]>([]);
const taskResults = ref<TaskCleanupItem[]>([]);
const logs = ref<string[]>([]);
const isScanning = ref(false);
const isCleaning = ref(false);
const isScanningShared = ref(false);
const isRemovingShared = ref(false);
const isScanningMail = ref(false);
const isCleaningMail = ref(false);
const isScanningContacts = ref(false);
const isDeletingContacts = ref(false);
const isScanningTasks = ref(false);
const isCleaningTasks = ref(false);
const isImportingTokens = ref(false);
const isClearingAccounts = ref(false);
const deletingAccount = ref<string | null>(null);
const hasScanned = ref(false);
const hasScannedShared = ref(false);
const hasScannedMail = ref(false);
const hasScannedContacts = ref(false);
const hasScannedTasks = ref(false);
const deleteMode = ref<DeleteMode>("trash");
const mailDeleteMode = ref<DeleteMode>("trash");
const mailQuery = ref("has:drive -from:me");
const accessTokensText = ref("");
const tokenImportMessage = ref<{ type: "success" | "error"; text: string } | null>(null);
const confirmDialog = ref({
  open: false,
  title: "",
  message: "",
  confirmText: "确定",
  cancelText: "取消",
  danger: true,
});
let confirmResolver: ((confirmed: boolean) => void) | null = null;
const authorizedAccounts = computed(() => authorizedAccountDetails.value.map((account) => account.email));
const pendingScanResults = computed(() => scanResults.value.filter((file) => file.cleanStatus !== "done"));
const pendingSharedResults = computed(() => sharedResults.value.filter((file) => file.cleanStatus !== "done"));
const pendingMailResults = computed(() => mailResults.value.filter((candidate) => candidate.cleanStatus !== "done"));
const pendingContactResults = computed(() => contactResults.value.filter((contact) => contact.cleanStatus !== "done"));
const pendingContactGroupResults = computed(() => contactGroupResults.value.filter((group) => group.cleanStatus !== "done"));
const pendingTaskResults = computed(() => taskResults.value.filter((item) => item.cleanStatus !== "done"));
const overviewStatus = (scanned: boolean, scanning: boolean, cleaning: boolean, pending: number) => {
  if (scanning) return "扫描中";
  if (cleaning) return "处理中";
  if (!scanned) return "未扫描";
  if (pending > 0) return "待处理";
  return "已完成";
};
const isAnyOverviewBusy = computed(() =>
  isScanning.value ||
  isCleaning.value ||
  isScanningShared.value ||
  isRemovingShared.value ||
  isScanningMail.value ||
  isCleaningMail.value ||
  isScanningContacts.value ||
  isDeletingContacts.value ||
  isScanningTasks.value ||
  isCleaningTasks.value
);
const overviewRows = computed<OverviewRow[]>(() => [
  {
    key: "files",
    title: "删除文件",
    summary: hasScanned.value
      ? `发现 ${scanResults.value.length} 个顶层文件或目录，待处理 ${pendingScanResults.value.length} 个`
      : "未扫描我的云端硬盘顶层文件和目录",
    status: overviewStatus(hasScanned.value, isScanning.value, isCleaning.value, pendingScanResults.value.length),
    pending: pendingScanResults.value.length,
    scanning: isScanning.value,
    cleaning: isCleaning.value,
    scanned: hasScanned.value,
  },
  {
    key: "shared",
    title: "移除共享",
    summary: hasScannedShared.value
      ? `发现 ${sharedResults.value.length} 个共享文件，待处理 ${pendingSharedResults.value.length} 个`
      : "未扫描“与我共享”中的直接权限",
    status: overviewStatus(hasScannedShared.value, isScanningShared.value, isRemovingShared.value, pendingSharedResults.value.length),
    pending: pendingSharedResults.value.length,
    scanning: isScanningShared.value,
    cleaning: isRemovingShared.value,
    scanned: hasScannedShared.value,
  },
  {
    key: "mail",
    title: "清理邮件",
    summary: hasScannedMail.value
      ? `发现 ${mailResults.value.length} 封带 Drive 线索的邮件，待处理 ${pendingMailResults.value.length} 封`
      : "未扫描 Gmail 文件线索邮件",
    status: overviewStatus(hasScannedMail.value, isScanningMail.value, isCleaningMail.value, pendingMailResults.value.length),
    pending: pendingMailResults.value.length,
    scanning: isScanningMail.value,
    cleaning: isCleaningMail.value,
    scanned: hasScannedMail.value,
  },
  {
    key: "contacts",
    title: "清理联系人",
    summary: hasScannedContacts.value
      ? `发现 ${contactResults.value.length} 个通讯录联系人、${contactGroupResults.value.length} 个自定义标签，待处理 ${pendingContactResults.value.length + pendingContactGroupResults.value.length} 项`
      : "未扫描通讯录联系人和自定义标签",
    status: overviewStatus(hasScannedContacts.value, isScanningContacts.value, isDeletingContacts.value, pendingContactResults.value.length + pendingContactGroupResults.value.length),
    pending: pendingContactResults.value.length + pendingContactGroupResults.value.length,
    scanning: isScanningContacts.value,
    cleaning: isDeletingContacts.value,
    scanned: hasScannedContacts.value,
  },
  {
    key: "tasks",
    title: "清理任务",
    summary: hasScannedTasks.value
      ? `发现 ${taskResults.value.length} 个任务清理项目，待处理 ${pendingTaskResults.value.length} 个`
      : "未扫描任务列表",
    status: overviewStatus(hasScannedTasks.value, isScanningTasks.value, isCleaningTasks.value, pendingTaskResults.value.length),
    pending: pendingTaskResults.value.length,
    scanning: isScanningTasks.value,
    cleaning: isCleaningTasks.value,
    scanned: hasScannedTasks.value,
  },
]);
const canCleanOverview = computed(() => overviewRows.value.some((row) => row.pending > 0));

// --- Methods ---
const fetchAccounts = async () => {
  authorizedAccountDetails.value = await invoke<AuthorizedAccount[]>("get_authorized_accounts");
};

const importTokens = async () => {
  const tokens = accessTokensText.value
    .split(/\r?\n/)
    .map((token) => token.trim())
    .filter(Boolean);
  if (!tokens.length) return;

  isImportingTokens.value = true;
  tokenImportMessage.value = null;
  try {
    const report = await invoke<TokenImportReport>("import_tokens", { tokens });
    if (report.failures.length === 0) {
      accessTokensText.value = "";
    }
    await fetchAccounts();
    const failureSummary = report.failures
      .map((failure) => `第 ${failure.line} 行 ${failure.preview}`)
      .join("；");
    tokenImportMessage.value = {
      type: report.failures.length ? "error" : "success",
      text: report.failures.length
        ? `已导入 ${report.imported.length} 个，${report.failures.length} 个失败：${failureSummary}`
        : `已导入 ${report.imported.length} 个 access token。`,
    };
    addLog(tokenImportMessage.value.text);
  } catch (err) {
    tokenImportMessage.value = {
      type: "error",
      text: "导入失败：" + err,
    };
    addLog("导入 token 失败: " + err);
  } finally {
    isImportingTokens.value = false;
  }
};

const deleteAccount = async (account: string) => {
  deletingAccount.value = account;
  try {
    await invoke("delete_authorized_account", { account });
    scanResults.value = scanResults.value.filter((file) => file.account !== account);
    sharedResults.value = sharedResults.value.filter((file) => file.account !== account);
    mailResults.value = mailResults.value.filter((candidate) => candidate.account !== account);
    contactResults.value = contactResults.value.filter((contact) => contact.account !== account);
    contactGroupResults.value = contactGroupResults.value.filter((group) => group.account !== account);
    taskResults.value = taskResults.value.filter((item) => item.account !== account);
    await fetchAccounts();
  } catch (err) {
    addLog("删除账号失败: " + err);
  } finally {
    deletingAccount.value = null;
  }
};

const clearAccounts = async () => {
  if (!authorizedAccounts.value.length) return;
  isClearingAccounts.value = true;
  try {
    const removed = await invoke<number>("clear_authorized_accounts");
    authorizedAccountDetails.value = [];
    scanResults.value = [];
    sharedResults.value = [];
    mailResults.value = [];
    contactResults.value = [];
    contactGroupResults.value = [];
    taskResults.value = [];
    if (removed === 0) {
      addLog("没有可清空的本地 token。");
    }
  } catch (err) {
    addLog("清空账号失败: " + err);
  } finally {
    isClearingAccounts.value = false;
  }
};

const startScan = async () => {
  if (authorizedAccounts.value.length === 0) return;
  isScanning.value = true;
  hasScanned.value = false;
  scanResults.value = [];
  try {
    scanResults.value = await invoke<DriveFile[]>("scan_drive", { accounts: authorizedAccounts.value });
    hasScanned.value = true;
  } catch (err) {
    addLog("扫描出错: " + err);
  } finally {
    isScanning.value = false;
  }
};

const startSharedScan = async () => {
  if (authorizedAccounts.value.length === 0) return;
  isScanningShared.value = true;
  hasScannedShared.value = false;
  sharedResults.value = [];
  try {
    sharedResults.value = await invoke<SharedDriveFile[]>("scan_shared_drive", { accounts: authorizedAccounts.value });
    hasScannedShared.value = true;
  } catch (err) {
    addLog("扫描共享文件出错: " + err);
  } finally {
    isScanningShared.value = false;
  }
};

const startMailScan = async () => {
  if (authorizedAccounts.value.length === 0) return;
  isScanningMail.value = true;
  hasScannedMail.value = false;
  mailResults.value = [];
  try {
    mailResults.value = await invoke<MailCleanupCandidate[]>("scan_mail_cleanup", {
      accounts: authorizedAccounts.value,
      query: mailQuery.value,
    });
    hasScannedMail.value = true;
  } catch (err) {
    addLog("扫描邮件出错: " + err);
  } finally {
    isScanningMail.value = false;
  }
};

const startContactsScan = async () => {
  if (authorizedAccounts.value.length === 0) return;
  isScanningContacts.value = true;
  hasScannedContacts.value = false;
  contactResults.value = [];
  contactGroupResults.value = [];
  try {
    const accounts = authorizedAccounts.value;
    const [contacts, groups] = await Promise.all([
      invoke<ContactItem[]>("scan_contacts_cleanup", { accounts }),
      invoke<ContactGroupItem[]>("scan_contact_groups_cleanup", { accounts }),
    ]);
    contactResults.value = contacts;
    contactGroupResults.value = groups;
    hasScannedContacts.value = true;
  } catch (err) {
    addLog("扫描联系人出错: " + err);
  } finally {
    isScanningContacts.value = false;
  }
};

const startTasksScan = async () => {
  if (authorizedAccounts.value.length === 0) return;
  isScanningTasks.value = true;
  hasScannedTasks.value = false;
  taskResults.value = [];
  try {
    taskResults.value = await invoke<TaskCleanupItem[]>("scan_tasks_cleanup", {
      accounts: authorizedAccounts.value,
    });
    hasScannedTasks.value = true;
  } catch (err) {
    addLog("扫描任务出错: " + err);
  } finally {
    isScanningTasks.value = false;
  }
};

const requestConfirmation = (options: {
  title: string;
  message: string;
  confirmText: string;
  cancelText?: string;
  danger?: boolean;
}) => {
  confirmDialog.value = {
    open: true,
    title: options.title,
    message: options.message,
    confirmText: options.confirmText,
    cancelText: options.cancelText || "取消",
    danger: options.danger ?? true,
  };

  return new Promise<boolean>((resolve) => {
    confirmResolver = resolve;
  });
};

const resolveConfirmation = (confirmed: boolean) => {
  confirmDialog.value.open = false;
  confirmResolver?.(confirmed);
  confirmResolver = null;
};

const clearPermissions = async (skipConfirm = false) => {
  const filesToClean = pendingScanResults.value;
  if (filesToClean.length === 0) return;
  const selectedMode = deleteMode.value;
  const action = modeLabel(selectedMode);
  if (!skipConfirm) {
    const confirmed = await requestConfirmation({
      title: `确认${action}`,
      message: `即将把当前扫描结果中的 ${filesToClean.length} 个待处理顶层文件或目录${action}。目录内的子文件会随目录一起处理。`,
      confirmText: action,
      danger: selectedMode === "permanent",
    });
    if (!confirmed) return;
  }

  isCleaning.value = true;
  try {
    for (const file of filesToClean) {
      file.deleteMode = selectedMode;
      file.cleanStatus = "cleaning";
    }

    await invoke("delete_files", {
      files: filesToClean,
      permanentlyDelete: selectedMode === "permanent",
    });
    addLog(`${action}流程已结束，结果已保留。可点击“开始扫描”重新确认。`);
  } catch (err) {
    for (const file of scanResults.value) {
      if (file.cleanStatus === "cleaning") {
        file.cleanStatus = "failed";
      }
    }
    addLog("删除出错: " + err);
  } finally {
    isCleaning.value = false;
  }
};

const removeSharedPermissions = async (skipConfirm = false) => {
  const filesToRemove = pendingSharedResults.value;
  if (filesToRemove.length === 0) return;
  if (!skipConfirm) {
    const confirmed = await requestConfirmation({
      title: "确认移除共享权限",
      message: `即将从 ${filesToRemove.length} 个待处理“与我共享”文件中移除当前账号的访问权限。移除后这些文件可能不再出现在该账号的“与我共享”中。`,
      confirmText: "移除权限",
      danger: true,
    });
    if (!confirmed) return;
  }

  isRemovingShared.value = true;
  try {
    for (const file of filesToRemove) {
      file.cleanStatus = "cleaning";
    }
    await invoke("remove_shared_permissions", { files: filesToRemove });
    addLog("共享权限移除流程已结束，结果已保留。可点击“开始扫描”重新确认。");
  } catch (err) {
    for (const file of sharedResults.value) {
      if (file.cleanStatus === "cleaning") {
        file.cleanStatus = "failed";
      }
    }
    addLog("移除共享权限出错: " + err);
  } finally {
    isRemovingShared.value = false;
  }
};

const cleanupMail = async (skipConfirm = false) => {
  const candidatesToClean = pendingMailResults.value;
  if (candidatesToClean.length === 0) return;
  const selectedMode = mailDeleteMode.value;
  const action = modeLabel(selectedMode);
  const removableCount = candidatesToClean.reduce((sum, item) => sum + item.removable_permissions, 0);
  if (!skipConfirm) {
    const confirmed = await requestConfirmation({
      title: "确认清理邮件",
      message: `即将处理 ${candidatesToClean.length} 封待处理邮件，移除 ${removableCount} 个可明确识别的 Drive 当前账号权限，并把这些邮件${action}。`,
      confirmText: `处理并${action}`,
      danger: true,
    });
    if (!confirmed) return;
  }

  isCleaningMail.value = true;
  try {
    for (const candidate of candidatesToClean) {
      candidate.deleteMode = selectedMode;
      candidate.cleanStatus = "cleaning";
    }
    await invoke("cleanup_mail_candidates", {
      candidates: candidatesToClean,
      permanentlyDelete: selectedMode === "permanent",
    });
    addLog("邮件清理流程已结束，结果已保留。可点击“开始扫描”重新确认。");
  } catch (err) {
    for (const candidate of mailResults.value) {
      if (candidate.cleanStatus === "cleaning") {
        candidate.cleanStatus = "failed";
      }
    }
    addLog("清理邮件出错: " + err);
  } finally {
    isCleaningMail.value = false;
  }
};

const deleteContacts = async (skipConfirm = false) => {
  const contactsToDelete = pendingContactResults.value;
  const groupsToDelete = pendingContactGroupResults.value;
  if (contactsToDelete.length === 0 && groupsToDelete.length === 0) return;
  if (!skipConfirm) {
    const confirmed = await requestConfirmation({
      title: "确认清理联系人",
      message: `即将删除当前扫描结果中的 ${contactsToDelete.length} 个通讯录联系人和 ${groupsToDelete.length} 个自定义标签。此操作不包含“其他联系人”。`,
      confirmText: "清理联系人",
      danger: true,
    });
    if (!confirmed) return;
  }

  isDeletingContacts.value = true;
  try {
    for (const contact of contactsToDelete) {
      contact.cleanStatus = "cleaning";
    }
    for (const group of groupsToDelete) {
      group.cleanStatus = "cleaning";
    }
    if (contactsToDelete.length > 0) {
      await invoke("delete_contacts_cleanup", { contacts: contactsToDelete });
    }
    if (groupsToDelete.length > 0) {
      await invoke("delete_contact_groups_cleanup", { groups: groupsToDelete });
    }
    addLog("通讯录联系人和标签清理流程已结束，结果已保留。可点击“开始扫描”重新确认。");
  } catch (err) {
    for (const contact of contactResults.value) {
      if (contact.cleanStatus === "cleaning") {
        contact.cleanStatus = "failed";
      }
    }
    for (const group of contactGroupResults.value) {
      if (group.cleanStatus === "cleaning") {
        group.cleanStatus = "failed";
      }
    }
    addLog("清理联系人出错: " + err);
  } finally {
    isDeletingContacts.value = false;
  }
};

const cleanupTasks = async (skipConfirm = false) => {
  const tasksToClean = pendingTaskResults.value;
  if (tasksToClean.length === 0) return;
  const defaultTaskCount = tasksToClean
    .filter((item) => item.is_default)
    .reduce((sum, item) => sum + item.task_count, 0);
  const listCount = tasksToClean.filter((item) => !item.is_default).length;
  if (!skipConfirm) {
    const confirmed = await requestConfirmation({
      title: "确认清理任务",
      message: `即将删除 ${listCount} 个用户创建的任务列表，并清空默认任务列表中的 ${defaultTaskCount} 个任务。Docs 或 Chat Spaces 分配任务可能会同步影响原始分配任务。`,
      confirmText: "清理任务",
      danger: true,
    });
    if (!confirmed) return;
  }

  isCleaningTasks.value = true;
  try {
    for (const item of tasksToClean) {
      item.cleanStatus = "cleaning";
    }
    await invoke("cleanup_tasks", { items: tasksToClean });
    addLog("任务清理流程已结束，结果已保留。可点击“开始扫描”重新确认。");
  } catch (err) {
    for (const item of taskResults.value) {
      if (item.cleanStatus === "cleaning") {
        item.cleanStatus = "failed";
      }
    }
    addLog("清理任务出错: " + err);
  } finally {
    isCleaningTasks.value = false;
  }
};

const runOverviewScan = async (key: string) => {
  if (key === "files") await startScan();
  if (key === "shared") await startSharedScan();
  if (key === "mail") await startMailScan();
  if (key === "contacts") await startContactsScan();
  if (key === "tasks") await startTasksScan();
};

const runOverviewClean = async (key: string, skipConfirm = false) => {
  if (key === "files") await clearPermissions(skipConfirm);
  if (key === "shared") await removeSharedPermissions(skipConfirm);
  if (key === "mail") await cleanupMail(skipConfirm);
  if (key === "contacts") await deleteContacts(skipConfirm);
  if (key === "tasks") await cleanupTasks(skipConfirm);
};

const scanAllOverview = async () => {
  if (authorizedAccounts.value.length === 0 || isAnyOverviewBusy.value) return;
  addLog("开始总览一键扫描。");
  for (const key of ["files", "shared", "mail", "contacts", "tasks"]) {
    await runOverviewScan(key);
  }
  addLog("总览一键扫描已完成。");
};

const cleanAllOverview = async () => {
  if (!canCleanOverview.value || isAnyOverviewBusy.value) return;
  const summary = overviewRows.value
    .filter((row) => row.pending > 0)
    .map((row) => `${row.title} ${row.pending} 项`)
    .join("，");
  const confirmed = await requestConfirmation({
    title: "确认一键清理全部",
    message: `即将按顺序处理：${summary}。文件和邮件会按当前选择的删除方式执行，各项失败不会阻断后续项目。`,
    confirmText: "一键清理全部",
    danger: true,
  });
  if (!confirmed) return;

  addLog("开始总览一键清理。");
  for (const key of ["files", "shared", "mail", "contacts", "tasks"]) {
    await runOverviewClean(key, true);
  }
  addLog("总览一键清理已完成。");
};

const tokenTimeLeftText = (account: AuthorizedAccount) => {
  if (!account.expires_at) return "未知有效期";
  const seconds = account.expires_at - Math.floor(Date.now() / 1000);
  if (seconds <= 0) return "已过期";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `剩余 ${minutes} 分钟`;
  return `剩余 ${Math.floor(minutes / 60)} 小时 ${minutes % 60} 分钟`;
};

const modeLabel = (mode: DeleteMode) => mode === "permanent" ? "永久删除" : "移入回收站";

const rowStatusText = (file: DriveFile) => {
  const mode = file.deleteMode ?? deleteMode.value;
  if (file.cleanStatus === "cleaning") {
    return mode === "permanent" ? "删除中" : "移入中";
  }
  if (file.cleanStatus === "done") {
    return mode === "permanent" ? "已删除" : "已移入";
  }
  if (file.cleanStatus === "failed") {
    return "失败";
  }
  return mode === "permanent" ? "待删除" : "待移入";
};

const sharedStatusText = (file: SharedDriveFile) => {
  if (file.cleanStatus === "cleaning") return "移除中";
  if (file.cleanStatus === "done") return "已移除";
  if (file.cleanStatus === "failed") return "失败";
  return "待移除";
};

const mailStatusText = (candidate: MailCleanupCandidate) => {
  const mode = candidate.deleteMode ?? mailDeleteMode.value;
  if (candidate.cleanStatus === "cleaning") {
    return mode === "permanent" ? "删除中" : "移入中";
  }
  if (candidate.cleanStatus === "done") {
    return mode === "permanent" ? "已删除" : "已移入";
  }
  if (candidate.cleanStatus === "failed") return "失败";
  return mode === "permanent" ? "待删除" : "待移入";
};

const contactStatusText = (contact: ContactItem) => {
  if (contact.cleanStatus === "cleaning") return "删除中";
  if (contact.cleanStatus === "done") return "已删除";
  if (contact.cleanStatus === "failed") return "失败";
  return "待删除";
};

const contactGroupStatusText = (group: ContactGroupItem) => {
  if (group.cleanStatus === "cleaning") return "删除中";
  if (group.cleanStatus === "done") return "已删除";
  if (group.cleanStatus === "failed") return "失败";
  return "待删除";
};

const taskStatusText = (item: TaskCleanupItem) => {
  if (item.cleanStatus === "cleaning") return item.is_default ? "清空中" : "删除中";
  if (item.cleanStatus === "done") return item.is_default ? "已清空" : "已删除";
  if (item.cleanStatus === "failed") return "失败";
  return item.is_default ? "待清空" : "待删除";
};

const addLog = (msg: string) => {
  const time = new Date().toLocaleTimeString();
  if (logs.value[0]?.endsWith(msg)) {
    return;
  }
  logs.value.unshift(`[${time}] ${msg}`);
};

// --- Lifecycle & Listeners ---
onMounted(async () => {
  await fetchAccounts();

  await listen<string>("log", (event) => {
    addLog(event.payload);
  });

  await listen<FileActionStatusPayload>("file-action-status", (event) => {
    const file = scanResults.value.find(
      (item) => item.id === event.payload.id && item.account === event.payload.account
    );
    if (file) {
      file.cleanStatus = event.payload.status;
    }
  });

  await listen<FileActionStatusPayload>("shared-file-action-status", (event) => {
    const file = sharedResults.value.find(
      (item) => item.id === event.payload.id && item.account === event.payload.account
    );
    if (file) {
      file.cleanStatus = event.payload.status;
    }
  });

  await listen<FileActionStatusPayload>("mail-cleanup-status", (event) => {
    const candidate = mailResults.value.find(
      (item) => item.id === event.payload.id && item.account === event.payload.account
    );
    if (candidate) {
      candidate.cleanStatus = event.payload.status;
    }
  });

  await listen<FileActionStatusPayload>("contact-action-status", (event) => {
    const contact = contactResults.value.find(
      (item) => item.id === event.payload.id && item.account === event.payload.account
    );
    if (contact) {
      contact.cleanStatus = event.payload.status;
    }
  });

  await listen<FileActionStatusPayload>("contact-group-action-status", (event) => {
    const group = contactGroupResults.value.find(
      (item) => item.id === event.payload.id && item.account === event.payload.account
    );
    if (group) {
      group.cleanStatus = event.payload.status;
    }
  });

  await listen<FileActionStatusPayload>("task-cleanup-status", (event) => {
    const item = taskResults.value.find(
      (task) => task.id === event.payload.id && task.account === event.payload.account
    );
    if (item) {
      item.cleanStatus = event.payload.status;
    }
  });
});
</script>

<template>
  <div class="app-container">
    <!-- Sidebar -->
    <aside class="sidebar">
      <div class="logo">
        <div class="logo-icon"><ShieldAlert :size="24" /></div>
        <span class="logo-text">谷歌账号清理器</span>
      </div>
      
      <nav class="nav">
        <button 
          :class="['nav-item', { active: currentTab === 'accounts' }]" 
          @click="currentTab = 'accounts'"
        >
          <UserPlus :size="20" />
          <span>账号授权</span>
        </button>
        <button 
          :class="['nav-item', { active: currentTab === 'overview' }]" 
          @click="currentTab = 'overview'"
        >
          <LayoutDashboard :size="20" />
          <span>项目总览</span>
        </button>
        <div class="nav-divider"></div>
        <button 
          :class="['nav-item', { active: currentTab === 'scan' }]" 
          @click="currentTab = 'scan'"
        >
          <RefreshCw :size="20" />
          <span>删除文件</span>
        </button>
        <button 
          :class="['nav-item', { active: currentTab === 'shared' }]" 
          @click="currentTab = 'shared'"
        >
          <Share2 :size="20" />
          <span>移除共享</span>
        </button>
        <button 
          :class="['nav-item', { active: currentTab === 'mail' }]" 
          @click="currentTab = 'mail'"
        >
          <Mail :size="20" />
          <span>清理邮件</span>
        </button>
        <button 
          :class="['nav-item', { active: currentTab === 'contacts' }]" 
          @click="currentTab = 'contacts'"
        >
          <Users :size="20" />
          <span>清理联系人</span>
        </button>
        <button 
          :class="['nav-item', { active: currentTab === 'tasks' }]" 
          @click="currentTab = 'tasks'"
        >
          <ListChecks :size="20" />
          <span>清理任务</span>
        </button>
        <div class="nav-divider"></div>
        <button 
          :class="['nav-item', { active: currentTab === 'logs' }]" 
          @click="currentTab = 'logs'"
        >
          <FileText :size="20" />
          <span>操作日志</span>
        </button>
      </nav>

      <div class="sidebar-footer">
        <div class="status-dot" :class="{ green: authorizedAccounts.length > 0 }"></div>
        <span>{{ authorizedAccounts.length ? 'Token 就绪' : '等待 token' }}</span>
      </div>
    </aside>

    <main class="content">
      <AccountsView
        v-if="currentTab === 'accounts'"
        v-model:access-tokens-text="accessTokensText"
        :authorized-accounts="authorizedAccountDetails"
        :is-importing-tokens="isImportingTokens"
        :is-clearing-accounts="isClearingAccounts"
        :deleting-account="deletingAccount"
        :import-message="tokenImportMessage"
        :token-time-left-text="tokenTimeLeftText"
        @import-tokens="importTokens"
        @clear-accounts="clearAccounts"
        @delete-account="deleteAccount"
      />

      <OverviewView
        v-if="currentTab === 'overview'"
        :rows="overviewRows"
        :authorized-accounts="authorizedAccounts"
        :is-busy="isAnyOverviewBusy"
        :can-clean-all="canCleanOverview"
        @scan-all="scanAllOverview"
        @clean-all="cleanAllOverview"
        @scan-item="runOverviewScan"
        @clean-item="runOverviewClean"
      />

      <DeleteFilesView
        v-if="currentTab === 'scan'"
        v-model:delete-mode="deleteMode"
        :authorized-accounts="authorizedAccounts"
        :scan-results="scanResults"
        :is-scanning="isScanning"
        :is-cleaning="isCleaning"
        :has-scanned="hasScanned"
        :has-pending-delete-files="pendingScanResults.length > 0"
        :mode-label="modeLabel"
        :row-status-text="rowStatusText"
        @start-scan="startScan"
        @delete-files="clearPermissions"
      />

      <SharedRemoveView
        v-if="currentTab === 'shared'"
        :authorized-accounts="authorizedAccounts"
        :shared-results="sharedResults"
        :is-scanning-shared="isScanningShared"
        :is-removing-shared="isRemovingShared"
        :has-scanned-shared="hasScannedShared"
        :has-pending-shared-files="pendingSharedResults.length > 0"
        :shared-status-text="sharedStatusText"
        @start-shared-scan="startSharedScan"
        @remove-shared-permissions="removeSharedPermissions"
      />

      <MailCleanupView
        v-if="currentTab === 'mail'"
        v-model:mail-query="mailQuery"
        v-model:mail-delete-mode="mailDeleteMode"
        :authorized-accounts="authorizedAccounts"
        :mail-results="mailResults"
        :is-scanning-mail="isScanningMail"
        :is-cleaning-mail="isCleaningMail"
        :has-scanned-mail="hasScannedMail"
        :has-pending-mail-candidates="pendingMailResults.length > 0"
        :mode-label="modeLabel"
        :mail-status-text="mailStatusText"
        @scan-mail="startMailScan"
        @cleanup-mail="cleanupMail"
      />

      <ContactsCleanupView
        v-if="currentTab === 'contacts'"
        :authorized-accounts="authorizedAccounts"
        :contact-results="contactResults"
        :contact-group-results="contactGroupResults"
        :is-scanning-contacts="isScanningContacts"
        :is-deleting-contacts="isDeletingContacts"
        :has-scanned-contacts="hasScannedContacts"
        :has-pending-contacts="pendingContactResults.length + pendingContactGroupResults.length > 0"
        :contact-status-text="contactStatusText"
        :contact-group-status-text="contactGroupStatusText"
        @scan-contacts="startContactsScan"
        @delete-contacts="deleteContacts"
      />

      <TasksCleanupView
        v-if="currentTab === 'tasks'"
        :authorized-accounts="authorizedAccounts"
        :task-results="taskResults"
        :is-scanning-tasks="isScanningTasks"
        :is-cleaning-tasks="isCleaningTasks"
        :has-scanned-tasks="hasScannedTasks"
        :has-pending-tasks="pendingTaskResults.length > 0"
        :task-status-text="taskStatusText"
        @scan-tasks="startTasksScan"
        @cleanup-tasks="cleanupTasks"
      />

      <LogsView v-if="currentTab === 'logs'" :logs="logs" />
    </main>

    <ConfirmDialog
      :open="confirmDialog.open"
      :title="confirmDialog.title"
      :message="confirmDialog.message"
      :confirm-text="confirmDialog.confirmText"
      :cancel-text="confirmDialog.cancelText"
      :danger="confirmDialog.danger"
      @confirm="resolveConfirmation(true)"
      @cancel="resolveConfirmation(false)"
    />
  </div>
</template>

<style>
:root {
  --primary-color: #007AFF;
  --primary-hover: #0063CC;
  --danger-color: #FF3B30;
  --bg-light: #FBFBFD;
  --sidebar-bg: #F8FAFD;
  --text-main: #1D1D1F;
  --text-sec: #86868B;
  --border-color: #E5E5E7;
  --card-shadow: 0 12px 30px rgba(0, 0, 0, 0.04);
  --radius-card: 24px;
  --radius-btn: 12px;
}

* {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
  background-color: var(--bg-light);
  color: var(--text-main);
  overflow: hidden;
}

.app-container {
  display: flex;
  height: 100vh;
}

/* Sidebar Styles */
.sidebar {
  width: 260px;
  background-color: var(--sidebar-bg);
  border-right: 1px solid #E9EFF6;
  display: flex;
  flex-direction: column;
  padding: 32px 16px;
}

.logo {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 16px 40px;
}

.logo-icon {
  background: var(--primary-color);
  color: white;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo-text {
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.5px;
}

.nav {
  flex: 1;
}

.nav-item {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border: none;
  background: transparent;
  color: var(--text-sec);
  font-size: 15px;
  font-weight: 500;
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.2s;
  margin-bottom: 4px;
}

.nav-item:hover {
  background: rgba(0, 0, 0, 0.03);
  color: var(--text-main);
}

.nav-item.active {
  background: white;
  color: var(--primary-color);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.03);
}

.nav-divider {
  height: 1px;
  margin: 10px 12px;
  background: #E3E8F0;
}

.sidebar-footer {
  padding: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-sec);
}

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #C7C7CC;
}

.status-dot.green {
  background: #34C759;
  box-shadow: 0 0 8px rgba(52, 199, 89, 0.4);
}

/* Content Styles */
.content {
  flex: 1;
  padding: 40px 60px;
  overflow-y: auto;
}

.pane-header {
  margin-bottom: 32px;
}

.pane-header h1 {
  font-size: 28px;
  font-weight: 700;
  margin-bottom: 4px;
}

.pane-header p {
  color: var(--text-sec);
}

.header-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.btns {
  display: flex;
  align-items: center;
  gap: 12px;
}

/* Cards & Grid */
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
}

.card {
  background: white;
  border-radius: var(--radius-card);
  padding: 32px;
  box-shadow: var(--card-shadow);
  display: flex;
  flex-direction: column;
}

.card.glass {
  background: rgba(255, 255, 255, 0.7);
  backdrop-filter: blur(10px);
}

.card h3 {
  font-size: 18px;
  margin-bottom: 8px;
}

.card-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.card-title-row h3 {
  margin-bottom: 0;
}

.card p {
  font-size: 14px;
  color: var(--text-sec);
}

.card-icon {
  color: var(--primary-color);
  margin-bottom: 16px;
}

/* Buttons */
.btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 20px;
  border-radius: var(--radius-btn);
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  border: none;
}

.btn-primary {
  background: var(--primary-color);
  color: white;
}

.btn-primary:hover {
  background: var(--primary-hover);
  transform: translateY(-1px);
}

.btn-outline {
  background: white;
  border: 1px solid var(--border-color);
  color: var(--text-main);
}

.btn-danger {
  background: var(--danger-color);
  color: white;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.delete-mode-control {
  display: inline-flex;
  align-items: center;
  gap: 10px;
}

.mode-label {
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
}

.segmented-control {
  display: inline-flex;
  padding: 3px;
  background: #EEF2F7;
  border: 1px solid var(--border-color);
  border-radius: 10px;
}

.segment-btn {
  border: none;
  background: transparent;
  color: var(--text-sec);
  border-radius: 7px;
  padding: 7px 10px;
  font-size: 13px;
  font-weight: 700;
  cursor: pointer;
}

.segment-btn.active {
  background: white;
  color: var(--primary-color);
  box-shadow: 0 1px 4px rgba(0, 0, 0, 0.08);
}

.segment-btn.danger.active {
  color: var(--danger-color);
}

.segment-btn:disabled {
  cursor: not-allowed;
  opacity: 0.5;
}

/* Lists & Items */
.url-list, .account-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.url-item, .account-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: rgba(0, 0, 0, 0.02);
  border-radius: 12px;
}

.url-item.used {
  color: var(--text-sec);
  background: rgba(52, 199, 89, 0.08);
}

.account-item {
  gap: 12px;
}

.account-name {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
}

.account-name span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.input-group {
  display: flex;
  gap: 8px;
}

input[type="number"] {
  width: 96px;
  padding: 8px;
  border-radius: 8px;
  border: 1px solid var(--border-color);
  outline: none;
}

.icon-btn {
  background: transparent;
  border: none;
  cursor: pointer;
  color: var(--primary-color);
  padding: 4px;
}

.icon-btn:disabled {
  color: var(--text-sec);
  cursor: not-allowed;
  opacity: 0.5;
}

.icon-btn.danger {
  color: var(--danger-color);
}

/* Table */
.results-table-wrapper {
  background: white;
  border-radius: var(--radius-card);
  box-shadow: var(--card-shadow);
  overflow: hidden;
}

.results-table {
  width: 100%;
  border-collapse: collapse;
}

.results-table th {
  text-align: left;
  padding: 16px 24px;
  background: #F9FAFB;
  font-size: 13px;
  color: var(--text-sec);
  text-transform: uppercase;
}

.results-table td {
  padding: 16px 24px;
  border-top: 1px solid #F3F4F6;
  font-size: 14px;
}

.file-name {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 500;
}

.row-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  min-width: 76px;
  color: var(--text-sec);
  font-size: 13px;
  font-weight: 600;
}

.row-status.cleaning {
  color: var(--primary-color);
}

.row-status.done {
  color: #34C759;
}

.row-status.failed {
  color: var(--danger-color);
}

/* Logs */
.log-console {
  background: #1D1D1F;
  color: #00FF00;
  font-family: 'JetBrains Mono', monospace;
  padding: 24px;
  border-radius: 16px;
  height: 400px;
  overflow-y: auto;
  font-size: 13px;
  line-height: 1.6;
}

.log-line {
  margin-bottom: 4px;
  opacity: 0.9;
}

.spin {
  animation: spin 1s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.mt-4 { margin-top: 16px; }
.mt-6 { margin-top: 24px; }
.text-green { color: #34C759; }
.text-sec { color: var(--text-sec); }
.clickable { cursor: pointer; }
.empty-state { text-align: center; color: var(--text-sec); padding: 20px; }
.empty-card { 
  display: flex; flex-direction: column; align-items: center; 
  padding: 60px; background: white; border-radius: 24px; gap: 16px;
}
.empty-card.success {
  border: 1px solid rgba(52, 199, 89, 0.18);
}
.loading-state {
  display: flex; flex-direction: column; align-items: center;
  padding: 60px; gap: 16px; color: var(--text-sec);
}
</style>
