import { gzipSync, gunzipSync } from "bun";
import { mkdir, readFile, writeFile, readdir, rm, rename } from "node:fs/promises";
import { existsSync } from "node:fs";
import { join } from "node:path";

const PORT = 7777;
const ROOT = join(import.meta.dir, "buckets");
const MAX_UPLOAD_BYTES = 256 * 1024 * 1024;

type FileMeta = {
  id: string;
  name: string;
  owner: string;
  type: string;
  size: number;
  uploadedAt: string;
};

const BUCKET_RE = /^[a-zA-Z0-9][a-zA-Z0-9_-]{0,63}$/;

const manifestCache = new Map<string, Map<string, FileMeta>>();

function bucketDir(bucket: string) {
  return join(ROOT, bucket);
}

function filesDir(bucket: string) {
  return join(bucketDir(bucket), "files");
}

function filePath(bucket: string, id: string) {
  return join(filesDir(bucket), id);
}

function manifestPath(bucket: string) {
  return join(bucketDir(bucket), "meta.json");
}

async function loadManifest(bucket: string): Promise<Map<string, FileMeta>> {
  const cached = manifestCache.get(bucket);
  if (cached) return cached;
  const map = new Map<string, FileMeta>();
  if (existsSync(manifestPath(bucket))) {
    const raw = JSON.parse(await readFile(manifestPath(bucket), "utf8")) as FileMeta[];
    for (const meta of raw) map.set(meta.id, meta);
  }
  manifestCache.set(bucket, map);
  return map;
}

async function saveManifest(bucket: string, map: Map<string, FileMeta>) {
  await mkdir(filesDir(bucket), { recursive: true });
  const tmp = manifestPath(bucket) + ".tmp";
  await writeFile(tmp, JSON.stringify([...map.values()], null, 2));
  await rename(tmp, manifestPath(bucket));
}

function json(data: unknown, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json; charset=utf-8", "cache-control": "no-store" },
  });
}

function error(message: string, status = 400) {
  return json({ error: message }, status);
}

function getBucket(url: URL): string | null {
  const bucket = (url.searchParams.get("bucket") ?? "").trim();
  return BUCKET_RE.test(bucket) ? bucket : null;
}

function encodeName(value: string) {
  return encodeURIComponent(value);
}

