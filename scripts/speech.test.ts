// The speech staging (scripts/speech.ts), with a fake fetch: nothing is downloaded.

import { describe, expect, it, mock } from "bun:test";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { MANIFEST_PATH, speechManifest, stageSpeechAssets } from "./speech.ts";

const bytes = "test";
const asset = {
  path: "voices/test.bin",
  url: "https://huggingface.co/model/file",
  size: 4,
  sha256: createHash("sha256").update(bytes).digest("hex"),
};

/** A manifest and a notice in a fresh folder, and the folder to stage into. */
async function fixture(speechAsset = asset) {
  const root = await mkdtemp(join(tmpdir(), "kempelen-stage-speech-"));
  const manifest = join(root, "speech-assets.json");
  const notice = join(root, "speech-NOTICE.txt");
  await writeFile(manifest, JSON.stringify({ files: [speechAsset] }));
  await writeFile(notice, "fixture attribution");
  return { root, manifest, notice, destination: join(root, "speech") };
}

function fetcherOf(body: () => Response | Promise<Response>) {
  return mock((() => Promise.resolve(body())) as unknown as typeof fetch);
}

describe("offline speech staging", () => {
  it("verifies multi-chunk binary assets and repairs same-size corruption", async () => {
    const content = Buffer.alloc(256 * 1024 + 1, 0xff);
    const { root, manifest, notice, destination } = await fixture({
      ...asset,
      size: content.length,
      sha256: createHash("sha256").update(content).digest("hex"),
    });
    const fetcher = fetcherOf(() => new Response(Uint8Array.from(content)));
    const options = { manifest, notice, fetcher: fetcher as unknown as typeof fetch };
    try {
      await stageSpeechAssets(destination, options);
      await stageSpeechAssets(destination, options);
      expect(fetcher).toHaveBeenCalledTimes(1);
      const path = join(destination, "voices", "test.bin");
      await writeFile(path, Buffer.alloc(content.length));
      await stageSpeechAssets(destination, options);
      expect(fetcher).toHaveBeenCalledTimes(2);
      expect(await readFile(path)).toEqual(content);
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  it("stages verified assets and reuses them without network access, repairing corruption", async () => {
    const { root, manifest, notice, destination } = await fixture();
    const fetcher = fetcherOf(() => new Response(bytes));
    const options = { manifest, notice, fetcher: fetcher as unknown as typeof fetch };
    try {
      await stageSpeechAssets(destination, options);
      await stageSpeechAssets(destination, options);
      expect(fetcher).toHaveBeenCalledTimes(1);
      const path = join(destination, "voices", "test.bin");
      await writeFile(path, "evil");
      await stageSpeechAssets(destination, options);
      expect(fetcher).toHaveBeenCalledTimes(2);
      expect(await readFile(path, "utf8")).toBe(bytes);
      expect(JSON.parse(await readFile(join(destination, "manifest.json"), "utf8"))).toEqual({ files: [asset] });
      expect(await readFile(join(destination, "NOTICE.txt"), "utf8")).toBe("fixture attribution");
    } finally {
      await rm(root, { recursive: true, force: true });
    }
  });

  it.each(["checksum", "short", "large", "http", "network"])(
    "rejects %s failures without leaving partial assets",
    async (failure) => {
      const { root, manifest, notice, destination } = await fixture();
      const fetcher = mock((() => {
        if (failure === "network") return Promise.reject(new TypeError("offline"));
        return Promise.resolve(
          new Response(failure === "short" ? "t" : failure === "large" ? "oversized" : "evil", {
            status: failure === "http" ? 503 : 200,
          }),
        );
      }) as unknown as typeof fetch);
      try {
        await expect(
          stageSpeechAssets(destination, { manifest, notice, fetcher: fetcher as unknown as typeof fetch }),
        ).rejects.toThrow();
        expect(await readdir(join(destination, "voices"))).toEqual([]);
      } finally {
        await rm(root, { recursive: true, force: true });
      }
    },
  );

  it.each(["../escape", "voices/../escape", "/absolute", "voices/nested/file", "voices\\file"])(
    "refuses unsafe manifest path %s",
    (path) => {
      expect(() => speechManifest({ files: [{ ...asset, path }] })).toThrow();
    },
  );

  it("refuses duplicate paths, invalid checksums and external URLs", () => {
    expect(() => speechManifest({ files: [] })).toThrow();
    expect(() => speechManifest({ files: [asset, asset] })).toThrow();
    expect(() => speechManifest({ files: [{ ...asset, sha256: "" }] })).toThrow();
    expect(() => speechManifest({ files: [{ ...asset, url: "http://huggingface.co/model" }] })).toThrow();
    expect(() => speechManifest({ files: [{ ...asset, url: "https://example.test/model" }] })).toThrow();
  });

  const voices = resolve(import.meta.dirname, "..", "crates", "kempelen", "src", "voice.rs");
  it.skipIf(!existsSync(voices))(
    "includes precisely every supported Rust voice and a pinned full-precision model",
    async () => {
      const files = speechManifest(JSON.parse(await readFile(MANIFEST_PATH, "utf8")));
      const rust = await readFile(voices, "utf8");
      const names = [...rust.matchAll(/\brename = "([^"]+)"/g)].map((m) => `voices/${m[1]}.bin`);
      expect(
        files
          .filter((f) => f.path.startsWith("voices/"))
          .map((f) => f.path)
          .sort(),
      ).toEqual(names.sort());
      expect(names).toHaveLength(45);
      expect(files.filter((f) => f.path.startsWith("models/")).map((f) => f.path)).toEqual(["models/model.onnx"]);
      expect(files.filter((f) => f.path.startsWith("notices/"))).toHaveLength(2);
      for (const file of files.filter((f) => f.url.startsWith("https://huggingface.co/"))) {
        expect(file.url).toMatch(/\/resolve\/[a-f0-9]{40}\//);
      }
    },
  );
});
