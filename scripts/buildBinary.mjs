#!/usr/bin/env node
/**
 * Build a single-file `agnosgram` executable, for install.sh and Node-less
 * machines. Two paths, in order of preference:
 *
 *   1. `bun build --compile` - zero extra dependencies (bun is a separate
 *      runtime, not an npm package), and Agnosgram has no runtime deps, so
 *      bun bundles our ESM `dist/` output directly. Used automatically when
 *      `bun` is on PATH.
 *   2. Node's built-in Single Executable Applications (SEA) - works with
 *      plain Node. SEA embeds exactly one file, so the multi-file ESM
 *      `dist/` output is first bundled into a single CJS file with esbuild
 *      (devDependency, build-time only - never shipped or used at runtime,
 *      so it doesn't touch the zero-runtime-deps rule). Injecting that
 *      bundle into a copied node binary needs `postject`; installed ad hoc
 *      (`npm install --no-save postject`) rather than as a project dep.
 *
 * Either way, `npm run build` must have already produced `dist/cli.js`.
 */
import * as esbuild from "esbuild";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, mkdirSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const entry = join(root, "dist", "cli.js");
const outDir = join(root, "dist-bin");
const outFile = join(outDir, process.platform === "win32" ? "agnosgram.exe" : "agnosgram");

function hasBun() {
  const res = spawnSync("bun", ["--version"], { stdio: "ignore" });
  return res.status === 0;
}

function buildWithBun() {
  mkdirSync(outDir, { recursive: true });
  execFileSync("bun", ["build", "--compile", entry, "--outfile", outFile], {
    stdio: "inherit",
    cwd: root,
  });
  process.stdout.write(`Built ${outFile} with bun build --compile\n`);
}

async function buildWithNodeSea() {
  process.stdout.write("bun not found - falling back to Node SEA (requires `postject`).\n");

  // SEA embeds one file; bundle the ESM dist/ output graph into a single CJS entry first.
  const bundlePath = join(root, "dist", "cli.sea.cjs");
  await esbuild.build({
    entryPoints: [entry],
    outfile: bundlePath,
    bundle: true,
    platform: "node",
    format: "cjs",
    target: "node20",
  });

  const seaConfigPath = join(root, "sea-config.json");
  const seaBlobPath = join(root, "dist", "sea-prep.blob");
  writeFileSync(
    seaConfigPath,
    JSON.stringify(
      { main: bundlePath, output: seaBlobPath, disableExperimentalSEAWarning: true },
      null,
      2,
    ),
  );
  mkdirSync(outDir, { recursive: true });

  execFileSync(process.execPath, ["--experimental-sea-config", seaConfigPath], {
    stdio: "inherit",
    cwd: root,
  });

  execFileSync(
    "node",
    [
      "-e",
      `require("fs").copyFileSync(${JSON.stringify(process.execPath)}, ${JSON.stringify(outFile)})`,
    ],
    { stdio: "inherit" },
  );
  // The source node binary may be read-only (e.g. a Nix store path); copyFileSync can
  // carry that mode over, and postject needs to write to the copy.
  chmodSync(outFile, 0o755);

  // macOS won't let postject modify a signed binary in place - drop the signature first,
  // then re-sign ad hoc afterward so the binary is still runnable on this machine.
  if (process.platform === "darwin") {
    spawnSync("codesign", ["--remove-signature", outFile], { stdio: "inherit" });
  }

  const postjectArgs = [
    "-y",
    "postject",
    outFile,
    "NODE_SEA_BLOB",
    seaBlobPath,
    "--sentinel-fuse",
    "NODE_SEA_FUSE_fce680ab2cc467b6e072b8b5df1996b2",
  ];
  if (process.platform === "darwin") postjectArgs.push("--macho-segment-name", "NODE_SEA");
  const res = spawnSync("npx", postjectArgs, { stdio: "inherit", cwd: root });
  if (res.status !== 0) {
    process.stderr.write(
      "postject failed - install it first: npm install --no-save postject\n" +
        "(kept as an ad-hoc dependency, not a project devDependency, per the zero-runtime-deps rule)\n",
    );
    process.exitCode = 1;
    return;
  }
  if (process.platform === "darwin") {
    spawnSync("codesign", ["--sign", "-", outFile], { stdio: "inherit" });
  }
  process.stdout.write(`Built ${outFile} with Node SEA\n`);
}

if (!existsSync(entry)) {
  process.stderr.write(`${entry} not found - run \`npm run build\` first.\n`);
  process.exit(1);
}

if (hasBun()) {
  buildWithBun();
} else {
  await buildWithNodeSea();
}
