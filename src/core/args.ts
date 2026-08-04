/**
 * Shared CLI argument parsing. Every command routes through `parseCliArgs`
 * instead of calling `node:util`'s `parseArgs` directly (FRI-001): it fixes
 * `parseArgs`'s "ambiguous option value" guard, which raw-crashes on any
 * dash-leading option value (`--budget -1`, `--learned "--x"`), and turns
 * every remaining parseArgs failure into a clean `UserError` instead of an
 * uncaught stack trace.
 */
import { parseArgs, type ParseArgsConfig } from "node:util";
import { UserError } from "./output.js";

type OptionsMap = NonNullable<ParseArgsConfig["options"]>;
type OptionSpec = OptionsMap[string];

function knownFlags(options: OptionsMap): Set<string> {
  const flags = new Set<string>();
  for (const [name, spec] of Object.entries(options)) {
    flags.add(`--${name}`);
    if (spec.short) flags.add(`-${spec.short}`);
  }
  return flags;
}

/**
 * Node's `parseArgs` throws `ERR_PARSE_ARGS_INVALID_OPTION_VALUE` whenever a
 * string option's value starts with a dash, on the theory that the value was
 * probably a forgotten flag rather than a real value. That's the right call
 * when the following token really is a known flag (a missing value, e.g.
 * `--scope --budget 500`), but wrong whenever it's a negative number or an
 * arbitrary string the user wants to store verbatim - both legitimate.
 *
 * Rewriting `--opt VALUE` to `--opt=VALUE` up front sidesteps the guard
 * entirely (parseArgs never second-guesses an `=`-joined value), but only
 * when the next token isn't itself a recognized flag - that case is left
 * alone so a genuinely missing value still surfaces as an error below.
 */
function disambiguate(args: readonly string[], options: OptionsMap): string[] {
  const flags = knownFlags(options);
  const out: string[] = [];
  for (let i = 0; i < args.length; i++) {
    const tok = args[i]!;
    if (tok === "--") {
      // The POSIX end-of-options terminator: everything from here on is
      // positional, verbatim - never rewritten, and never treated as an
      // option's value (see the `next !== "--"` guard below).
      out.push(...args.slice(i));
      break;
    }
    if (tok.includes("=") || !tok.startsWith("-")) {
      out.push(tok);
      continue;
    }
    const name = tok.startsWith("--") ? tok.slice(2) : tok.length === 2 ? tok.slice(1) : undefined;
    const spec: OptionSpec | undefined =
      name !== undefined ? Object.entries(options).find(([n, s]) => n === name || s.short === name)?.[1] : undefined;
    const next = args[i + 1];
    if (
      spec?.type === "string" &&
      next !== undefined &&
      next !== "--" &&
      next.startsWith("-") &&
      next !== "-" &&
      !flags.has(next)
    ) {
      // Long options join with `=`; short options glue directly (`-b=-1` is
      // parsed literally as the value "=-1", not stripped - see the error
      // message's own suggested fix, `-b-XYZ`).
      out.push(tok.startsWith("--") ? `${tok}=${next}` : `${tok}${next}`);
      i++;
      continue;
    }
    out.push(tok);
  }
  return out;
}

/** First line only - parseArgs' multi-line "did you mean" hints read fine in
 * a terminal but are noise once folded into a one-line UserError. */
function firstLine(message: string): string {
  return message.split("\n")[0]!;
}

function isParseArgsError(err: unknown): err is Error & { code: string } {
  return (
    err instanceof Error &&
    "code" in err &&
    typeof (err as { code: unknown }).code === "string" &&
    (err as { code: string }).code.startsWith("ERR_PARSE_ARGS_")
  );
}

export function parseCliArgs<T extends ParseArgsConfig>(config: T): ReturnType<typeof parseArgs<T>> {
  const options = (config.options ?? {}) as OptionsMap;
  const rewritten = { ...config, args: disambiguate(config.args ?? [], options) } as T;
  try {
    return parseArgs(rewritten);
  } catch (err) {
    if (isParseArgsError(err)) {
      throw new UserError(firstLine(err.message));
    }
    throw err;
  }
}
