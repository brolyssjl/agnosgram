/**
 * Pluggable structured-output serializer. JSON is the default and the only
 * format CI should rely on; TOON is opt-in for token savings on uniform,
 * tabular payloads (record lists) and worse on small objects, so it is never
 * the default. Modeled on Gate's `src/serialize/index.ts` + `src/cli/output.ts`
 * so both CLIs present the same `--format` contract to agents.
 */
import { UserError } from "./output.js";
import { encodeToon } from "./toon.js";

export type Format = "json" | "toon";

export function isFormat(value: string): value is Format {
  return value === "json" || value === "toon";
}

/** Render a structured value in the requested format. */
export function serialize(value: unknown, format: Format): string {
  return format === "toon" ? encodeToon(value) : JSON.stringify(value, null, 2);
}

export interface FormatFlags {
  format?: string;
  json?: boolean;
}

/**
 * Resolve the output format from `--format <fmt>` / `--json` flags. `--format`
 * wins when both are given; `--json` is kept as the pre-existing shorthand for
 * `--format json` so earlier commands' flags keep working unchanged.
 */
export function resolveFormat(flags: FormatFlags): Format | "human" {
  if (typeof flags.format === "string") {
    if (!isFormat(flags.format)) {
      throw new UserError(`unknown --format "${flags.format}" (use json or toon)`);
    }
    return flags.format;
  }
  if (flags.json === true) return "json";
  return "human";
}