function decodeName(value: string | null) {
  if (!value) return "";
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function contentDisposition(name: string) {
  const ascii = name.replace(/[^\x20-\x7e]/g, "_").replace(/["\\]/g, "_");
  return `attachment; filename="${ascii}"; filename*=UTF-8''${encodeURIComponent(name)}`;
}

async function handleUpload(req: Request, url: URL): Promise<Response> {
  const bucket = getBucket(url);
  if (!bucket) return error("invalid bucket name");
  const name = decodeName(req.headers.get("x-file-name")).trim();
  const owner = decodeName(req.headers.get("x-owner")).trim() || "anonymous";
  const type = req.headers.get("x-content-type") || "application/octet-stream";
  if (!name) return error("missing file name");

  const body = await req.arrayBuffer();
  if (body.byteLength > MAX_UPLOAD_BYTES) return error("file too large", 413);
  let bytes = new Uint8Array(body);
  if (req.headers.get("x-encoding") === "gzip") {
    try {
      bytes = gunzipSync(bytes);
    } catch {
      return error("invalid gzip payload");
    }
  }

  const meta: FileMeta = {
    id: crypto.randomUUID(),
    name: name.slice(0, 255),
    owner: owner.slice(0, 64),
    type: type.slice(0, 128),
    size: bytes.byteLength,
    uploadedAt: new Date().toISOString(),
  };

  await mkdir(filesDir(bucket), { recursive: true });
  await writeFile(filePath(bucket, meta.id), bytes);
  const manifest = await loadManifest(bucket);
  manifest.set(meta.id, meta);
  await saveManifest(bucket, manifest);
  return json(meta, 201);
}

async function handleDownload(bucket: string, meta: FileMeta): Promise<Response> {
  const file = Bun.file(filePath(bucket, meta.id));
  if (!(await file.exists())) return error("file not found on disk", 404);
  const bytes = new Uint8Array(await file.arrayBuffer());
  return new Response(gzipSync(bytes), {
    headers: {
      "content-type": "application/octet-stream",
      "content-encoding": "gzip",
      "content-disposition": contentDisposition(meta.name),
      "x-file-name": encodeName(meta.name),
      "cache-control": "no-store",
    },
  });
}

async function handlePatch(req: Request, bucket: string, manifest: Map<string, FileMeta>, id: string): Promise<Response> {
  const meta = manifest.get(id);
  if (!meta) return error("file not found", 404);
  const body = (await req.json().catch(() => ({}))) as { name?: string; owner?: string };
  if (typeof body.name === "string" && body.name.trim()) meta.name = body.name.trim().slice(0, 255);
  if (typeof body.owner === "string" && body.owner.trim()) meta.owner = body.owner.trim().slice(0, 64);
  manifest.set(id, meta);
  await saveManifest(bucket, manifest);
  return json(meta);
}

async function handleDelete(bucket: string, manifest: Map<string, FileMeta>, id: string): Promise<Response> {
  if (!manifest.has(id)) return error("file not found", 404);
  manifest.delete(id);
  await saveManifest(bucket, manifest);
  await rm(filePath(bucket, id), { force: true });
  return json({ ok: true });
}

async function handleListBuckets(): Promise<Response> {
  const entries = await readdir(ROOT, { withFileTypes: true }).catch(() => []);
  const buckets = entries.filter((e) => e.isDirectory() && BUCKET_RE.test(e.name)).map((e) => e.name);
  return json({ buckets: buckets.sort() });
}

const STATIC: Record<string, string> = {
  "/": join(import.meta.dir, "public", "index.html"),
  "/app.js": join(import.meta.dir, "public", "app.js"),
  "/style.css": join(import.meta.dir, "public", "style.css"),
};

Bun.serve({
  port: PORT,
  async fetch(req) {
    const url = new URL(req.url);
    const method = req.method.toUpperCase();

    const staticFile = STATIC[url.pathname];
    if (method === "GET" && staticFile && existsSync(staticFile)) {
      return new Response(Bun.file(staticFile), {
        headers: { "cache-control": "no-store" },
      });
    }

    const pathParts = url.pathname.split("/").filter(Boolean);
    if (method === "GET" && pathParts.length === 1 && BUCKET_RE.test(pathParts[0])) {
      return new Response(Bun.file(STATIC["/"]), {
        headers: { "cache-control": "no-store" },
      });
    }

    if (method === "GET" && url.pathname === "/api/buckets") return handleListBuckets();

    const fileMatch = url.pathname.match(/^\/api\/files\/([a-f0-9-]+)$/);
    if (fileMatch) {
      const bucket = getBucket(url);
      if (!bucket) return error("invalid bucket name");
      const manifest = await loadManifest(bucket);
      const meta = manifest.get(fileMatch[1]);
      if (!meta) return error("file not found", 404);
      if (method === "GET") return handleDownload(bucket, meta);
      if (method === "PATCH") return handlePatch(req, bucket, manifest, fileMatch[1]);
      if (method === "DELETE") return handleDelete(bucket, manifest, fileMatch[1]);
    }

    if (method === "GET" && url.pathname === "/api/files") {
      const bucket = getBucket(url);
      if (!bucket) return error("invalid bucket name");
      const manifest = await loadManifest(bucket);
      return json({ files: [...manifest.values()].sort((a, b) => b.uploadedAt.localeCompare(a.uploadedAt)) });
    }

    if (method === "POST" && url.pathname === "/api/upload") return handleUpload(req, url);

    return error("not found", 404);
  },
});

console.log(`paoloose share running at http://localhost:${PORT}`);
