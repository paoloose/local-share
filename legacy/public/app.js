const OWNER_KEY = "paoloose-share.owner";
const BUCKET_KEY = "paoloose-share.bucket";

const store = {
  get(key) {
    try {
      return localStorage.getItem(key);
    } catch {
      return null;
    }
  },
  set(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch {
      // storage blocked: keep running on in-memory values only
    }
  },
};

function bucketFromPath() {
  const parts = location.pathname.split("/").filter(Boolean);
  if (parts.length !== 1) return "";
  return /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(parts[0]) ? parts[0] : "";
}

function syncBucketUrl() {
  if (state.bucket && bucketFromPath() !== state.bucket) {
    history.replaceState(null, "", "/" + encodeURIComponent(state.bucket));
  }
}

const pathBucket = bucketFromPath();
if (pathBucket) store.set(BUCKET_KEY, pathBucket);

const state = {
  owner: store.get(OWNER_KEY) || "",
  bucket: pathBucket || store.get(BUCKET_KEY) || "",
  files: [],
  uploadQueue: [],
};

let onIdentityDone = null;
let onBucketDone = null;
let renameTarget = null;
let deleteTarget = null;
let toastTimer = null;

const $ = (id) => document.getElementById(id);

function openModal(id) {
  const backdrop = $(id);
  backdrop.classList.add("open");
  const input = backdrop.querySelector("input.input");
  if (input) {
    input.focus();
    input.select();
  }
}

function closeModal(id) {
  $(id).classList.remove("open");
}

function showToast(message) {
  const toast = $("toast");
  toast.textContent = message;
  toast.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.hidden = true;
  }, 5000);
}

function showFieldError(id, message) {
  const el = $(id);
  el.textContent = message;
  el.hidden = false;
}

function clearFieldError(id) {
  $(id).hidden = true;
}

function normalizeBucket(value) {
  const raw = value.trim();
  if (!raw || (!raw.includes("/") && !raw.includes("://"))) return raw;
  try {
    const url = new URL(raw.includes("://") ? raw : `https://${raw}`);
    const fromQuery = url.searchParams.get("bucket");
    if (fromQuery) return fromQuery.trim();
    const first = url.pathname.split("/").filter(Boolean)[0];
    return first || raw;
  } catch {
    return raw.split("/").filter(Boolean).pop() || raw;
  }
}

