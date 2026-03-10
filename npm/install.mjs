import { createWriteStream, chmodSync, mkdirSync, existsSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import { get } from "https";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO = "abdullahatrash/flowleap-cli";
const VERSION = "v0.1.0";

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

function download(url) {
  return new Promise((resolve, reject) => {
    get(url, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        download(res.headers.location).then(resolve).catch(reject);
        return;
      }
      if (res.statusCode !== 200) {
        reject(new Error(`Download failed: HTTP ${res.statusCode}`));
        return;
      }
      const file = createWriteStream(binPath);
      res.pipe(file);
      file.on("finish", () => {
        file.close();
        if (!isWindows) {
          chmodSync(binPath, 0o755);
        }
        resolve();
      });
      file.on("error", reject);
    }).on("error", reject);
  });
}

download(url)
  .then(() => console.log("flowleap installed successfully!"))
  .catch((err) => {
    console.error(`Failed to install flowleap: ${err.message}`);
    process.exit(1);
  });
