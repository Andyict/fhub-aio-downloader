<script lang="ts">
  import { onMount } from "svelte";
  type SettingsTab = "overview" | "accounts" | "activity";
  type UpdateStatus = { current_version: string; current_commit?: string | null; latest_commit?: string | null; latest_commit_url?: string | null; update_available: boolean; updater_available: boolean; image: string; container: string; message: string; };
  type UiLanguage = "vi" | "en";
  type UiMode = "modern" | "classic";

  type AppSettings = {
    max_concurrent_downloads: number;
    download_directory: string;
    segments_per_download: number;
    fshare_configured: boolean;
    tmdb_configured: boolean;
    server_host: string;
    server_port: number;
  };

  type AppUser = {
    id: string;
    username: string;
    role: "admin" | "user";
    is_active: boolean;
    last_login_at?: string | null;
  };

  const tabOrder: SettingsTab[] = ["overview", "accounts", "activity"];

  const tabIcons: Record<SettingsTab, string> = {
    overview: "dashboard",
    accounts: "group",
    activity: "receipt_long",
  };

  const labels = {
    vi: {
      tabs: { overview: "Tổng quan", accounts: "Tài khoản", activity: "Nhật ký" },
      fshareLogin: "Đăng nhập FShare",
      fshareAccount: "Tài khoản FShare",
      localAccount: "Quản lý người dùng",
      usersTitle: "Người dùng",
      createUser: "Tạo người dùng",
      signOut: "Đăng xuất",
      createAccount: "Tạo tài khoản",
      adminOrUser: "Quản trị hoặc người dùng mới",
      createHint: "Chọn quyền phù hợp rồi tạo tài khoản nội bộ cho FHUB.",
      username: "Tên đăng nhập",
      password: "Mật khẩu",
      role: "Quyền",
      cancel: "Hủy",
      creating: "Đang tạo...",
      submitCreate: "Tạo tài khoản",
      uiModeTitle: "Chế độ sử dụng",
      uiModeHint: "",
      modernMode: "Chế độ Hub",
      classicMode: "Chế độ tải",
    },
    en: {
      tabs: { overview: "Overview", accounts: "Accounts", activity: "Activity" },
      fshareLogin: "FShare Login",
      fshareAccount: "FShare Account",
      localAccount: "User Manager",
      usersTitle: "Users",
      createUser: "Create user",
      signOut: "Sign out",
      createAccount: "Create account",
      adminOrUser: "New Admin or User",
      createHint: "Choose the right role and create a local FHub account.",
      username: "Username",
      password: "Password",
      role: "Role",
      cancel: "Cancel",
      creating: "Creating...",
      submitCreate: "Create account",
      uiModeTitle: "Usage mode",
      uiModeHint: "",
      modernMode: "Hub Mode",
      classicMode: "Download Mode",
    },
  } as const;


  const favoriteItems = [
    { title: "Oppenheimer", note: "IMAX collection", poster: "https://image.tmdb.org/t/p/w500/8Gxv8gSFCU0XGDykEGv7zR1n2ua.jpg" },
    { title: "Dune: Part Two", note: "Sci-fi night", poster: "https://image.tmdb.org/t/p/w500/1pdfLvkbY9ohJlCjQH2CZjjYVvJ.jpg" },
    { title: "The Dark Knight", note: "Family favorite", poster: "https://image.tmdb.org/t/p/w500/qJ2tW6WMUDux911r6m7haRef0WH.jpg" },
  ];

  const historyItems = [
    { time: "08:42", action: "Đã mở phim", title: "Dune: Part Two", detail: "Từ thư viện local" },
    { time: "08:31", action: "Đã tải xong", title: "Interstellar", detail: "FShare · 24.8 GB" },
    { time: "08:18", action: "Đã tìm kiếm", title: "Oppenheimer 4K", detail: "Discover query" },
    { time: "07:56", action: "Đã thêm yêu thích", title: "The Dark Knight", detail: "Movie card" },
  ];

  const services = [
    { id: "fshare", name: "FShare", label: "Nguồn tải chính", icon: "cloud_done", accent: "cyan" },
  ];

  const auditTrail = [
    { time: "08:36", text: "Admin console được làm mới ở chế độ UI preview." },
    { time: "08:31", text: "Luồng tải đồng thời đã đặt ở mức tối ưu cho mạng nội bộ." },
    { time: "08:24", text: "Tài khoản family_user sẵn sàng để gán quyền truy cập." },
  ];

  const downloadPresets = [
    { id: "safe", label: "Ổn định", hint: "Ít lỗi nhất", slots: 3, segments: 12 },
    { id: "balanced", label: "Cân bằng", hint: "Khuyên dùng", slots: 4, segments: 24 },
    { id: "speed", label: "Nhanh nhất", hint: "Ưu tiên tốc độ", slots: 6, segments: 32 },
  ] as const;

  let activeTab = $state<SettingsTab>("overview");
  let uiLanguage = $state<UiLanguage>("vi");
  let uiMode = $state<UiMode>("modern");
  let t = $derived(labels[uiLanguage]);
  let accessChecked = $state(false);
  let isAdminUser = $state(false);
  let settings = $state<AppSettings>({
    max_concurrent_downloads: 8,
    download_directory: "/downloads",
    segments_per_download: 8,
    fshare_configured: false,
    tmdb_configured: false,
    server_host: "0.0.0.0",
    server_port: 8484,
  });
  let maxConcurrent = $state(8);
  let downloadDirectory = $state("/downloads");
  let segmentsPerDownload = $state(8);
  let fshareAccountEmail = $state("");
  let fshareAccountRank = $state("");
  let fshareEmail = $state("");
  let fsharePassword = $state("");
  let showFsharePassword = $state(false);
  let fshareSaving = $state(false);
  let fshareLoginMessage = $state("");
  let fsharePanelOpen = $state(false);
  let downloadAdvancedOpen = $state(false);
  let autoTrackIntervalSecs = $state(3600);
  let autoTrackSaving = $state(false);
  let autoTrackSavedMessage = $state("");
  let status = $state("Đang tải cấu hình thật từ FHUB...");
  let saving = $state(false);
  let userSaving = $state(false);
  let selectedUserId = $state<string | null>(null);
  let passwordDraft = $state("");
  let showAdminModal = $state(false);
  let newUsername = $state("");
  let newPassword = $state("");
  let newRole = $state<"admin" | "user">("user");
  let users = $state<AppUser[]>([]);
  let updateStatus = $state<UpdateStatus | null>(null);
  let checkingUpdate = $state(false);
  let updatingApp = $state(false);
  let updateReloadScheduled = $state(false);
  let updateClickLock = false;
  let updateMessage = $state("");
  let updateCommandCopied = $state(false);
  let showUpdateConfirm = $state(false);


  const activeUsers = $derived(users.filter((user) => user.is_active).length);
  const adminUsers = $derived(users.filter((user) => user.role === "admin").length);
  const fshareIsVip = $derived(["VIP", "PREMIUM", "VIP ACCOUNT"].includes((fshareAccountRank || "").toUpperCase().trim()));
  const activeDownloadPreset = $derived(downloadPresets.find((preset) => preset.slots === Number(maxConcurrent) && preset.segments === Number(segmentsPerDownload))?.id ?? "custom");
  const shouldShowUpdateBanner = $derived(Boolean(updateStatus?.update_available && updateStatus?.latest_commit && updateStatus?.current_commit !== updateStatus?.latest_commit));
  const updateCurrentLabel = $derived(updateStatus?.current_commit || (uiLanguage === "vi" ? "bản local" : "local build"));

  onMount(() => {
    syncLanguage();
    syncUiMode();
    const onLanguageChange = (event: Event) => {
      const next = (event as CustomEvent<UiLanguage>).detail;
      if (next === "vi" || next === "en") uiLanguage = next;
    };
    window.addEventListener("fhub-language-change", onLanguageChange);
    void (async () => {
      const allowed = await verifyAdminAccess();
      if (!allowed) return;
      const params = new URLSearchParams(window.location.search);
      if (params.get("adminPopup") === "1") {
        activeTab = "accounts";
        newRole = "admin";
        showAdminModal = true;
      }
      await refreshAll();
      await checkUpdateStatus();
    })();
    return () => window.removeEventListener("fhub-language-change", onLanguageChange);
  });

  function syncLanguage() {
    try {
      const saved = localStorage.getItem("fhub-ui-language");
      if (saved === "vi" || saved === "en") uiLanguage = saved;
    } catch {
      // ignore
    }
  }

  function syncUiMode() {
    try {
      const saved = localStorage.getItem("fhub-ui-mode");
      if (saved === "modern" || saved === "classic") uiMode = saved;
    } catch {
      // ignore
    }
  }

  function setUiMode(mode: UiMode) {
    uiMode = mode;
    try {
      localStorage.setItem("fhub-ui-mode", mode);
    } catch {
      // ignore
    }
    window.dispatchEvent(new CustomEvent("fhub-ui-mode-change", { detail: mode }));
    status = uiLanguage === "vi"
      ? (mode === "classic" ? "Đã chuyển sang chế độ tải." : "Đã chuyển sang chế độ Hub.")
      : (mode === "classic" ? "Switched to Download Mode." : "Switched to Hub Mode.");
  }

  async function verifyAdminAccess() {
    try {
      const response = await fetch("/api/auth/me", { credentials: "include" });
      const payload = response.ok ? await response.json() : null;
      isAdminUser = payload?.user?.role === "admin";
      if (!isAdminUser) {
        status = "Tab Setting chỉ dành cho tài khoản admin.";
        window.location.replace("/downloads");
        return false;
      }
      return true;
    } catch {
      status = "Không xác thực được quyền admin.";
      window.location.replace("/downloads");
      return false;
    } finally {
      accessChecked = true;
    }
  }

  async function refreshAll() {
    if (!isAdminUser) return;
    try {
      const [settingsResponse, downloadsResponse, accountsResponse, healthResponse, usersResponse, autoTrackResponse] = await Promise.all([
        fetch("/api/settings"),
        fetch("/api/settings/downloads"),
        fetch("/api/accounts"),
        fetch("/api/health"),
        fetch("/api/auth/users", { credentials: "include" }),
        fetch("/api/settings/auto-track", { credentials: "include" }),
      ]);

      const appSettings = settingsResponse.ok ? await settingsResponse.json() : null;
      const downloadSettings = downloadsResponse.ok ? await downloadsResponse.json() : appSettings?.downloads;
      const accountPayload = accountsResponse.ok ? await accountsResponse.json() : { accounts: [] };
      const accountList = Array.isArray(accountPayload) ? accountPayload : (accountPayload.accounts || []);
      const health = healthResponse.ok ? await healthResponse.json() : null;
      const usersPayload = usersResponse.ok ? await usersResponse.json() : { users: [] };
      const autoTrackSettings = autoTrackResponse.ok ? await autoTrackResponse.json() : null;
      users = Array.isArray(usersPayload) ? usersPayload : (usersPayload.users || []);
      autoTrackIntervalSecs = Number(autoTrackSettings?.check_interval_secs || autoTrackIntervalSecs);

      settings = {
        max_concurrent_downloads: downloadSettings?.max_concurrent ?? appSettings?.downloads?.max_concurrent ?? settings.max_concurrent_downloads,
        download_directory: downloadSettings?.directory ?? appSettings?.downloads?.directory ?? settings.download_directory,
        segments_per_download: downloadSettings?.segments_per_download ?? appSettings?.downloads?.segments_per_download ?? settings.segments_per_download,
        fshare_configured: accountList.length > 0,
        tmdb_configured: Boolean(health?.tmdb_configured ?? health?.integrations?.tmdb ?? false),
        server_host: appSettings?.server?.host ?? settings.server_host,
        server_port: appSettings?.server?.port ?? settings.server_port,
      };
      maxConcurrent = settings.max_concurrent_downloads;
      downloadDirectory = settings.download_directory;
      segmentsPerDownload = settings.segments_per_download;
      fshareAccountEmail = accountList[0]?.email || "";
      fshareAccountRank = accountList[0]?.rank || "";
      if (!fshareEmail && fshareAccountEmail) fshareEmail = fshareAccountEmail;
      status = "Đã đồng bộ cấu hình thật từ FHUB.";
    } catch (error) {
      const message = error instanceof Error ? error.message : "Không tải được cấu hình";
      status = `Không tải được cấu hình FHUB: ${message}`;
    }
  }


  function isFshareWrongPassword(message: string) {
    return /wrong password|invalid credential|invalid credentials|authenticate fail|login failed|invalid credentials|sai mật khẩu|mật khẩu sai|invalid credentials|invalid credentials|invalid credentials/i.test(message || "");
  }

  function normalizeFshareError(message: string) {
    return isFshareWrongPassword(message)
      ? "Sai mật khẩu FShare. Kiểm tra lại rồi đăng nhập lại."
      : (message || "Đăng nhập FShare thất bại");
  }


  async function checkUpdateStatus() {
    if (!isAdminUser) return;
    checkingUpdate = true;
    try {
      const response = await fetch("/api/update/status", { credentials: "include" });
      if (!response.ok) throw new Error(await response.text());
      updateStatus = await response.json();
      updateMessage = updateStatus?.message || "Đã kiểm tra cập nhật.";
    } catch (error) {
      updateMessage = error instanceof Error ? error.message : "Không kiểm tra được cập nhật.";
    } finally {
      checkingUpdate = false;
    }
  }

  async function runWebUpdate(event?: Event) {
    event?.preventDefault();
    event?.stopPropagation();
    if (updatingApp || updateClickLock) return;
    updateClickLock = true;
    updateMessage = "Sẵn sàng cập nhật FHub.";
    if (!updateStatus?.updater_available) {
      await checkUpdateStatus();
    }
    if (!updateStatus?.updater_available) {
      updateMessage = "Chưa bật quyền update trong web. Cần mount /var/run/docker.sock vào container FHub rồi restart một lần.";
      updateClickLock = false;
      return;
    }
    showUpdateConfirm = true;
    updateClickLock = false;
  }

  async function submitUpdateForm(event: SubmitEvent) {
    event.preventDefault();
    event.stopPropagation();
    await runWebUpdate(event);
  }

  function scheduleUpdateReload() {
    updateReloadScheduled = true;
    window.setTimeout(() => window.location.reload(), 60_000);
  }

  async function confirmWebUpdate() {
    showUpdateConfirm = true;
    updatingApp = true;
    updateReloadScheduled = false;
    updateMessage = "Đang update FHub... Giữ nguyên trang này, hệ thống sẽ tự làm mới sau 1 phút.";
    try {
      const response = await fetch("/api/update/run", { method: "POST", credentials: "include" });
      const result = response.ok ? await response.json() : { success: false, message: await response.text() };
      const rawMessage = result.message || (result.success ? "Đã bắt đầu cập nhật." : "Cập nhật thất bại.");
      if (result.success) {
        updateMessage = "Đã gửi lệnh update. FHub đang kéo bản mới và khởi động lại; trang sẽ tự làm mới sau 1 phút.";
        scheduleUpdateReload();
        return;
      }
      updateMessage = /permission denied|docker socket|var\/run\/docker\.sock/i.test(rawMessage)
        ? "Chưa có quyền update Docker. Cần mount /var/run/docker.sock vào container FHub rồi restart một lần."
        : rawMessage;
    } catch (error) {
      updateMessage = "Đã gửi lệnh update. Nếu FHub đang khởi động lại, trang sẽ tự làm mới sau 1 phút.";
      scheduleUpdateReload();
      return;
    } finally {
      if (!updateReloadScheduled) {
        updatingApp = false;
        updateClickLock = false;
      }
    }
  }

  async function saveFshareAccount() {
    if (!fshareEmail.trim() || !fsharePassword) {
      fshareLoginMessage = "Nhập email/username và mật khẩu FShare trước đã.";
      status = fshareLoginMessage;
      return;
    }

    fshareSaving = true;
    fshareLoginMessage = "Đang gửi tài khoản FShare lên FHUB...";
    status = fshareLoginMessage;
    try {
      const response = await fetch("/api/accounts", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email: fshareEmail.trim(), password: fsharePassword }),
      });
      const result = response.ok ? await response.json() : { success: false, message: await response.text() };
      const resultMessage = result.message || result.error || "";
      if (isFshareWrongPassword(resultMessage)) {
        throw new Error(normalizeFshareError(resultMessage));
      }
      if (!response.ok || result.success === false) {
        const authHint = response.status === 401 ? "Bạn cần đăng nhập FHUB trước khi lưu tài khoản FShare." : "";
        throw new Error([authHint, normalizeFshareError(resultMessage)].filter(Boolean).join(" ") || "Đăng nhập FShare thất bại");
      }
      fsharePassword = "";
      fshareLoginMessage = resultMessage || "Đã lưu tài khoản FShare.";
      status = fshareLoginMessage;
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Đăng nhập FShare thất bại";
      fshareLoginMessage = message;
      status = message;
    } finally {
      fshareSaving = false;
    }
  }

  async function logoutFshareAccount() {
    if (!fshareAccountEmail) return;

    fshareSaving = true;
    fshareLoginMessage = "Đang đăng xuất FShare...";
    status = fshareLoginMessage;
    try {
      const response = await fetch(`/api/accounts/${encodeURIComponent(fshareAccountEmail)}`, { method: "DELETE" });
      const result = response.ok ? await response.json() : { success: false, message: await response.text() };
      if (!response.ok || result.success === false) throw new Error(result.message || "Đăng xuất FShare thất bại");
      fshareAccountEmail = "";
      fshareAccountRank = "";
      fsharePassword = "";
      settings = { ...settings, fshare_configured: false };
      fshareLoginMessage = "Đã đăng xuất FShare.";
      status = fshareLoginMessage;
      await refreshAll();
    } catch (error) {
      const message = error instanceof Error ? error.message : "Đăng xuất FShare thất bại";
      fshareLoginMessage = message;
      status = message;
    } finally {
      fshareSaving = false;
    }
  }

  function switchFshareAccount() {
    fshareEmail = "";
    fsharePassword = "";
    fsharePanelOpen = true;
    fshareLoginMessage = "Nhập tài khoản FShare mới để đổi tài khoản.";
  }

  function applyDownloadPreset(slots: number, segments: number) {
    maxConcurrent = slots;
    segmentsPerDownload = segments;
  }

  async function saveAutoTrackSettings() {
    autoTrackSaving = true;
    try {
      const response = await fetch("/api/settings/auto-track", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ check_interval_secs: Number(autoTrackIntervalSecs) }),
      });
      const result = response.ok ? await response.json() : { success: false, message: await response.text() };
      if (!response.ok || result.success === false) throw new Error(result.message || "Lưu Auto Track thất bại");
      autoTrackSavedMessage = `Đã lưu · Tự track mỗi ${formatAutoTrackInterval(autoTrackIntervalSecs)}`;
      status = `Đã lưu Auto Track: quét mỗi ${formatAutoTrackInterval(autoTrackIntervalSecs)}. Các track hiện có đã được cập nhật.`;
    } catch (error) {
      autoTrackSavedMessage = "Lưu thất bại";
      status = error instanceof Error ? error.message : "Lưu Auto Track thất bại";
    } finally {
      autoTrackSaving = false;
    }
  }

  function formatAutoTrackInterval(value: number) {
    const secs = Number(value || 3600);
    if (secs < 3600) return `${Math.round(secs / 60)} phút`;
    const hours = secs / 3600;
    return `${Number.isInteger(hours) ? hours : hours.toFixed(1)} giờ`;
  }

  async function saveSettings() {
    saving = true;
    try {
      const response = await fetch("/api/settings/downloads", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          directory: downloadDirectory,
          max_concurrent: Number(maxConcurrent),
          segments_per_download: Number(segmentsPerDownload),
        }),
      });
      const result = response.ok ? await response.json() : { success: false, message: await response.text() };
      if (!response.ok || result.success === false) throw new Error(result.message || "Lưu cấu hình thất bại");
      settings = { ...settings, max_concurrent_downloads: Number(maxConcurrent), download_directory: downloadDirectory, segments_per_download: Number(segmentsPerDownload) };
      status = "Đã lưu cấu hình tải vào engine FHUB.";
    } catch (error) {
      const message = error instanceof Error ? error.message : "Lưu cấu hình thất bại";
      status = message;
    } finally {
      saving = false;
    }
  }

  async function createUser() {
    if (!newUsername.trim() || newPassword.length < 6) {
      status = uiLanguage === "vi" ? "Tên đăng nhập không được trống và mật khẩu cần ít nhất 6 ký tự." : "Username is required and password must be at least 6 characters.";
      return;
    }

    userSaving = true;
    try {
      const response = await fetch("/api/auth/users", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ username: newUsername.trim(), password: newPassword, role: newRole }),
      });
      if (!response.ok) throw new Error(await response.text());
      newUsername = "";
      newPassword = "";
      newRole = "user";
      status = "Đã tạo tài khoản người dùng.";
      await refreshAll();
    } catch {
      status = "Tạo tài khoản thất bại hoặc username đã tồn tại.";
    } finally {
      userSaving = false;
    }
  }

  async function updateUser(user: AppUser, patch: Partial<Pick<AppUser, "role" | "is_active">>) {
    try {
      const response = await fetch(`/api/auth/users/${user.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify(patch),
      });
      if (!response.ok) throw new Error(await response.text());
      status = `Đã cập nhật ${user.username}.`;
      await refreshAll();
    } catch {
      status = `Cập nhật ${user.username} thất bại.`;
    }
  }

  async function resetUserPassword(user: AppUser) {
    const password = passwordDraft.trim();
    if (password.length < 6) {
      status = "Mật khẩu mới cần ít nhất 6 ký tự.";
      return;
    }
    try {
      const response = await fetch(`/api/auth/users/${user.id}`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({ password }),
      });
      if (!response.ok) throw new Error(await response.text());
      passwordDraft = "";
      status = `Đã đổi mật khẩu cho ${user.username}.`;
      await refreshAll();
    } catch {
      status = `Đổi mật khẩu cho ${user.username} thất bại.`;
    }
  }

  async function deleteUser(user: AppUser) {
    if (!confirm(`Xóa tài khoản ${user.username}?`)) return;
    try {
      const response = await fetch(`/api/auth/users/${user.id}`, { method: "DELETE", credentials: "include" });
      if (!response.ok) throw new Error(await response.text());
      status = `Đã xóa ${user.username}.`;
      await refreshAll();
    } catch {
      status = `Xóa ${user.username} thất bại.`;
    }
  }

  function formatDate(value?: string | null) {
    if (!value) return "Chưa đăng nhập";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "Không rõ";
    return date.toLocaleString("vi-VN", { hour12: false });
  }
</script>

{#if accessChecked && isAdminUser}
<div class="settings-screen">
  <section class="settings-hero" aria-label="FHUB admin console">
    <div class="hero-glow"></div>
    <div class="brand-orb">
      <img src="/images/logo5.png" alt="FHUB" />
    </div>
    <div class="hero-copy">
      <h1>{uiLanguage === "vi" ? "Cài đặt" : "Settings"}</h1>
    </div>
    <div class="hero-actions">
      <button type="button" class="ghost-button" onclick={async () => { await refreshAll(); await checkUpdateStatus(); }} aria-label={uiLanguage === "vi" ? "Làm mới" : "Refresh"}>
        <span class="material-icons">refresh</span>
        {uiLanguage === "vi" ? "Làm mới" : "Refresh"}
      </button>
      {#if shouldShowUpdateBanner}
        <button type="button" class="primary-button header-update-button" disabled={updatingApp} onclick={runWebUpdate} aria-label="Update FHub">
          <span class="material-icons">upgrade</span>
          {updatingApp ? "Đang update..." : "Update"}
        </button>
      {/if}
    </div>
  </section>

  {#if status.startsWith("Không") || status.startsWith("Tab") || status.includes("thất bại") || status.includes("lỗi")}
    <div class="status-line">
      <span>{status}</span>
      <button type="button" onclick={refreshAll}>Thử lại</button>
    </div>
  {/if}

  <section class="overview-strip">
    <article><span class="material-icons">palette</span><div><strong>{uiLanguage === "vi" ? "Chế độ" : "Mode"}</strong><small>{uiMode === "classic" ? t.classicMode : t.modernMode}</small></div></article>
    <article><span class="material-icons">cloud_done</span><div><strong>FShare</strong><small>{settings.fshare_configured ? "Đã cấu hình" : "Chưa cấu hình"}</small></div></article>
    <article><span class="material-icons">dns</span><div><strong>Server</strong><small>{`${settings.server_host}:${settings.server_port}`}</small></div></article>
  </section>

  {#if shouldShowUpdateBanner}
    <section class="update-banner available" aria-label="FHub update available">
      <div class="update-icon"><span class="material-icons">system_update_alt</span></div>
      <div class="update-copy">
        <strong>{updatingApp ? "Đang update FHub" : (uiLanguage === "vi" ? "Có bản cập nhật mới" : "Update available")}</strong>
        <small>{updatingApp ? updateMessage : `Hiện tại ${updateCurrentLabel} · Mới nhất ${updateStatus?.latest_commit}`}</small>
      </div>
      <div class="update-actions">
        <button type="button" class="update-now" disabled={updatingApp} onclick={runWebUpdate} aria-label="Update FHub">
          <span class="material-icons">upgrade</span>
          {updatingApp ? "Đang update..." : "Update FHub"}
        </button>
        {#if updatingApp}<small>Trang tự làm mới sau 1 phút</small>{:else if checkingUpdate}<small>Đang kiểm tra...</small>{/if}
      </div>
    </section>
  {/if}

  <nav class="settings-tabs" aria-label="Settings sections">
    {#each tabOrder as tab}
      <button type="button" class:active={activeTab === tab} onclick={() => activeTab = tab}>
        <span class="material-icons">{tabIcons[tab]}</span>
        <strong>{t.tabs[tab]}</strong>
      </button>
    {/each}
  </nav>

  {#if showUpdateConfirm || updatingApp}
    <div class="update-modal-backdrop" role="dialog" aria-modal="true" aria-label="Xác nhận cập nhật FHub">
      <div class="update-modal">
        <div class="update-modal-glow glow-one"></div>
        <div class="update-modal-glow glow-two"></div>
        {#if !updatingApp}
          <button type="button" class="update-modal-close" onclick={() => showUpdateConfirm = false} aria-label="Đóng"><span class="material-icons">close</span></button>
        {/if}
        <div class="update-modal-head compact">
          <div class="update-modal-mark"><span class="material-icons">{updatingApp ? "sync" : "system_update_alt"}</span></div>
          <span class="update-modal-kicker">FHUB UPDATE</span>
          <h2>{updatingApp ? "Đang update" : "Xác nhận update"}</h2>
          <p>{updatingApp ? "FHub đang kéo image mới, recreate container và kiểm tra health. Trang sẽ tự làm mới sau 1 phút." : "FHub sẽ kéo image mới nhất từ GHCR, khởi động lại container và giữ nguyên dữ liệu hiện có."}</p>
        </div>
        <div class="update-version-grid">
          <div class="update-version-card"><span>Hiện tại</span><strong>{updateCurrentLabel}</strong></div>
          <div class="update-version-arrow"><span class="material-icons">arrow_forward</span></div>
          <div class="update-version-card latest"><span>Mới nhất</span><strong>{updateStatus?.latest_commit || "latest"}</strong></div>
        </div>
        <div class="update-modal-note">
          <span class="material-icons">{updatingApp ? "hourglass_top" : "info"}</span>
          <p>{updatingApp ? updateMessage : "Không đóng trang trong lúc update. Nếu container restart làm ngắt request, FHub vẫn sẽ tự làm mới trang sau 1 phút."}</p>
        </div>
        <div class="update-modal-actions simple">
          {#if updatingApp}
            <button type="button" class="update-cancel-button" disabled>Đang cập nhật</button>
            <button type="button" class="update-confirm-button" disabled><span class="material-icons">sync</span>Làm mới sau 1 phút</button>
          {:else}
            <button type="button" class="update-cancel-button" onclick={() => showUpdateConfirm = false}>Huỷ</button>
            <button type="button" class="update-confirm-button" onclick={confirmWebUpdate}><span class="material-icons">upgrade</span>Update</button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <section class="settings-grid overview-grid">
    {#if activeTab === "overview"}
      <article class="panel wide ui-mode-panel">{@render UiModePanel()}</article>
      <article class="panel wide fshare-overview-panel">{@render ServicePanel()}</article>
      <article class="panel download-overview-panel">{@render DownloadPanel()}</article>
      <article class="panel autotrack-overview-panel">{@render AutoTrackPanel()}</article>
    {:else if activeTab === "accounts"}
      <article class="panel users-panel">{@render UsersPanel()}</article>
    {:else if activeTab === "activity"}
      <article class="panel wide">{@render ActivityPanel()}</article>
    {/if}
  </section>

</div>


{#snippet UiModePanel()}
  <div class="panel-title simple-mode-title">
    <div><h2>{t.uiModeTitle}</h2></div>
  </div>
  <div class="ui-mode-switch" role="group" aria-label={t.uiModeTitle}>
    <button type="button" class:active={uiMode === "modern"} onclick={() => setUiMode("modern")}>{t.modernMode}</button>
    <button type="button" class:active={uiMode === "classic"} onclick={() => setUiMode("classic")}>{t.classicMode}</button>
  </div>
{/snippet}

{#snippet DownloadPanel()}
  <div class="panel-title compact-title">
    <div><h2>{uiLanguage === "vi" ? "Bộ máy tải" : "Download engine"}</h2></div>
    <span class="panel-chip">{uiLanguage === "vi" ? "Sẵn sàng" : "Ready"}</span>
  </div>
  <div class="download-meter compact-meter">
    <div><strong>{maxConcurrent}</strong><span>{uiLanguage === "vi" ? "luồng" : "slots"} · {segmentsPerDownload} segment/file</span></div>
    <div class="meter-bars" aria-hidden="true">
      {#each Array(8) as _, index}<span class:active={index < Math.min(maxConcurrent, 8)}></span>{/each}
    </div>
  </div>
  <button type="button" class="advanced-toggle" class:open={downloadAdvancedOpen} onclick={() => downloadAdvancedOpen = !downloadAdvancedOpen}>
    <span class="material-icons">tune</span>
    <strong>Cấu hình nâng cao</strong>
    <span class="material-icons">expand_more</span>
  </button>
  {#if downloadAdvancedOpen}
    <div class="download-advanced-box">
      <div class="download-preset-grid" role="group" aria-label="Cấu hình tải nhanh">
        {#each downloadPresets as preset}
          <button type="button" class:active={activeDownloadPreset === preset.id} onclick={() => applyDownloadPreset(preset.slots, preset.segments)}>
            <strong>{preset.label}</strong>
            <span>{preset.hint}</span>
            <small>{preset.slots} luồng · {preset.segments} segment/file</small>
          </button>
        {/each}
      </div>
      <div class="best-config-note">
        <span class="material-icons">bolt</span>
        <div><strong>{uiLanguage === "vi" ? "Cấu hình nhanh nhất: 6 luồng · 32 segment/file" : "Fastest config: 6 slots · 32 segments/file"}</strong><small>{uiLanguage === "vi" ? "Tối thiểu: 1 luồng / 1 segment. Tối đa khuyên dùng: 6 luồng / 32 segment. Cao hơn dễ bị FShare timeout hoặc 503." : "Minimum: 1 slot / 1 segment. Recommended max: 6 slots / 32 segments. Higher values may trigger FShare timeout or 503."}</small></div>
      </div>
      <details class="manual-download-tune">
        <summary>Chỉnh tay</summary>
        <div class="compact-download-fields">
          <label><span>Số luồng <small>1–20</small></span><input type="range" min="1" max="20" bind:value={maxConcurrent} /><strong>{maxConcurrent}</strong></label>
          <label><span>Segment/file <small>1–32</small></span><input type="range" min="1" max="32" bind:value={segmentsPerDownload} /><strong>{segmentsPerDownload}</strong></label>
        </div>
      </details>
      <small class="config-note">Ưu tiên tốc độ: chọn “Nhanh nhất”. Nếu FShare báo timeout/503 thì lùi về “Cân bằng”.</small>
      <button type="button" class="primary-button compact-save" onclick={saveSettings} disabled={saving}>{saving ? "Đang lưu..." : "Lưu cấu hình"}</button>
    </div>
  {/if}
{/snippet}

{#snippet AutoTrackPanel()}
  <div class="panel-title compact-title">
    <div><h2>Auto Track</h2></div>
    <span class="panel-chip">{formatAutoTrackInterval(autoTrackIntervalSecs)}</span>
  </div>
  <div class="autotrack-box">
    <div class="autotrack-copy">
      <span class="material-icons">sync</span>
      <div>
        <strong>Chu kỳ theo dõi phim bộ</strong>
        <small>FHUB sẽ tự quét các folder Auto Track theo thời gian này. Track mới tạo cũng dùng giá trị này.</small>
      </div>
    </div>
    <div class="autotrack-presets" role="group" aria-label="Auto Track interval">
      {#each [900, 1800, 3600, 10800, 21600] as interval}
        <button type="button" class:active={Number(autoTrackIntervalSecs) === interval} onclick={() => { autoTrackIntervalSecs = interval; autoTrackSavedMessage = ""; }}>
          {formatAutoTrackInterval(interval)}
        </button>
      {/each}
    </div>
    <label class="autotrack-slider">
      <span>Tuỳ chỉnh <small>{formatAutoTrackInterval(autoTrackIntervalSecs)}</small></span>
      <input type="range" min="300" max="21600" step="300" bind:value={autoTrackIntervalSecs} oninput={() => autoTrackSavedMessage = ""} />
    </label>
    <button type="button" class="primary-button compact-save" onclick={saveAutoTrackSettings} disabled={autoTrackSaving}>{autoTrackSaving ? "Đang lưu..." : "Lưu Auto Track"}</button>
    {#if autoTrackSavedMessage}<div class="save-ok"><span class="material-icons">check_circle</span>{autoTrackSavedMessage}</div>{/if}
  </div>
{/snippet}

{#snippet FavoritesPanel()}
  <div class="panel-title">
    <div><span class="eyebrow">Yêu thích</span><h2>Nội dung đã lưu</h2></div>
    <span class="panel-chip">{favoriteItems.length} phim</span>
  </div>
  <div class="favorite-list">
    {#each favoriteItems as item}
      <article class="favorite-row">
        <img src={item.poster} alt={item.title} />
        <div>
          <strong>{item.title}</strong>
          <small>{item.note}</small>
        </div>
        <button type="button"><span class="material-icons">play_arrow</span></button>
      </article>
    {/each}
  </div>
{/snippet}

{#snippet HistoryPanel()}
  <div class="panel-title">
    <div><span class="eyebrow">Lịch sử</span><h2>Đã xem và đã tải</h2></div>
    <span class="panel-chip">24h</span>
  </div>
  <div class="history-list">
    {#each historyItems as item}
      <article class="history-row">
        <time>{item.time}</time>
        <div>
          <strong>{item.action}</strong>
          <span>{item.title}</span>
          <small>{item.detail}</small>
        </div>
      </article>
    {/each}
  </div>
{/snippet}

{#snippet ServicePanel()}
  <div class="panel-title fshare-title"><div><h2>{uiLanguage === "vi" ? "Cấu hình FShare" : "FShare config"}</h2></div></div>
  <div class="service-list">
    {#each services as service}
      {#if service.id === "fshare"}
        <article class="service-row fshare-service {service.accent}" class:open={fsharePanelOpen}>
          <button type="button" class="service-main" onclick={() => fsharePanelOpen = !fsharePanelOpen} aria-expanded={fsharePanelOpen}>
            <div class="service-icon" class:vip={fshareIsVip}>
              {#if fshareIsVip}
                <span class="vip-logo-inline">VIP</span>
              {:else}
                <span class="material-icons">{settings.fshare_configured ? "cloud_done" : "cloud_off"}</span>
              {/if}
            </div>
            <div><strong>FShare</strong><small>{fshareAccountEmail || "Chưa đăng nhập FShare"}</small></div>
            <span class="service-state">{settings.fshare_configured ? "Đã đăng nhập" : "Đăng nhập"}</span>
            <span class="material-icons expand-mark">{fsharePanelOpen ? "expand_less" : "expand_more"}</span>
          </button>

          {#if fsharePanelOpen}
            <div class="fshare-inline-panel">
              {#if settings.fshare_configured && fshareAccountEmail}
                <div class="fshare-current">
                  <span class="material-icons">verified_user</span>
                  <div><strong>{fshareAccountEmail}</strong><small>Tài khoản FShare đang dùng cho tải xuống</small></div>
                </div>
                <div class="fshare-actions compact-row">
                  <button type="button" class="ghost-button" onclick={switchFshareAccount} disabled={fshareSaving}>Đổi tài khoản</button>
                  <button type="button" class="danger-button" onclick={logoutFshareAccount} disabled={fshareSaving}>Đăng xuất</button>
                </div>
              {:else}
                <div class="fshare-login-box">
                  <label class="credential-field"><span>{uiLanguage === "vi" ? "Email / tên đăng nhập FShare" : "FShare email / username"}</span><input bind:value={fshareEmail} autocomplete="username" placeholder={uiLanguage === "vi" ? "email hoặc tên đăng nhập" : "email or username"} /></label>
                  <label class="credential-field"><span>Mật khẩu FShare</span><div class="credential-input compact"><span class="material-icons">lock</span><input bind:value={fsharePassword} type={showFsharePassword ? "text" : "password"} autocomplete="current-password" placeholder="Mật khẩu FShare" /><button class="password-toggle" type="button" aria-label={showFsharePassword ? "Ẩn mật khẩu" : "Hiện mật khẩu"} onclick={() => showFsharePassword = !showFsharePassword}><span class="material-icons">{showFsharePassword ? "visibility_off" : "visibility"}</span></button></div></label>
                  <button type="button" class="primary-button" onclick={saveFshareAccount} disabled={fshareSaving}>{fshareSaving ? "Đang đăng nhập..." : "Đăng nhập FShare"}</button>
                </div>
              {/if}
              {#if fshareLoginMessage}<div class="credential-feedback">{fshareLoginMessage}</div>{/if}
            </div>
          {/if}
        </article>
      {:else}
        <div class="service-row {service.accent}">
          <div class="service-icon"><span class="material-icons">{service.icon}</span></div>
          <div><strong>{service.name}</strong><small>{service.label}</small></div>
          <span class="service-state">
            {service.id === "tmdb" ? (settings.tmdb_configured ? "Đã cấu hình" : "Chưa cấu hình") : "Đang chạy"}
          </span>
        </div>
      {/if}
    {/each}
  </div>
{/snippet}

{#snippet CreateUserPanel()}
  <div class="panel-title"><div><span class="eyebrow">Tạo tài khoản</span><h2>{uiLanguage === "vi" ? "Quản trị hoặc người dùng mới" : "New Admin or User"}</h2></div></div>
  <form onsubmit={(event) => { event.preventDefault(); createUser(); }}>
    <label><span>{t.username}</span><input bind:value={newUsername} autocomplete="username" placeholder="vd: family_user" /></label>
    <label><span>Mật khẩu</span><input bind:value={newPassword} type="password" autocomplete="new-password" placeholder="Ít nhất 6 ký tự" /></label>
    <label><span>Quyền</span><select bind:value={newRole}><option value="user">Người dùng</option><option value="admin">Quản trị</option></select></label>
    <button type="submit" class="primary-button" disabled={userSaving}>{userSaving ? "Đang tạo..." : "Tạo tài khoản"}</button>
  </form>
{/snippet}

{#snippet UsersPanel()}
  <div class="panel-title users-title">
    <div><span class="eyebrow">{t.localAccount}</span><h2>{t.usersTitle}</h2></div>
    <button type="button" class="panel-chip create-user-button" onclick={() => { newUsername = ""; newPassword = ""; newRole = "user"; showAdminModal = true; }}>
      <span class="material-icons">person_add</span>
      {t.createUser}
    </button>
  </div>
  <div class="user-list compact-users">
    {#if users.length}
      {#each users as user}
        <article class="user-card" class:open={selectedUserId === user.id}>
          <button type="button" class="user-line" onclick={() => selectedUserId = selectedUserId === user.id ? null : user.id}>
            <span class="avatar mini">{user.username.slice(0, 1).toUpperCase()}</span>
            <span class="user-copy"><strong>{user.username}</strong><small>{user.role === "admin" ? "Quản trị" : "Người dùng"} · {user.is_active ? "hoạt động" : "đã khóa"} · {formatDate(user.last_login_at)}</small></span>
            <span class="material-icons">{selectedUserId === user.id ? "expand_less" : "expand_more"}</span>
          </button>
          {#if selectedUserId === user.id}
            <div class="user-tools">
              <label class="inline-field"><span>Quyền</span><select value={user.role} onchange={(event) => updateUser(user, { role: event.currentTarget.value as "admin" | "user" })}><option value="user">Người dùng</option><option value="admin">Quản trị</option></select></label>
              <label class="inline-field"><span>Mật khẩu mới</span><input bind:value={passwordDraft} type="password" autocomplete="new-password" placeholder="Nhập mật khẩu mới" /></label>
              <div class="tool-buttons">
                <button type="button" onclick={() => resetUserPassword(user)}>Đổi MK</button>
                <button type="button" onclick={() => updateUser(user, { is_active: !user.is_active })}>{user.is_active ? "Khóa" : "Mở"}</button>
                <button type="button" class="danger-button" onclick={() => deleteUser(user)}>Xóa</button>
              </div>
              <div class="mini-history"><strong>Lịch sử</strong><span>Đăng nhập cuối: {formatDate(user.last_login_at)}</span><span>Trạng thái: {user.is_active ? "Đang hoạt động" : "Đã khóa"}</span></div>
            </div>
          {/if}
        </article>
      {/each}
    {:else}
      <div class="empty-users">Chưa có tài khoản nào hoặc bạn chưa có quyền admin.</div>
    {/if}
  </div>
{/snippet}

{#snippet ActivityPanel()}
  <div class="panel-title"><div><span class="eyebrow">Nhật ký</span><h2>Hoạt động gần đây</h2></div></div>
  <div class="audit-list">
    {#each auditTrail as item}
      <div><time>{item.time}</time><span>{item.text}</span></div>
    {/each}
  </div>
{/snippet}

{/if}

{#if showAdminModal}
  <div
    class="admin-modal-backdrop"
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) showAdminModal = false;
    }}
  >
    <div class="admin-modal" role="dialog" aria-modal="true" aria-labelledby="admin-create-title">
      <button class="modal-close" type="button" aria-label="Đóng" onclick={() => showAdminModal = false}>
        <span class="material-icons">close</span>
      </button>
      <div class="modal-mark">
        <span class="material-icons">admin_panel_settings</span>
      </div>
      <div class="modal-copy">
        <span class="eyebrow">{t.createAccount}</span>
        <h2 id="admin-create-title">{t.adminOrUser}</h2>
        <p>{t.createHint}</p>
      </div>
      <form class="modal-form" onsubmit={(event) => { event.preventDefault(); createUser(); showAdminModal = false; }}>
        <label><span>{t.username}</span><input bind:value={newUsername} autocomplete="username" placeholder="vd: family_user" /></label>
        <label><span>{t.password}</span><input bind:value={newPassword} type="password" autocomplete="new-password" placeholder="Ít nhất 6 ký tự" /></label>
        <label><span>{t.role}</span><select bind:value={newRole}><option value="admin">{uiLanguage === "vi" ? "Quản trị" : "Admin"}</option><option value="user">{uiLanguage === "vi" ? "Người dùng" : "User"}</option></select></label>
        <div class="modal-actions">
          <button type="button" class="ghost-button" onclick={() => showAdminModal = false}>{t.cancel}</button>
          <button type="submit" class="primary-button" disabled={userSaving}>{userSaving ? t.creating : t.submitCreate}</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .admin-modal-backdrop { position: fixed; inset: 0; z-index: 80; display: grid; place-items: center; padding: 1rem; background: rgba(3, 6, 14, .72); backdrop-filter: blur(18px) saturate(150%); }
  .admin-modal { position: relative; width: min(560px, 100%); display: grid; gap: 1rem; padding: clamp(1rem, 3vw, 1.35rem); border: 1px solid rgba(167,139,250,.34); border-radius: 24px; background: radial-gradient(circle at 14% 0%, rgba(248,193,74,.22), transparent 28%), radial-gradient(circle at 100% 12%, rgba(56,189,248,.16), transparent 30%), linear-gradient(180deg, rgba(20,26,42,.98), rgba(8,10,18,.98)); box-shadow: 0 34px 120px rgba(0,0,0,.58), inset 0 1px 0 rgba(255,255,255,.05); }
  .modal-close { position: absolute; right: .85rem; top: .85rem; width: 38px; height: 38px; display: grid; place-items: center; border: 1px solid rgba(148,163,184,.16); border-radius: 13px; color: #f8fafc; background: rgba(255,255,255,.06); }
  .modal-mark { width: 62px; height: 62px; display: grid; place-items: center; border-radius: 20px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); box-shadow: 0 0 36px rgba(248,193,74,.18); }
  .modal-mark .material-icons { color: #080a12; font-size: 2rem; }
  .modal-copy { display: grid; gap: .45rem; padding-right: 2.4rem; }
  .modal-copy p { color: rgba(226,232,240,.68); line-height: 1.5; }
  .modal-form { display: grid; gap: .8rem; }
  .modal-actions { display: grid; grid-template-columns: .8fr 1.2fr; gap: .75rem; margin-top: .2rem; }

  .settings-screen { display: grid; gap: 1.15rem; max-width: 1320px; margin: 0 auto; }
  .settings-hero { position: relative; display: grid; grid-template-columns: minmax(180px,250px) minmax(0,1fr) minmax(160px,auto); align-items: center; gap: clamp(1.25rem,4vw,2.8rem); min-height: 332px; overflow: hidden; padding: clamp(1.5rem,4vw,3.1rem); border: 1px solid rgba(167,139,250,.28); border-radius: 26px; background: radial-gradient(circle at 14% 48%, rgba(248,193,74,.34), transparent 19%), radial-gradient(circle at 12% 42%, rgba(124,58,237,.58), transparent 37%), radial-gradient(circle at 78% 4%, rgba(56,189,248,.12), transparent 24%), linear-gradient(120deg, rgba(41,30,79,.98), rgba(13,17,29,.98) 56%, rgba(7,10,18,.98)); box-shadow: 0 32px 100px rgba(0,0,0,.48), inset 0 1px 0 rgba(255,255,255,.05); }
  .settings-hero::before { content: ""; position: absolute; inset: 18px; pointer-events: none; border-radius: 22px; border: 1px solid rgba(255,255,255,.035); }
  .settings-hero::after { content: ""; position: absolute; inset: auto 0 0; height: 1px; background: linear-gradient(90deg, transparent, rgba(248,193,74,.72), rgba(56,189,248,.66), transparent); }
  .hero-glow { position: absolute; width: 320px; height: 320px; left: -96px; top: 16px; border-radius: 999px; background: rgba(248,193,74,.18); filter: blur(44px); }
  .brand-orb, .hero-copy, .hero-actions { position: relative; z-index: 1; }
  .brand-orb { display: grid; place-items: center; min-height: 210px; }
  .brand-orb img { width: min(235px,100%); filter: drop-shadow(0 0 30px rgba(248,193,74,.24)) drop-shadow(0 24px 52px rgba(0,0,0,.42)); }
  .eyebrow { color: #c4b5fd; font-size: .73rem; font-weight: 950; letter-spacing: .16em; text-transform: uppercase; }
  h1, h2, p { margin: 0; }
  h1 { margin-top: .55rem; max-width: 800px; color: #fff; font-size: clamp(4rem,7vw,7.4rem); font-weight: 950; line-height: .9; letter-spacing: -.06em; }
  h2 { color: #fff; font-size: clamp(1.25rem,2.1vw,1.6rem); line-height: 1.1; letter-spacing: -.04em; }
  .hero-copy p { max-width: 700px; margin-top: 1rem; color: rgba(226,232,240,.7); font-size: 1.06rem; line-height: 1.55; }
  .hero-actions { display: flex; flex-wrap: wrap; gap: .65rem; justify-content: flex-end; align-items: center; pointer-events: auto; }
  .update-native-form { margin: 0; display: inline-flex; min-width: 0; }
  .update-submit-frame { position: fixed; width: 1px; height: 1px; opacity: 0; pointer-events: none; border: 0; }
  .header-update-button { width: auto; min-width: 118px; pointer-events: auto; display: inline-flex; align-items: center; justify-content: center; gap: .45rem; }
  .ghost-button, .primary-button, .danger-button, .user-row button { min-height: 48px; border-radius: 14px; padding: 0 1rem; border: 1px solid rgba(148,163,184,.16); color: #f8fafc; background: rgba(255,255,255,.06); font-weight: 900; }
  .ghost-button { display: inline-flex; align-items: center; gap: .45rem; background: rgba(255,255,255,.08); }
  .hero-signal { display: inline-flex; align-items: center; gap: .5rem; color: rgba(226,232,240,.72); font-size: .82rem; font-weight: 800; }
  .hero-signal span { width: 10px; height: 10px; border-radius: 999px; background: #38bdf8; box-shadow: 0 0 20px rgba(56,189,248,.82); }
  .overview-strip { display: none; }
  .overview-strip article, .status-line, .panel, .update-banner { border: 1px solid rgba(148,163,184,.16); background: linear-gradient(135deg, rgba(124,58,237,.08), transparent 44%), linear-gradient(180deg, rgba(20,26,42,.92), rgba(9,12,21,.86)); box-shadow: 0 18px 48px rgba(0,0,0,.26); backdrop-filter: blur(18px) saturate(140%); }
  .overview-strip article { min-height: 104px; padding: 1rem; border-radius: 18px; }
  .overview-strip small, .overview-strip span, .status-line, label span, .service-row small, .user-copy small, .audit-list span, .note-list p { color: rgba(226,232,240,.64); }
  .overview-strip small { display: block; font-weight: 850; }
  .overview-strip strong { display: block; margin: .35rem 0 .18rem; color: #fff; font-size: 2rem; line-height: 1; letter-spacing: -.05em; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .overview-strip span { font-size: .82rem; }
  .update-banner { position: relative; display: grid; grid-template-columns: 42px minmax(0,1fr) auto; align-items: center; gap: .78rem; overflow: hidden; padding: .72rem .78rem; border-radius: 18px; }
  .update-banner::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: radial-gradient(circle at 9% 0%, rgba(248,193,74,.18), transparent 34%), radial-gradient(circle at 100% 0%, rgba(167,139,250,.15), transparent 32%); opacity: .9; }
  .update-banner.available { border-color: rgba(248,193,74,.28); background: linear-gradient(180deg, rgba(18,24,39,.88), rgba(8,11,20,.82)); box-shadow: 0 14px 36px rgba(0,0,0,.2), inset 0 1px 0 rgba(255,255,255,.04); }
  .update-banner > * { position: relative; z-index: 1; }
  .update-icon { width: 42px; height: 42px; display: grid; place-items: center; border-radius: 14px; color: #080a12; background: linear-gradient(135deg,#f8d983 0%,#e4c0d2 48%,#b9a7ff 100%); box-shadow: 0 10px 24px rgba(167,139,250,.16); }
  .update-icon .material-icons { color: #080a12; font-size: 1.35rem; }
  .update-copy { min-width: 0; display: grid; gap: .14rem; }
  .update-copy strong, .update-copy small, .update-copy p { display: block; }
  .update-copy strong { color: #fff; font-size: .96rem; letter-spacing: -.02em; }
  .update-copy small { color: rgba(226,232,240,.58); font-size: .78rem; font-weight: 820; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .update-copy p { margin-top: .32rem; color: rgba(226,232,240,.62); font-size: .82rem; line-height: 1.35; }
  .update-warning code, .update-hint code { padding: .08rem .28rem; border-radius: 6px; background: rgba(0,0,0,.32); color: #fde68a; }
  .update-command-box { margin-top: .55rem; display: grid; grid-template-columns: minmax(0,1fr) auto; gap: .45rem; align-items: center; max-width: 560px; padding: .42rem; border: 1px solid rgba(148,163,184,.14); border-radius: 12px; background: rgba(2,6,23,.34); }
  .update-command-box code { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #fde68a; font-size: .8rem; }
  .update-command-box button { min-height: 34px; padding: 0 .75rem; border: 0; border-radius: 10px; color: #08111f; background: linear-gradient(135deg,#f8c14a,#c4b5fd); font-weight: 950; }
  .update-hint { color: rgba(226,232,240,.54) !important; }
  .update-actions { display: grid; gap: .5rem; min-width: 122px; justify-items: end; }
  .update-now { min-height: 40px; display: inline-flex; align-items: center; justify-content: center; gap: .35rem; padding: 0 .95rem; border: 0; color: #080a12; background: linear-gradient(135deg,#f8d983 0%,#e4c0d2 48%,#b9a7ff 100%); box-shadow: 0 10px 24px rgba(167,139,250,.18); }
  .update-now .material-icons { color: #080a12; font-size: 1.05rem; }
  .status-line { min-height: 52px; display: flex; align-items: center; padding: .9rem 1rem; border-radius: 16px; }
  .console-grid, .tab-grid { display: grid; gap: 1rem; align-items: start; }
  .console-grid { grid-template-columns: minmax(330px,.95fr) minmax(360px,1.05fr) minmax(300px,.8fr); }
  .two-col { grid-template-columns: minmax(0,1fr) minmax(340px,.7fr); }
  .accounts-grid { grid-template-columns: minmax(340px,.65fr) minmax(0,1.35fr); }
  .overview-grid { gap: 1.55rem; align-items: start; }
  .ui-mode-panel { margin-bottom: .35rem; }
  .fshare-overview-panel { margin-top: .15rem; margin-bottom: .15rem; }
  .download-overview-panel { margin-top: .35rem; border-color: rgba(167,139,250,.22); }
  .autotrack-overview-panel { border-color: rgba(56,189,248,.22); }
  .panel { border-radius: 22px; padding: 1.15rem; }
  .ui-mode-hint { display: none; }
  .simple-mode-title { margin-bottom: .7rem; }
  .simple-mode-title h2 { font-size: 1.16rem; }
  .fshare-title { margin-bottom: .72rem; }
  .fshare-title h2 { font-size: 1.16rem; }
  .ui-mode-switch { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .22rem; padding: .34rem; border-radius: 23px; border: 1px solid rgba(148,163,184,.14); background: rgba(10,13,24,.58); box-shadow: inset 0 1px 0 rgba(255,255,255,.05); }
  .ui-mode-switch button { min-height: 43px; border: 0; border-radius: 18px; color: rgba(226,232,240,.72); background: transparent; font-size: .92rem; font-weight: 950; white-space: nowrap; }
  .ui-mode-switch button.active { color: #080a12; background: linear-gradient(135deg,#f8d983 0%,#e4c0d2 48%,#b9a7ff 100%); box-shadow: 0 10px 24px rgba(167,139,250,.22), inset 0 1px 0 rgba(255,255,255,.34); }
  .config-panel, .users-panel { grid-column: span 2; }
  .panel-title { display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 1rem; }
  .panel-chip { flex: 0 0 auto; padding: .42rem .7rem; border: 1px solid rgba(167,139,250,.24); border-radius: 999px; color: #e9d5ff; background: rgba(124,58,237,.16); font-size: .75rem; font-weight: 900; }
  .create-user-button { display: inline-flex; align-items: center; gap: .36rem; min-height: 40px; cursor: pointer; }
  .create-user-button .material-icons { color: inherit; font-size: 1rem; }
  .settings-tabs { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); align-items: center; gap: .45rem; padding: .42rem; border: 1px solid rgba(148,163,184,.14); border-radius: 999px; background: linear-gradient(180deg, rgba(20,26,42,.72), rgba(7,10,18,.68)); box-shadow: inset 0 1px 0 rgba(255,255,255,.045), 0 14px 34px rgba(0,0,0,.2); }
  .settings-tabs::-webkit-scrollbar { display: none; }
  .settings-tabs button { width: 100%; min-width: 0; min-height: 48px; display: inline-flex; align-items: center; justify-content: center; gap: .58rem; padding: 0 .72rem; border: 1px solid transparent; border-radius: 999px; color: rgba(226,232,240,.72); background: transparent; font-weight: 900; white-space: nowrap; transition: background .18s ease, color .18s ease, border-color .18s ease, transform .18s ease; }
  .settings-tabs button:hover { color: #fff; background: rgba(255,255,255,.06); }
  .settings-tabs button.active { color: #080a12; border-color: rgba(248,193,74,.45); background: linear-gradient(135deg,#f8c14a,#d6a8c9 52%,#a78bfa); box-shadow: 0 10px 28px rgba(167,139,250,.22); }
  .settings-tabs .material-icons { flex: 0 0 30px; width: 30px; height: 30px; display: grid; place-items: center; border-radius: 999px; color: inherit; background: rgba(255,255,255,.07); font-size: 1.08rem; line-height: 1; }
  .settings-tabs button.active .material-icons { color: #080a12; background: rgba(8,10,18,.12); }
  .settings-tabs strong { min-width: 0; font-size: .9rem; line-height: 1; text-align: center; overflow: hidden; text-overflow: ellipsis; }
  .settings-tabs small { display: none; }
  .settings-mobile-actions, .settings-mobile-update, .settings-mobile-signout { display: none; }
  .download-meter { display: grid; grid-template-columns: 132px minmax(0,1fr); align-items: center; gap: 1rem; min-height: 116px; margin-bottom: 1rem; padding: 1rem; border: 1px solid rgba(148,163,184,.14); border-radius: 18px; background: radial-gradient(circle at 0% 50%, rgba(56,189,248,.14), transparent 36%), rgba(255,255,255,.045); }
  .download-meter strong { display: block; color: #fff; font-size: 3rem; line-height: .92; letter-spacing: -.06em; }
  .download-meter span { color: rgba(226,232,240,.62); font-weight: 800; }
  .compact-title { margin-bottom: .65rem; }
  .compact-meter { grid-template-columns: 88px minmax(0,1fr); min-height: 76px; margin-bottom: .75rem; padding: .75rem; border-radius: 16px; }
  .compact-meter strong { font-size: 2.15rem; }
  .compact-meter span { font-size: .8rem; }
  .advanced-toggle { width: 100%; min-height: 44px; display: grid; grid-template-columns: 28px minmax(0, 1fr) 28px; align-items: center; gap: .5rem; padding: 0 .75rem; border-radius: 15px; border: 1px solid rgba(167,139,250,.18); color: #e9d5ff; background: rgba(124,58,237,.1); font-weight: 950; text-align: left; }
  .advanced-toggle .material-icons { color: #c4b5fd; transition: transform .18s ease; }
  .advanced-toggle.open .material-icons:last-child { transform: rotate(180deg); }
  .download-advanced-box { margin-top: .75rem; padding: .8rem; border-radius: 18px; border: 1px solid rgba(148,163,184,.14); background: rgba(6,9,17,.42); }
  .autotrack-box { display: grid; gap: .8rem; }
  .autotrack-copy { display: grid; grid-template-columns: 42px minmax(0,1fr); gap: .7rem; align-items: center; padding: .75rem; border-radius: 16px; border: 1px solid rgba(56,189,248,.15); background: rgba(56,189,248,.07); }
  .autotrack-copy > .material-icons { width: 42px; height: 42px; display: grid; place-items: center; border-radius: 14px; color: #080a12; background: linear-gradient(135deg,#38bdf8,#a78bfa); }
  .autotrack-copy strong, .autotrack-copy small { display: block; }
  .autotrack-copy strong { color: #fff; }
  .autotrack-copy small { margin-top: .18rem; color: #aab4c3; line-height: 1.35; }
  .autotrack-presets { display: grid; grid-template-columns: repeat(5,minmax(0,1fr)); gap: .42rem; }
  .autotrack-presets button { min-height: 38px; padding: 0 .35rem; border-radius: 12px; border: 1px solid rgba(148,163,184,.15); color: #e2e8f0; background: rgba(255,255,255,.055); font-weight: 900; }
  .autotrack-presets button.active { color: #080a12; border: 0; background: linear-gradient(135deg,#f8c14a,#a78bfa); }
  .autotrack-slider { display: grid; gap: .45rem; padding: .72rem; border-radius: 14px; background: rgba(255,255,255,.04); }
  .autotrack-slider span { display: flex; justify-content: space-between; gap: .7rem; color: #f8fafc; font-weight: 900; }
  .autotrack-slider small { color: #f8c14a; }
  .save-ok { display: inline-flex; align-items: center; justify-content: center; gap: .4rem; min-height: 38px; padding: 0 .75rem; border-radius: 12px; color: #86efac; background: rgba(34,197,94,.12); border: 1px solid rgba(34,197,94,.22); font-weight: 900; }
  .save-ok .material-icons { color: #86efac; font-size: 1.1rem; }
  .download-preset-grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: .55rem; }
  .download-preset-grid button { min-height: 82px; display: grid; align-content: center; gap: .14rem; padding: .55rem .45rem; border-radius: 15px; text-align: left; color: #e5eef7; background: rgba(255,255,255,.045); border: 1px solid rgba(148,163,184,.16); }
  .download-preset-grid button.active { color: #080a12; border-color: transparent; background: linear-gradient(135deg,#f8c14a,#a78bfa); box-shadow: 0 14px 36px rgba(167,139,250,.18); }
  .download-preset-grid strong { font-size: .92rem; }
  .download-preset-grid span { color: inherit; opacity: .76; font-size: .72rem; font-weight: 900; }
  .download-preset-grid small { color: inherit; opacity: .62; font-size: .68rem; line-height: 1.2; }
  .best-config-note { display: grid; grid-template-columns: 34px minmax(0, 1fr); gap: .58rem; align-items: start; margin-top: .72rem; padding: .65rem; border-radius: 14px; border: 1px solid rgba(248,193,74,.18); background: rgba(248,193,74,.075); }
  .best-config-note .material-icons { width: 34px; height: 34px; display: grid; place-items: center; border-radius: 12px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); }
  .best-config-note strong, .best-config-note small { display: block; line-height: 1.28; }
  .best-config-note strong { color: #f8fafc; font-size: .82rem; }
  .best-config-note small { margin-top: .18rem; color: rgba(226,232,240,.62); font-size: .72rem; }
  .manual-download-tune { margin-top: .65rem; border-radius: 14px; border: 1px solid rgba(148,163,184,.13); background: rgba(255,255,255,.035); overflow: hidden; }
  .manual-download-tune summary { cursor: pointer; padding: .55rem .65rem; color: #c4b5fd; font-weight: 950; }
  .compact-download-fields { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: .65rem; padding: 0 .65rem .65rem; }
  .compact-download-fields label { gap: .38rem; }
  .compact-download-fields label span { display: flex; align-items: baseline; justify-content: space-between; gap: .5rem; }
  .compact-download-fields label span small { color: rgba(226,232,240,.42); font-size: .68rem; }
  .compact-download-fields input[type="range"] { padding: 0; min-height: 28px; accent-color: #f8c14a; }
  .compact-download-fields label > strong { color: #f8c14a; font-size: .95rem; }
  .config-note { display: block; margin: .72rem 0; color: rgba(226,232,240,.52); font-size: .78rem; line-height: 1.35; }
  .compact-save { min-height: 42px; width: 100%; }
  .meter-bars { display: grid; grid-template-columns: repeat(8,minmax(12px,1fr)); gap: .45rem; align-items: end; min-height: 70px; }
  .meter-bars span { min-height: 22px; border-radius: 999px; background: rgba(148,163,184,.12); }
  .meter-bars span:nth-child(2n) { min-height: 42px; }
  .meter-bars span:nth-child(3n) { min-height: 58px; }
  .meter-bars span.active { background: linear-gradient(180deg,#38bdf8,#a78bfa 55%,#f8c14a); box-shadow: 0 0 24px rgba(56,189,248,.22); }
  label, form, .service-list, .user-list, .audit-list, .note-list { display: grid; gap: .75rem; }
  label span { font-size: .86rem; font-weight: 850; }
  input, select { width: 100%; min-height: 54px; border: 1px solid rgba(148,163,184,.16); border-radius: 14px; padding: 0 .9rem; color: #f8fafc; background: rgba(6,9,17,.74); outline: none; }
  input:focus, select:focus { border-color: rgba(167,139,250,.46); box-shadow: 0 0 0 4px rgba(124,58,237,.12); }
  .primary-button { width: 100%; color: #080a12; border: 0; background: linear-gradient(100deg,#f8c14a 0%,#d6a8c9 50%,#a78bfa 100%); }
  .fshare-credential-panel { position: relative; overflow: hidden; display: grid; gap: 1rem; padding: clamp(1rem,2.4vw,1.35rem); border-color: rgba(167,139,250,.26); background: radial-gradient(circle at 0% 0%, rgba(248,193,74,.18), transparent 30%), radial-gradient(circle at 100% 4%, rgba(56,189,248,.13), transparent 30%), linear-gradient(180deg, rgba(20,26,42,.96), rgba(7,10,18,.92)); }
  .fshare-credential-panel::before { content: ""; position: absolute; inset: 0; pointer-events: none; background: linear-gradient(120deg, rgba(255,255,255,.08), transparent 28%, transparent 72%, rgba(167,139,250,.07)); opacity: .55; }
  .credential-orb, .credential-title, .credential-form, .credential-note { position: relative; z-index: 1; }
  .credential-orb { width: 58px; height: 58px; display: grid; place-items: center; border-radius: 18px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); box-shadow: 0 18px 42px rgba(167,139,250,.18), 0 0 30px rgba(248,193,74,.12); }
  .credential-orb .material-icons { color: #080a12; font-size: 1.85rem; }
  .credential-title { margin-bottom: 0; }
  .credential-title p { max-width: 520px; margin-top: .55rem; color: rgba(226,232,240,.66); line-height: 1.48; }
  .credential-form { display: grid; gap: .85rem; }
  .credential-field { gap: .42rem; }
  .credential-field > span { color: rgba(226,232,240,.72); font-size: .78rem; letter-spacing: .08em; text-transform: uppercase; }
  .credential-input { display: grid; grid-template-columns: 24px minmax(0,1fr) auto; align-items: center; gap: .72rem; min-height: 58px; padding: 0 .9rem; border: 1px solid rgba(148,163,184,.16); border-radius: 17px; background: rgba(3,6,14,.62); box-shadow: inset 0 1px 0 rgba(255,255,255,.045); transition: border-color .18s ease, box-shadow .18s ease, background .18s ease; }
  .credential-input:focus-within { border-color: rgba(167,139,250,.42); background: rgba(3,6,14,.78); box-shadow: 0 0 0 4px rgba(124,58,237,.12), inset 0 1px 0 rgba(255,255,255,.05); }
  .credential-input .material-icons { color: #c4b5fd; font-size: 1.2rem; }
  .credential-input input { min-height: 56px; padding: 0; border: 0; border-radius: 0; background: transparent; box-shadow: none; font-weight: 850; letter-spacing: -.01em; }
  .credential-input input:focus { box-shadow: none; }
  .password-toggle { width: 42px; height: 42px; display: grid; place-items: center; border: 1px solid rgba(167,139,250,.18); border-radius: 13px; color: #c4b5fd; background: rgba(255,255,255,.055); }
  .password-toggle:hover { color: #fff; background: rgba(167,139,250,.16); }
  .credential-input .password-toggle .material-icons { color: inherit; font-size: 1.2rem; }
  .credential-feedback { margin: -.2rem 0 .1rem; padding: .72rem .82rem; border: 1px solid rgba(248,193,74,.18); border-radius: 14px; color: rgba(255,247,237,.84); background: rgba(248,193,74,.08); font-size: .84rem; font-weight: 780; line-height: 1.42; }
  .credential-button { display: inline-flex; align-items: center; justify-content: center; gap: .45rem; min-height: 54px; border-radius: 17px; box-shadow: 0 18px 38px rgba(167,139,250,.13); }
  .credential-button .material-icons { color: #080a12; font-size: 1.15rem; }
  .credential-note { display: flex; align-items: center; gap: .55rem; padding: .75rem .85rem; border: 1px solid rgba(56,189,248,.16); border-radius: 15px; color: rgba(226,232,240,.66); background: rgba(56,189,248,.06); font-size: .82rem; font-weight: 750; }
  .credential-note .material-icons { color: #7dd3fc; font-size: 1.05rem; }
  .service-row { display: grid; grid-template-columns: 48px minmax(0,1fr) auto; align-items: center; gap: .85rem; min-height: 78px; padding: .85rem; border: 1px solid rgba(148,163,184,.14); border-radius: 17px; background: rgba(255,255,255,.05); }
  .service-row.fshare-service { display: block; padding: 0; overflow: hidden; }
  .service-main { width: 100%; min-height: 78px; display: grid; grid-template-columns: 48px minmax(0,1fr) auto 24px; align-items: center; gap: .85rem; padding: .85rem; border: 0; color: inherit; background: transparent; text-align: left; }
  .service-main .expand-mark { color: rgba(226,232,240,.52); font-size: 1.2rem; }
  .fshare-service.open { border-color: rgba(56,189,248,.24); background: radial-gradient(circle at 0 0, rgba(56,189,248,.12), transparent 34%), rgba(255,255,255,.05); }
  .fshare-inline-panel { display: grid; gap: .75rem; padding: 0 .85rem .85rem; }
  .fshare-current { display: grid; grid-template-columns: 38px minmax(0,1fr); align-items: center; gap: .65rem; padding: .72rem; border-radius: 14px; background: rgba(3,6,14,.34); border: 1px solid rgba(148,163,184,.1); }
  .fshare-current > .material-icons { width: 38px; height: 38px; display: grid; place-items: center; border-radius: 12px; color: #7dd3fc; background: rgba(56,189,248,.1); }
  .fshare-current strong, .fshare-current small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fshare-current strong { color: #fff; font-size: .94rem; }
  .fshare-current small { margin-top: .12rem; color: rgba(226,232,240,.58); font-size: .76rem; }
  .fshare-actions { display: grid; grid-template-columns: 1fr 1fr; gap: .55rem; }
  .fshare-actions.compact-row { display: grid; grid-template-columns: 1fr 1fr; align-items: center; gap: .65rem; }
  .fshare-actions.compact-row button { width: 100%; min-width: 0; justify-content: center; padding: 0 .75rem; }
  .fshare-actions button { min-height: 44px; }
  .fshare-login-box { display: grid; gap: .7rem; }
  .credential-input.compact { min-height: 48px; border-radius: 14px; }
  .credential-input.compact input { min-height: 46px; }
  .service-icon { width: 48px; height: 48px; display: grid; place-items: center; border-radius: 15px; background: rgba(255,255,255,.06); }
  .service-icon.vip { width: 42px; height: 42px; border-radius: 13px; background: linear-gradient(135deg,#fff3a3 0%,#f8c14a 42%,#d98d12 100%); box-shadow: 0 0 18px rgba(248,193,74,.22), inset 0 1px 0 rgba(255,255,255,.42); }
  .vip-logo-inline { color: #241407; font-size: .64rem; font-weight: 1000; letter-spacing: .06em; }
  .service-row.cyan .service-icon { color: #7dd3fc; box-shadow: inset 0 0 24px rgba(56,189,248,.12); }
  .service-row.cyan .service-icon.vip { color: #241407; box-shadow: 0 0 18px rgba(248,193,74,.22), inset 0 1px 0 rgba(255,255,255,.42); }
  .service-row.violet .service-icon { color: #c4b5fd; box-shadow: inset 0 0 24px rgba(167,139,250,.14); }
  .service-row.gold .service-icon { color: #facc15; box-shadow: inset 0 0 24px rgba(248,193,74,.12); }
  .service-row strong, .service-row small, .user-copy strong, .user-copy small { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .service-state { color: rgba(226,232,240,.78); font-size: .8rem; font-weight: 850; }
  .user-row { display: grid; grid-template-columns: 54px minmax(0,1fr) 124px 78px 70px; align-items: center; gap: .72rem; min-height: 80px; padding: .8rem; border: 1px solid rgba(148,163,184,.14); border-radius: 17px; background: rgba(255,255,255,.05); }
  .avatar { width: 54px; height: 54px; display: grid; place-items: center; border-radius: 16px; color: #f8c14a; background: radial-gradient(circle at 35% 18%, rgba(56,189,248,.24), transparent 38%), linear-gradient(180deg, rgba(31,41,64,.96), rgba(7,10,18,.96)); font-weight: 950; }
  .danger-button { color: #fecaca; border-color: rgba(248,113,113,.26); background: rgba(127,29,29,.18); }
  .empty-users { padding: 1rem; border-radius: 16px; color: rgba(226,232,240,.64); background: rgba(255,255,255,.045); }
  .activity-panel { min-height: 100%; }
  .audit-list div { display: grid; grid-template-columns: 54px minmax(0,1fr); gap: .8rem; padding: .8rem 0; border-bottom: 1px solid rgba(148,163,184,.1); }
  .audit-list div:last-child { border-bottom: 0; }
  .audit-list time { color: #c4b5fd; font-weight: 950; }
  .storage-map { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: .75rem; }
  .storage-map div { padding: 1rem; border-radius: 16px; background: rgba(255,255,255,.045); border: 1px solid rgba(148,163,184,.12); }
  .storage-map strong, .storage-map span { display: block; }
  .storage-map strong { color: #fff; font-size: 1.55rem; }
  .storage-map span { margin-top: .3rem; color: rgba(226,232,240,.62); }
  .media-grid { display: grid; grid-template-columns: repeat(auto-fit,minmax(170px,1fr)); gap: 1rem; }
  .media-tile { position: relative; min-height: 260px; overflow: hidden; border-radius: 18px; border: 1px solid rgba(148,163,184,.16); background: #111827; }
  .media-tile img { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: cover; }
  .media-shade { position: absolute; inset: 0; background: linear-gradient(180deg, transparent 38%, rgba(0,0,0,.94)); }
  .media-copy { position: absolute; left: .9rem; right: .9rem; bottom: .9rem; display: grid; gap: .35rem; }
  .media-copy strong, .media-copy small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .media-copy strong { color: #fff; }
  .media-copy small { color: rgba(226,232,240,.68); }
  .progress-track { height: 7px; overflow: hidden; border-radius: 999px; background: rgba(148,163,184,.2); }
  .progress-track span { display: block; height: 100%; border-radius: inherit; background: linear-gradient(90deg,#38bdf8,#a78bfa,#f8c14a); }
  .favorite-list, .history-list { display: grid; gap: .85rem; }
  .favorite-row { display: grid; grid-template-columns: 64px minmax(0,1fr) 44px; align-items: center; gap: .85rem; min-height: 82px; padding: .7rem; border-radius: 17px; border: 1px solid rgba(148,163,184,.14); background: rgba(255,255,255,.05); }
  .favorite-row img { width: 64px; height: 64px; object-fit: cover; border-radius: 14px; }
  .favorite-row strong, .favorite-row small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .favorite-row strong { color: #fff; }
  .favorite-row small { color: rgba(226,232,240,.62); }
  .favorite-row button { width: 44px; height: 44px; border: 1px solid rgba(167,139,250,.2); border-radius: 14px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); }
  .favorite-row button .material-icons { color: #080a12; }
  .history-row { display: grid; grid-template-columns: 62px minmax(0,1fr); gap: .9rem; padding: .9rem; border-radius: 17px; border: 1px solid rgba(148,163,184,.14); background: rgba(255,255,255,.05); }
  .history-row time { color: #f8c14a; font-weight: 950; }
  .history-row strong, .history-row span, .history-row small { display: block; }
  .history-row strong { color: #fff; }
  .history-row span { margin-top: .18rem; color: rgba(226,232,240,.78); font-weight: 850; }
  .history-row small { margin-top: .18rem; color: rgba(226,232,240,.58); }
  @media (max-width: 1180px) { .settings-hero, .console-grid, .two-col, .accounts-grid { grid-template-columns: 1fr; } .hero-actions { justify-items: start; } .config-panel, .users-panel { grid-column: auto; } }
  @media (max-width: 760px) {
    .settings-screen { gap: .7rem; }
    .settings-hero { display: none; }
    .settings-tabs { margin-inline: -.15rem; padding: .32rem; gap: .28rem; border-radius: 999px; }
    .settings-tabs button { min-height: 40px; padding: 0 .36rem; gap: .28rem; }
    .settings-tabs .material-icons { flex-basis: 25px; width: 25px; height: 25px; font-size: .98rem; }
    .settings-tabs strong { font-size: .76rem; }
    .overview-strip { display: none; }
    .status-line { display: none; }
    .download-meter, .user-row, .storage-map, .update-banner { grid-template-columns: 1fr; }
    .compact-update { justify-content: stretch; }
    .update-actions { grid-template-columns: 1fr; min-width: 0; justify-items: stretch; }
    .update-actions .update-now { width: 100%; min-height: 52px; }
    .compact-update .update-actions { grid-template-columns: 1fr; }
    .download-preset-grid { grid-template-columns: 1fr; }
    .download-preset-grid button { min-height: 64px; }
    .compact-download-fields { grid-template-columns: 1fr; }
    .service-row { grid-template-columns: 36px minmax(0,1fr) auto; gap: .55rem; min-height: 58px; padding: .55rem .62rem; border-radius: 14px; }
    .service-row.fshare-service { padding: 0; }
    .service-main { min-height: 58px; grid-template-columns: 36px minmax(0,1fr) auto 20px; gap: .55rem; padding: .55rem .62rem; }
    .fshare-inline-panel { padding: 0 .62rem .62rem; }
    .fshare-actions { grid-template-columns: 1fr; }
    .fshare-actions.compact-row { display: grid; grid-template-columns: 1fr 1fr; gap: .5rem; }
    .fshare-actions.compact-row button { min-height: 40px; font-size: .82rem; padding: 0 .35rem; }
    .service-icon { width: 36px; height: 36px; border-radius: 12px; }
    .service-row strong { font-size: .92rem; }
    .service-row small { font-size: .78rem; }
    .service-state { font-size: .72rem; white-space: nowrap; }
    .panel { padding: .85rem; border-radius: 17px; }
    .ui-mode-switch { grid-template-columns: repeat(2, minmax(0, 1fr)); }
    .panel-title { margin-bottom: .65rem; }
    .panel-title h2 { font-size: 1.22rem; }
    .eyebrow { font-size: .64rem; letter-spacing: .14em; }
    .user-row { min-height: 62px; padding: .58rem; gap: .5rem; }
    .media-grid { display: flex; overflow-x: auto; padding-bottom: .4rem; }
    .media-tile { min-width: 176px; }
    .service-icon, .avatar { width: 48px; height: 48px; }
    .user-row select, .user-row button { width: 100%; }
  }
  @media (max-width: 460px) { .panel { padding: .82rem; border-radius: 17px; } }
  .compact-users { gap: .5rem; }
  .user-card { border: 1px solid rgba(148,163,184,.14); border-radius: 15px; background: rgba(255,255,255,.045); overflow: hidden; }
  .user-card.open { border-color: rgba(167,139,250,.28); background: rgba(124,58,237,.08); }
  .user-line { width: 100%; min-height: 56px; display: grid; grid-template-columns: 36px minmax(0,1fr) 24px; align-items: center; gap: .58rem; padding: .5rem .58rem; border: 0; border-radius: 0; color: #f8fafc; background: transparent; text-align: left; }
  .avatar.mini { width: 36px; height: 36px; border-radius: 12px; font-size: .95rem; }
  .user-line .material-icons { color: rgba(226,232,240,.68); font-size: 1.2rem; }
  .user-tools { display: grid; gap: .5rem; padding: 0 .58rem .58rem; }
  .inline-field { display: grid; gap: .25rem; }
  .inline-field span { font-size: .72rem; color: rgba(226,232,240,.62); }
  .inline-field input, .inline-field select { min-height: 40px; border-radius: 12px; font-size: .9rem; }
  .tool-buttons { display: grid; grid-template-columns: repeat(3,minmax(0,1fr)); gap: .45rem; }
  .tool-buttons button { min-height: 38px; border-radius: 12px; padding: 0 .45rem; font-size: .82rem; }
  .mini-history { display: grid; gap: .12rem; padding: .5rem .55rem; border-radius: 12px; color: rgba(226,232,240,.64); background: rgba(3,6,14,.35); font-size: .75rem; }
  .mini-history strong { color: #fff; font-size: .8rem; }
  @media(max-width:720px){ .settings-grid{grid-template-columns:1fr;gap:1rem}.settings-hero{display:none}.overview-strip{display:none}.status-line{display:none}.download-overview-panel{margin-top:.45rem}.settings-mobile-actions{position:sticky;top:.75rem;z-index:9999;display:grid;grid-template-columns:1fr;gap:.6rem;margin:.55rem 0 1.15rem;padding:.48rem;border-radius:22px;border:1px solid rgba(148,163,184,.18);background:rgba(8,12,22,.88);box-shadow:0 18px 50px rgba(0,0,0,.42);backdrop-filter:blur(18px) saturate(150%)}.mobile-update-form,.mobile-update-link{display:flex;width:100%}.settings-mobile-update{width:100%;min-height:56px;display:flex;align-items:center;justify-content:center;gap:.55rem;padding:0 1rem;border-radius:17px;border:0;color:#080a12;text-decoration:none;background:linear-gradient(135deg,#f8c14a,#a78bfa);font-weight:950;touch-action:manipulation;pointer-events:auto}.settings-mobile-update .material-icons{color:#080a12}.settings-mobile-signout{min-height:56px;display:flex;align-items:center;justify-content:center;gap:.55rem;margin:0;padding:0 1rem;border-radius:17px;border:1px solid rgba(248,113,113,.22);color:#fecaca;text-decoration:none;background:rgba(127,29,29,.16);font-weight:950}.settings-mobile-signout .material-icons{color:#fecaca}.users-panel{padding:.75rem!important}.users-title{margin-bottom:.55rem}.user-line{min-height:52px;grid-template-columns:34px minmax(0,1fr) 22px;padding:.46rem .52rem}.avatar.mini{width:34px;height:34px;border-radius:11px}.user-copy strong{font-size:.94rem}.user-copy small{font-size:.74rem}.user-tools{padding:0 .52rem .52rem;gap:.42rem}.tool-buttons button{min-height:36px}.inline-field input,.inline-field select{min-height:38px}.mini-history{font-size:.72rem}}


  .update-modal-backdrop { position: fixed; inset: 0; z-index: 99999; display: grid; place-items: center; padding: clamp(.9rem, 3vw, 2rem); background: radial-gradient(circle at 50% 0%, rgba(167,139,250,.18), transparent 34%), rgba(2,5,13,.76); backdrop-filter: blur(22px) saturate(150%); }
  .update-modal { position: relative; isolation: isolate; width: min(640px, 100%); overflow: hidden; display: grid; gap: 1rem; padding: clamp(1rem, 3vw, 1.45rem); border: 1px solid rgba(167,139,250,.34); border-radius: 30px; background: radial-gradient(circle at 12% 0%, rgba(248,193,74,.20), transparent 30%), radial-gradient(circle at 96% 4%, rgba(167,139,250,.20), transparent 32%), linear-gradient(180deg, rgba(18,24,40,.98), rgba(6,9,18,.98)); box-shadow: 0 38px 130px rgba(0,0,0,.62), 0 0 0 1px rgba(255,255,255,.035) inset, 0 0 60px rgba(167,139,250,.12); }
  .update-modal::before { content: ""; position: absolute; inset: 10px; z-index: -1; border-radius: 24px; border: 1px solid rgba(255,255,255,.045); pointer-events: none; }
  .update-modal::after { content: ""; position: absolute; left: 8%; right: 8%; top: 0; height: 1px; background: linear-gradient(90deg, transparent, rgba(248,193,74,.88), rgba(167,139,250,.82), transparent); }
  .update-modal-glow { position: absolute; z-index: -2; border-radius: 999px; filter: blur(36px); opacity: .75; pointer-events: none; }
  .update-modal-glow.glow-one { width: 220px; height: 220px; left: -80px; top: -82px; background: rgba(248,193,74,.28); }
  .update-modal-glow.glow-two { width: 260px; height: 260px; right: -120px; bottom: -112px; background: rgba(167,139,250,.26); }
  .update-modal-close { position: absolute; right: .9rem; top: .9rem; width: 42px; height: 42px; display: grid; place-items: center; border: 1px solid rgba(255,255,255,.10); border-radius: 15px; color: rgba(248,250,252,.78); background: rgba(255,255,255,.055); backdrop-filter: blur(12px); }
  .update-modal-close:hover { color: #fff; background: rgba(255,255,255,.10); }
  .update-modal-close .material-icons { color: inherit; font-size: 1.2rem; }
  .update-modal-head { display: grid; gap: .55rem; padding-right: 2.7rem; }
  .update-modal-mark { width: 66px; height: 66px; display: grid; place-items: center; border-radius: 22px; color: #080a12; background: linear-gradient(135deg, #f8d983 0%, #e5bdd6 48%, #a78bfa 100%); box-shadow: 0 18px 44px rgba(167,139,250,.22), 0 0 42px rgba(248,193,74,.14), inset 0 1px 0 rgba(255,255,255,.42); }
  .update-modal-mark .material-icons { color: #080a12; font-size: 2.05rem; }
  .update-modal-kicker { width: fit-content; padding: .34rem .6rem; border: 1px solid rgba(248,193,74,.24); border-radius: 999px; color: #fde68a; background: rgba(248,193,74,.075); font-size: .68rem; font-weight: 1000; letter-spacing: .14em; }
  .update-modal h2 { color: #fff; font-size: clamp(2.1rem, 6vw, 3.35rem); font-weight: 1000; line-height: .92; letter-spacing: -.07em; text-shadow: 0 18px 42px rgba(0,0,0,.36); }
  .update-modal-head p { max-width: 560px; color: rgba(248,250,252,.78); font-size: clamp(.98rem, 2.2vw, 1.12rem); line-height: 1.48; }
  .update-version-grid { display: grid; grid-template-columns: minmax(0,1fr) 42px minmax(0,1fr); align-items: center; gap: .7rem; }
  .update-version-card { min-width: 0; display: grid; gap: .28rem; min-height: 86px; padding: .82rem .9rem; border: 1px solid rgba(148,163,184,.14); border-radius: 20px; background: rgba(255,255,255,.055); box-shadow: inset 0 1px 0 rgba(255,255,255,.045); }
  .update-version-card.latest { border-color: rgba(248,193,74,.26); background: radial-gradient(circle at 0 0, rgba(248,193,74,.12), transparent 42%), rgba(255,255,255,.06); }
  .update-version-card span { color: rgba(226,232,240,.58); font-size: .76rem; font-weight: 950; letter-spacing: .08em; text-transform: uppercase; }
  .update-version-card strong { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: #fff; font-size: clamp(1.05rem, 3vw, 1.28rem); font-weight: 1000; letter-spacing: -.03em; }
  .update-version-card.latest strong { color: #fde68a; }
  .update-version-arrow { width: 42px; height: 42px; display: grid; place-items: center; border-radius: 15px; color: #080a12; background: linear-gradient(135deg,#f8c14a,#a78bfa); box-shadow: 0 12px 28px rgba(167,139,250,.18); }
  .update-version-arrow .material-icons { color: #080a12; font-size: 1.22rem; }
  .update-modal-note { display: grid; grid-template-columns: 38px minmax(0,1fr); align-items: start; gap: .68rem; padding: .78rem .85rem; border: 1px solid rgba(56,189,248,.16); border-radius: 18px; background: rgba(56,189,248,.06); }
  .update-modal-note .material-icons { width: 38px; height: 38px; display: grid; place-items: center; border-radius: 13px; color: #7dd3fc; background: rgba(56,189,248,.1); font-size: 1.08rem; }
  .update-modal-note p { color: rgba(226,232,240,.68); font-size: .86rem; line-height: 1.42; }
  .update-modal-actions { display: grid; grid-template-columns: .85fr 1.25fr; gap: .75rem; }
  .update-cancel-button, .update-confirm-button { min-height: 64px; border-radius: 21px; padding: 0 1rem; font-size: clamp(1rem, 2.8vw, 1.32rem); font-weight: 1000; }
  .update-cancel-button { color: #f8fafc; border: 1px solid rgba(148,163,184,.18); background: linear-gradient(180deg, rgba(255,255,255,.08), rgba(255,255,255,.045)); box-shadow: inset 0 1px 0 rgba(255,255,255,.055); }
  .update-confirm-button { display: inline-flex; align-items: center; justify-content: center; gap: .6rem; color: #080a12; border: 0; background: linear-gradient(135deg,#f8c14a 0%,#e0b7d4 52%,#a78bfa 100%); box-shadow: 0 16px 40px rgba(167,139,250,.24), inset 0 1px 0 rgba(255,255,255,.34); }
  .update-confirm-button .material-icons { color: #080a12; font-size: 1.24rem; }
  .update-cancel-button:disabled, .update-confirm-button:disabled { opacity: .55; cursor: not-allowed; }
  @media (max-width: 560px) { .update-modal { border-radius: 24px; } .update-modal-head { padding-right: 2.2rem; } .update-version-grid { grid-template-columns: 1fr; } .update-version-arrow { justify-self: center; transform: rotate(90deg); } .update-modal-actions { grid-template-columns: 1fr; } .update-cancel-button, .update-confirm-button { min-height: 56px; } }

  .update-modal { max-width: 430px; gap: 1.15rem; padding: clamp(1.05rem, 4vw, 1.35rem); border-radius: 28px; }
  .update-modal-head.compact { place-items: center; padding-right: 0; text-align: center; }
  .update-modal-head.compact .update-modal-mark { width: 72px; height: 72px; border-radius: 24px; }
  .update-modal-head.compact h2 { max-width: 8ch; font-size: clamp(2rem, 8vw, 2.75rem); line-height: .94; }
  .update-modal-head.compact p { max-width: 330px; font-size: .98rem; line-height: 1.45; }
  .update-modal-actions.simple { grid-template-columns: 1fr 1fr; margin-top: .2rem; }
  .update-modal-actions.simple .update-cancel-button,
  .update-modal-actions.simple .update-confirm-button { min-height: 58px; }
  @media (max-width: 560px) { .update-modal { width: min(360px, 100%); max-height: calc(100dvh - 150px); overflow: auto; } .update-modal-head.compact .update-modal-mark { width: 64px; height: 64px; } .update-modal-head.compact h2 { font-size: 2.25rem; } .update-modal-actions.simple { grid-template-columns: 1fr 1fr; } }

</style>
