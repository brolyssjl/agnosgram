/**
 * Shared harness for conformance tests (`*.conformance.test.ts`). These
 * exercise the CLI surface by spawning a real binary - never by importing
 * command internals - so the exact same test suite can be pointed at any
 * implementation of the frozen surface via `AGNOSGRAM_BIN` (see
 * CONTRIBUTING.md). Excluded from the published package; see tsconfig.json.
 */
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = fileURLToPath(new URL(".", import.meta.url));
/** `dist/cli.js` next to this file's own compiled location (`dist/conformance/harness.js`). */
const DEFAULT_CLI_JS = resolve(HERE, "..", "cli.js");

export interface RunCliOptions {
  cwd: string;
  input?: string;
  env?: NodeJS.ProcessEnv;
}

export interface CliRun {
  stdout: string;
  stderr: string;
  status: number;
}

/**
 * Run one CLI invocation against `AGNOSGRAM_BIN` (default: `node dist/cli.js`
 * from this checkout). `AGNOSGRAM_BIN` may be a directly-executable binary
 * (e.g. an installed npm shim) or any `command arg...` string - split on
 * whitespace, so a path containing spaces isn't supported (matches how the
 * env var is documented and used in CONTRIBUTING.md).
 */
export function runCli(args: string[], opts: RunCliOptions): CliRun {
  const override = process.env.AGNOSGRAM_BIN;
  const [command, ...prefixArgs] = override ? override.trim().split(/\s+/) : ["node", DEFAULT_CLI_JS];
  const result = spawnSync(command!, [...prefixArgs, ...args], {
    cwd: opts.cwd,
    input: opts.input,
    encoding: "utf8",
    env: { ...process.env, ...opts.env },
  });
  if (result.error) throw result.error;
  return { stdout: result.stdout, stderr: result.stderr, status: result.status ?? -1 };
}
