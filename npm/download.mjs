// Lazy binary fetcher, invoked from bin/flowleap on first run. There is
// deliberately no npm install script: install-time code is what npm's
// allow-scripts guard exists to flag. The binary is downloaded from the
// GitHub release matching this package's version and verified against the
// release's checksums.txt before it is ever executed.
//
// All human-facing output goes to stderr — the first run may be
// `flowleap --json …` piped by an agent, and stdout must stay parseable.

import {
  existsSync,
  mkdirSync,
  readFileSync,
  renameSync,
  writeFileSync,
} from "fs";
import { createHash } from "crypto";
import { join, dirname } from "path";
import { homedir } from "os";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO = "flowleap-ai/flowleap-cli";
const isWindows = process.platform === "win32";
const binaryName = isWindows ? "flowleap-native.exe" : "flowleap-native";

// Release tags are named "v<version>"; the publish workflow stamps the npm
// version from the tag, so package.json is always the source of truth.
const pkg = JSON.parse(readFileSync(join(__dirname, "package.json"), "utf8"));
const VERSION = `v${pkg.version}`;

const PLATFORM_MAP = {
  "darwin-arm64": "flowleap-darwin-aarch64",
  "darwin-x64": "flowleap-darwin-x86_64",
  "linux-arm64": "flowleap-linux-aarch64",
  "linux-x64": "flowleap-linux-x86_64",
  "win32-x64": "flowleap-windows-x86_64.exe",
};

// Preferred location is inside the package (fast path, removed on uninstall);
// the per-user cache is the fallback when the package dir isn't writable
// (e.g. sudo-installed global node).
function candidates() {
  const cacheBase = process.env.XDG_CACHE_HOME || join(homedir(), ".cache");
  return [
    join(__dirname, "bin", binaryName),
    join(cacheBase, "flowleap", VERSION, binaryName),
  ];
}

export function findBinary() {
  return candidates().find(existsSync) ?? null;
}

function formatBytes(n) {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${n.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

function progressReporter(total) {
  const isTTY = Boolean(process.stderr.isTTY);
  const hasTotal = Number.isFinite(total) && total > 0;
  let received = 0;
  let lastRenderAt = 0;

  return {
    update(chunkSize) {
      received += chunkSize;
      if (!isTTY) return; // non-TTY (agents, CI): stay quiet beyond the one-line notice
      const now = Date.now();
      if (now - lastRenderAt < 100 && received !== total) return;
      lastRenderAt = now;
      const line = hasTotal
        ? `  ${formatBytes(received)} / ${formatBytes(total)} (${Math.floor((received / total) * 100)}%)`
        : `  ${formatBytes(received)}…`;
      process.stderr.write(`\r${line}`);
    },
    finish() {
      if (isTTY) process.stderr.write("\n");
    },
  };
}

async function fetchOk(url) {
  const res = await fetch(url, { redirect: "follow" });
  if (!res.ok) {
    throw new Error(`download failed: HTTP ${res.status} for ${url}`);
  }
  return res;
}

async function downloadBytes(url) {
  const res = await fetchOk(url);
  const total = parseInt(res.headers.get("content-length") || "", 10);
  const progress = progressReporter(total);
  const reader = res.body.getReader();
  const chunks = [];
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    chunks.push(value);
    progress.update(value.length);
  }
  progress.finish();
  return Buffer.concat(chunks);
}

// checksums.txt lines look like "<sha256>  <asset-dir>/<asset>".
function expectedChecksum(text, asset) {
  for (const line of text.split("\n")) {
    const [hash, file] = line.trim().split(/\s+/);
    if (file && (file === asset || file.endsWith(`/${asset}`))) return hash;
  }
  throw new Error(`no checksum entry for ${asset} in checksums.txt`);
}

export async function ensureBinary() {
  const existing = findBinary();
  if (existing) return existing;

  const key = `${process.platform}-${process.arch}`;
  const asset = PLATFORM_MAP[key];
  if (!asset) {
    throw new Error(
      `unsupported platform ${key} (supported: ${Object.keys(PLATFORM_MAP).join(", ")})`
    );
  }

  process.stderr.write(`flowleap: first run — downloading ${VERSION} for ${key}…\n`);
  const base = `https://github.com/${REPO}/releases/download/${VERSION}`;
  const [bytes, checksums] = await Promise.all([
    downloadBytes(`${base}/${asset}`),
    fetchOk(`${base}/checksums.txt`).then((r) => r.text()),
  ]);

  const expected = expectedChecksum(checksums, asset);
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== expected) {
    throw new Error(
      `sha256 mismatch for ${asset}: expected ${expected}, got ${actual} — refusing to install`
    );
  }

  for (const target of candidates()) {
    try {
      mkdirSync(dirname(target), { recursive: true });
      const tmp = `${target}.tmp-${process.pid}`;
      writeFileSync(tmp, bytes, { mode: 0o755 });
      renameSync(tmp, target);
      process.stderr.write(`flowleap: sha256 verified, installed to ${target}\n`);
      return target;
    } catch (err) {
      if (["EACCES", "EPERM", "EROFS"].includes(err.code)) continue;
      throw err;
    }
  }
  throw new Error("no writable location for the flowleap binary");
}