function formatSize(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

function formatDate(iso) {
  return new Date(iso).toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function typeLabel(file) {
  const ext = file.name.includes(".") ? file.name.split(".").pop() : "";
  return (ext || file.type.split("/").pop() || "file").slice(0, 5).toLowerCase();
}

async function gzipBuffer(buffer) {
  const stream = new Blob([buffer]).stream().pipeThrough(new CompressionStream("gzip"));
  return await new Response(stream).arrayBuffer();
}

async function api(path, options = {}) {
  const sep = path.includes("?") ? "&" : "?";
  const res = await fetch(`${path}${sep}bucket=${encodeURIComponent(state.bucket)}`, options);
  const body = await res.json().catch(() => ({}));
  if (!res.ok) throw new Error(body.error || `request failed (${res.status})`);
  return body;
}

async function loadFiles() {
  syncBucketUrl();
  $("bucket-name").textContent = state.bucket || "none";
  $("owner-name").textContent = state.owner || "none";
  try {
    const { files } = await api("/api/files");
    state.files = files;
    renderFiles();
  } catch (err) {
    showToast(`Could not load files: ${err.message}`);
  }
}

function visibleFiles() {
  const query = $("search").value.trim().toLowerCase();
  const files = state.files.filter(
    (f) => !query || f.name.toLowerCase().includes(query) || f.owner.toLowerCase().includes(query)
  );
  const sort = $("sort").value;
  const byName = (a, b) => a.name.localeCompare(b.name);
  const byOwner = (a, b) => a.owner.localeCompare(b.owner) || b.uploadedAt.localeCompare(a.uploadedAt);
  if (sort === "date-desc") files.sort((a, b) => b.uploadedAt.localeCompare(a.uploadedAt));
  if (sort === "date-asc") files.sort((a, b) => a.uploadedAt.localeCompare(b.uploadedAt));
  if (sort === "owner") files.sort(byOwner);
  if (sort === "name") files.sort(byName);
  if (sort === "type") files.sort((a, b) => typeLabel(a).localeCompare(typeLabel(b)) || byName(a, b));
  return files;
}

function renderFiles() {
  const list = $("file-list");
  list.textContent = "";
  const files = visibleFiles();
  $("empty-state").classList.toggle("hidden", files.length > 0);

  for (const file of files) {
    const row = document.createElement("article");
    row.className = "file-row";

    const badge = document.createElement("div");
    badge.className = "type-badge";
    badge.textContent = typeLabel(file);

    const main = document.createElement("div");
    main.className = "file-main";
    const name = document.createElement("div");
    name.className = "file-name";
    name.textContent = file.name;
    name.title = file.name;
    const sub = document.createElement("div");
    sub.className = "file-sub";
    sub.textContent = `${file.owner} · ${formatSize(file.size)}`;
    const dot = document.createElement("span");
    dot.className = "dot";
    dot.textContent = "·";
    sub.append(dot, document.createTextNode(formatDate(file.uploadedAt)));
    main.append(name, sub);

    const actions = document.createElement("div");
    actions.className = "file-actions";
    actions.append(
      actionButton("Download", () => downloadFile(file)),
      actionButton("Rename", () => openRename(file)),
      actionButton("Delete", () => openConfirm(file), "danger")
    );

    row.append(badge, main, actions);
    list.append(row);
  }
}

function actionButton(label, onClick, extraClass = "") {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = `icon-btn ${extraClass}`.trim();
  btn.textContent = label;
  btn.addEventListener("click", onClick);
  return btn;
}

async function downloadFile(file) {
  try {
    const res = await fetch(`/api/files/${file.id}?bucket=${encodeURIComponent(state.bucket)}`);
    if (!res.ok) throw new Error("download failed");
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = file.name;
    a.click();
    URL.revokeObjectURL(url);
  } catch (err) {
    showToast(err.message);
  }
}

function openIdentityPrompt(onDone) {
  onIdentityDone = onDone;
  $("identity-input").value = state.owner;
  clearFieldError("identity-error");
  openModal("modal-identity");
}

function openBucketPrompt(cancellable, onDone) {
  onBucketDone = onDone;
  $("bucket-input").value = state.bucket;
  $("bucket-cancel").classList.toggle("hidden", !cancellable);
  clearFieldError("bucket-error");
  openModal("modal-bucket");
  renderBucketPills();
}

function openRename(file) {
  renameTarget = file;
  $("rename-input").value = file.name;
  clearFieldError("rename-error");
  openModal("modal-rename");
}

function openConfirm(file) {
  deleteTarget = file;
  $("confirm-text").textContent = `"${file.name}" will be gone for everyone in this bucket.`;
  openModal("modal-confirm");
}

async function renderBucketPills() {
  const list = $("bucket-list");
  list.textContent = "";
  try {
    const { buckets } = await fetch("/api/buckets").then((r) => r.json());
    for (const name of buckets) {
      const pill = document.createElement("button");
      pill.type = "button";
      pill.className = "bucket-pill";
      pill.textContent = name;
      pill.addEventListener("click", () => {
        $("bucket-input").value = name;
        $("bucket-input").focus();
      });
      list.append(pill);
    }
  } catch {
    // bucket discovery is best-effort
  }
}

function openUploadModal() {
  state.uploadQueue = [];
  $("upload-bucket").textContent = state.bucket;
  $("file-input").value = "";
  renderQueue();
  openModal("modal-upload");
}

function addFiles(fileList) {
  for (const file of fileList) state.uploadQueue.push(file);
  renderQueue();
}

function renderQueue() {
  const queue = $("upload-queue");
  queue.textContent = "";
  state.uploadQueue.forEach((file, index) => {
    const li = document.createElement("li");
    const name = document.createElement("span");
    name.className = "queue-name";
    name.textContent = file.name;
    const status = document.createElement("span");
    status.className = "queue-status";
    status.dataset.index = String(index);
    status.textContent = formatSize(file.size);
    const remove = document.createElement("button");
    remove.type = "button";
    remove.textContent = "x";
    remove.title = "Remove";
    remove.addEventListener("click", () => {
      state.uploadQueue.splice(index, 1);
      renderQueue();
    });
    li.append(name, status, remove);
    queue.append(li);
  });
  $("upload-start").disabled = state.uploadQueue.length === 0;
}

function queueStatus(file) {
  return $("upload-queue").querySelector(`[data-index="${state.uploadQueue.indexOf(file)}"]`);
}

async function uploadQueuedFiles() {
  const startButton = $("upload-start");
  startButton.disabled = true;

  for (const file of [...state.uploadQueue]) {
    const status = queueStatus(file);
    try {
      if (status) status.textContent = "compressing...";
      const compressed = await gzipBuffer(await file.arrayBuffer());
      if (status) status.textContent = "uploading...";
      await api("/api/upload", {
        method: "POST",
        headers: {
          "x-file-name": encodeURIComponent(file.name),
          "x-owner": encodeURIComponent(state.owner),
          "x-content-type": file.type || "application/octet-stream",
          "x-encoding": "gzip",
        },
        body: compressed,
      });
      state.uploadQueue = state.uploadQueue.filter((f) => f !== file);
      renderQueue();
    } catch (err) {
      if (status) status.textContent = "failed";
      showToast(`Upload failed for ${file.name}: ${err.message}`);
    }
  }

  startButton.disabled = state.uploadQueue.length === 0;
  if (state.uploadQueue.length === 0) closeModal("modal-upload");
  await loadFiles();
}

function wireEvents() {
  $("identity-form").addEventListener("submit", (event) => {
    event.preventDefault();
    const name = $("identity-input").value.trim();
    if (!name) {
      showFieldError("identity-error", "Type a name first.");
      return;
    }
    state.owner = name;
    store.set(OWNER_KEY, name);
    closeModal("modal-identity");
    const done = onIdentityDone;
    onIdentityDone = null;
    if (done) done();
  });

  $("bucket-form").addEventListener("submit", (event) => {
    event.preventDefault();
    const name = normalizeBucket($("bucket-input").value);
    $("bucket-input").value = name;
    if (!/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/.test(name)) {
      showFieldError("bucket-error", "Type a bucket name or a bucket link like https://drive.paoloose.site/andes");
      return;
    }
    state.bucket = name;
    store.set(BUCKET_KEY, name);
    closeModal("modal-bucket");
    const done = onBucketDone;
    onBucketDone = null;
    if (done) done();
  });

  $("bucket-cancel").addEventListener("click", () => {
    closeModal("modal-bucket");
    const done = onBucketDone;
    onBucketDone = null;
    if (done) done();
  });

  $("rename-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const name = $("rename-input").value.trim();
    if (!name || !renameTarget) {
      showFieldError("rename-error", "Type a new name first.");
      return;
    }
    try {
      await api(`/api/files/${renameTarget.id}`, {
        method: "PATCH",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name }),
      });
      renameTarget = null;
      closeModal("modal-rename");
      await loadFiles();
    } catch (err) {
      showFieldError("rename-error", err.message);
    }
  });

  $("rename-cancel").addEventListener("click", () => closeModal("modal-rename"));

  $("confirm-delete").addEventListener("click", async () => {
    if (!deleteTarget) return;
    try {
      await api(`/api/files/${deleteTarget.id}`, { method: "DELETE" });
      deleteTarget = null;
      closeModal("modal-confirm");
      await loadFiles();
    } catch (err) {
      showToast(err.message);
    }
  });

  $("confirm-cancel").addEventListener("click", () => {
    deleteTarget = null;
    closeModal("modal-confirm");
  });

  $("upload-btn").addEventListener("click", openUploadModal);
  $("upload-cancel").addEventListener("click", () => closeModal("modal-upload"));
  $("upload-start").addEventListener("click", uploadQueuedFiles);
  $("browse-btn").addEventListener("click", () => $("file-input").click());
  $("file-input").addEventListener("change", (e) => addFiles(e.target.files));

  const dropzone = $("dropzone");
  ["dragenter", "dragover"].forEach((type) =>
    dropzone.addEventListener(type, (e) => {
      e.preventDefault();
      dropzone.classList.add("drag");
    })
  );
  ["dragleave", "drop"].forEach((type) =>
    dropzone.addEventListener(type, (e) => {
      e.preventDefault();
      dropzone.classList.remove("drag");
    })
  );
  dropzone.addEventListener("drop", (e) => addFiles(e.dataTransfer.files));

  $("search").addEventListener("input", renderFiles);
  $("sort").addEventListener("change", renderFiles);

  $("owner-chip").addEventListener("click", () => openIdentityPrompt(loadFiles));
  $("bucket-chip").addEventListener("click", () => openBucketPrompt(true, loadFiles));

  document.addEventListener("keydown", (e) => {
    if (e.key !== "Escape") return;
    for (const id of ["modal-upload", "modal-rename", "modal-confirm"]) {
      if ($(id).classList.contains("open")) closeModal(id);
    }
  });
}

function boot() {
  wireEvents();
  const start = () => {
    if (state.bucket) loadFiles();
    else openBucketPrompt(false, loadFiles);
  };
  if (state.owner) start();
  else openIdentityPrompt(start);
}

boot();
