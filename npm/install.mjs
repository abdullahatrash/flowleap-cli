import { createWriteStream, chmodSync, mkdirSync, existsSync, readFileSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { get } from "https";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO = "abdullahatrash/flowleap-cli";

// Read version from package.json so the release tag stays in sync with the
// published npm version automatically. Assumes release tags are named "v<version>".
const pkg = JSON.parse(readFileSync(join(__dirname, "package.json"), "utf8"));
const VERSION = `v${pkg.version}`;

const PLATFORM_MAP = {
  "darwin-arm64": "flowleap-darwin-aarch64",
  "darwin-x64": "flowleap-darwin-x86_64",
  "linux-arm64": "flowleap-linux-aarch64",
  "linux-x64": "flowleap-linux-x86_64",
  "win32-x64": "flowleap-windows-x86_64.exe",
};

const key = `${process.platform}-${process.arch}`;
const asset = PLATFORM_MAP[key];

if (!asset) {
  console.error(`Unsupported platform: ${key}`);
  console.error(`Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`);
  process.exit(1);
}

const binDir = join(__dirname, "bin");
const isWindows = process.platform === "win32";
const binaryName = isWindows ? "flowleap-native.exe" : "flowleap-native";
const binPath = join(binDir, binaryName);

if (existsSync(binPath)) {
  process.exit(0);
}

mkdirSync(binDir, { recursive: true });

const url = `https://github.com/${REPO}/releases/download/${VERSION}/${asset}`;

console.log(`Downloading flowleap ${VERSION} for ${key}...`);

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

function renderBar(fraction, width = 24) {
  const filled = Math.max(0, Math.min(width, Math.round(fraction * width)));
  const empty = width - filled;
  return `[${"=".repeat(filled)}${filled > 0 && empty > 0 ? ">" : ""}${" ".repeat(Math.max(0, empty - (filled > 0 ? 1 : 0)))}]`;
}

function createProgressReporter(total) {
  const isTTY = Boolean(process.stdout.isTTY);
  const hasTotal = Number.isFinite(total) && total > 0;
  let received = 0;
  let lastRenderAt = 0;
  let lastMilestone = -1;

  const update = (chunkSize) => {
    received += chunkSize;
    const now = Date.now();

    if (isTTY) {
      // Throttle TTY redraws to ~10 Hz
      if (now - lastRenderAt < 100 && received !== total) return;
      lastRenderAt = now;
      if (hasTotal) {
        const fraction = received / total;
        const pct = Math.floor(fraction * 100);
        const line = `  ${renderBar(fraction)} ${formatBytes(received)} / ${formatBytes(total)} (${pct}%)`;
        process.stdout.write(`\r${line}`);
      } else {
        process.stdout.write(`\r  Downloaded ${formatBytes(received)}...`);
      }
    } else if (hasTotal) {
      // Non-TTY: emit milestone lines so CI logs stay readable
      const pct = Math.floor((received / total) * 100);
      const milestone = Math.floor(pct / 25) * 25;
      if (milestone > lastMilestone && milestone <= 100) {
        lastMilestone = milestone;
        console.log(`  ${milestone}% (${formatBytes(received)} / ${formatBytes(total)})`);
      }
    }
  };

  const finish = () => {
    if (isTTY) process.stdout.write("\n");
  };

  return { update, finish };
}

function download(url) {
  return new Promise((resolve, reject) => {
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        download(res.headers.location).then(resolve).catch(reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`Download failed: HTTP ${res.statusCode}`));
        return;
      }

      const total = parseInt(res.headers["content-length"] || "", 10);
      const progress = createProgressReporter(total);

      const file = createWriteStream(binPath);
      res.on("data", (chunk) => progress.update(chunk.length));
      res.pipe(file);
      file.on("finish", () => {
        file.close();
        progress.finish();
        if (!isWindows) {
          chmodSync(binPath, 0o755);
        }
        resolve();
      });
      file.on("error", (err) => {
        progress.finish();
        reject(err);
      });
      res.on("error", (err) => {
        progress.finish();
        reject(err);
      });
    }).on("error", reject);
  });
}

download(url)
  .then(() => console.log("flowleap installed successfully!"))
  .catch((err) => {
    console.error(`Failed to install flowleap: ${err.message}`);
    process.exit(1);
  });
