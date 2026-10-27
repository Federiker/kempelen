// Stages the pinned Kokoro model, voices and their notices into a speech data folder, checking
// every file's size and SHA-256 (crates/kempelen/assets/speech-assets.json). kempelen itself never
// downloads anything: this is for development, CI and the installers of programs that use it.
//
//   bun scripts/speech.ts [FOLDER]
//
// FOLDER defaults to KEMPELEN_DATA_DIR, else tmp/speech. The first run downloads about 349 MB;
// files already staged and verified are kept. Programs that stage the same files (manuzio's
// installer) import `stageSpeechAssets` from here, or read the manifest at `MANIFEST_PATH`.

import { createHash, randomUUID } from "node:crypto";
import { createReadStream } from "node:fs";
import { mkdir, open, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pipeline } from "node:stream/promises";

const ROOT = resolve(import.meta.dirname, "..");
/** The pinned speech files: path in the data folder, URL, size and SHA-256. */
export const MANIFEST_PATH = join(ROOT, "crates", "kempelen", "assets", "speech-assets.json");
/** The notice staged beside them as NOTICE.txt. */
export const NOTICE_PATH = join(ROOT, "crates", "kempelen", "assets", "speech-NOTICE.txt");
/** Where `bun scripts/speech.ts` stages without an argument or KEMPELEN_DATA_DIR. */
export const DEFAULT_DESTINATION = join(ROOT, "tmp", "speech");

export interface SpeechAsset {
  path: string;
  url: string;
  size: number;
  sha256: string;
}

export function speechManifest(data: unknown): SpeechAsset[] {
  if (
    typeof data !== "object" ||
    data === null ||
    !("files" in data) ||
    !Array.isArray(data.files) ||
    data.files.length === 0
  )
    throw new Error("Invalid speech manifest");
  const paths = new Set<string>();
  return data.files.map((file: unknown) => {
    if (
      typeof file !== "object" ||
      file === null ||
      !("path" in file) ||
      typeof file.path !== "string" ||
      !/^(models|voices|notices)\/[a-zA-Z0-9_.-]+$/.test(file.path) ||
      file.path.includes("..") ||
      paths.has(file.path) ||
      !("url" in file) ||
      typeof file.url !== "string" ||
      !("size" in file) ||
      typeof file.size !== "number" ||
      !Number.isSafeInteger(file.size) ||
      file.size <= 0 ||
      !("sha256" in file) ||
      typeof file.sha256 !== "string" ||
      !/^[a-f0-9]{64}$/.test(file.sha256)
    )
      throw new Error("Invalid speech asset");
    const url = new URL(file.url);
    if (
      url.protocol !== "https:" ||
      url.username ||
      url.password ||
      !["huggingface.co", "raw.githubusercontent.com"].includes(url.hostname)
    )
      throw new Error("Unsafe speech asset URL");
    paths.add(file.path);
    return {
      path: file.path,
      url: file.url,
      size: file.size,
      sha256: file.sha256,
    };
  });
}

async function verified(path: string, asset: SpeechAsset): Promise<boolean> {
  try {
    const info = await stat(path);
    if (!info.isFile() || info.size !== asset.size) return false;
    const hash = createHash("sha256");
    for await (const chunk of createReadStream(path)) {
      if (!Buffer.isBuffer(chunk)) throw new TypeError(`Expected binary speech asset: ${path}`);
      hash.update(chunk);
    }
    return hash.digest("hex") === asset.sha256;
  } catch (error) {
    if (error instanceof Error && "code" in error && error.code === "ENOENT") return false;
    throw error;
  }
}

export interface StageOptions {
  /** The manifest to stage; kempelen's by default. */
  manifest?: string;
  /** The notice written beside the files as NOTICE.txt; kempelen's by default. */
  notice?: string;
  fetcher?: typeof fetch;
}

/**
 * Download into `destination` every file of the manifest that is missing or fails its check, then
 * write the manifest (manifest.json) and the notice (NOTICE.txt) beside them.
 */
export async function stageSpeechAssets(destination: string, options: StageOptions = {}): Promise<void> {
  const fetcher = options.fetcher ?? fetch;
  const manifest = await readFile(options.manifest ?? MANIFEST_PATH, "utf8");
  const assets = speechManifest(JSON.parse(manifest));
  for (const asset of assets) {
    const path = join(destination, ...asset.path.split("/"));
    if (await verified(path, asset)) continue;
    await mkdir(dirname(path), { recursive: true });
    const temporary = `${path}.${randomUUID()}.part`;
    try {
      console.log(`Preparing speech asset: ${asset.path}`);
      const response = await fetcher(asset.url, {
        signal: AbortSignal.timeout(300_000),
      });
      if (!response.ok || !response.body)
        throw new Error(`Speech download failed: HTTP ${response.status} (${asset.path})`);
      const hash = createHash("sha256");
      let size = 0;
      async function* chunks() {
        if (!response.body) throw new Error("Missing speech download body");
        const reader = response.body.getReader();
        try {
          while (true) {
            const { done, value } = await reader.read();
            if (done) break;
            size += value.length;
            if (size > asset.size) throw new Error(`Speech asset too large: ${asset.path}`);
            hash.update(value);
            yield value;
          }
        } finally {
          await reader.cancel();
          reader.releaseLock();
        }
      }
      const file = await open(temporary, "wx");
      try {
        await pipeline(chunks(), file.createWriteStream());
      } finally {
        await file.close();
      }
      if (size !== asset.size || hash.digest("hex") !== asset.sha256)
        throw new Error(`Speech checksum mismatch: ${asset.path}`);
      await rename(temporary, path);
    } finally {
      await rm(temporary, { force: true });
    }
  }
  await mkdir(destination, { recursive: true });
  await writeFile(join(destination, "NOTICE.txt"), await readFile(options.notice ?? NOTICE_PATH, "utf8"));
  await writeFile(join(destination, "manifest.json"), manifest);
}

if (import.meta.main) {
  const destination = resolve(process.argv[2] || process.env.KEMPELEN_DATA_DIR || DEFAULT_DESTINATION);
  await stageSpeechAssets(destination);
  console.log(`Speech assets staged in ${destination}`);
}
