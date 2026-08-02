import assert from "node:assert/strict";
import { test } from "node:test";
import { UserError } from "./output.js";
import { parseCliArgs } from "./args.js";

const STRING_OPT = { budget: { type: "string" as const } };

test("a dash-leading negative number is accepted as a string option's value", () => {
  const { values } = parseCliArgs({ args: ["--budget", "-1"], options: STRING_OPT, allowPositionals: false });
  assert.equal(values.budget, "-1");
});

test("a literal value starting with -- is accepted verbatim", () => {
  const { values } = parseCliArgs({
    args: ["--learned", "--x"],
    options: { learned: { type: "string" } },
    allowPositionals: false,
  });
  assert.equal(values.learned, "--x");
});

test("an already-disambiguated --opt=value form still works", () => {
  const { values } = parseCliArgs({ args: ["--budget=-1"], options: STRING_OPT, allowPositionals: false });
  assert.equal(values.budget, "-1");
});

test("a genuinely missing value (next token is a known flag) still raises a clean UserError", () => {
  assert.throws(
    () =>
      parseCliArgs({
        args: ["--scope", "--budget", "500"],
        options: { scope: { type: "string" }, budget: { type: "string" } },
        allowPositionals: false,
      }),
    (err) => {
      assert.ok(err instanceof UserError);
      assert.ok(!/at Object|node:internal|\.js:\d+/.test((err as Error).message));
      return true;
    },
  );
});

test("an unknown option raises a clean UserError, not a raw parseArgs crash", () => {
  assert.throws(
    () => parseCliArgs({ args: ["--nope", "x"], options: STRING_OPT, allowPositionals: false }),
    (err) => {
      assert.ok(err instanceof UserError);
      assert.match((err as Error).message, /Unknown option/);
      return true;
    },
  );
});

test("an unexpected positional raises a clean UserError when positionals are disallowed", () => {
  assert.throws(
    () => parseCliArgs({ args: ["foo"], options: STRING_OPT, allowPositionals: false }),
    (err) => {
      assert.ok(err instanceof UserError);
      return true;
    },
  );
});

test("a boolean option given =value raises a clean UserError instead of crashing", () => {
  assert.throws(
    () =>
      parseCliArgs({
        args: ["--json=1"],
        options: { json: { type: "boolean", default: false } },
        allowPositionals: false,
      }),
    (err) => {
      assert.ok(err instanceof UserError);
      return true;
    },
  );
});

test("a missing value at the end of argv raises a clean UserError", () => {
  assert.throws(
    () => parseCliArgs({ args: ["--budget"], options: STRING_OPT, allowPositionals: false }),
    (err) => {
      assert.ok(err instanceof UserError);
      return true;
    },
  );
});

test("short option aliases still disambiguate dash-leading values", () => {
  const { values } = parseCliArgs({
    args: ["-b", "-1"],
    options: { budget: { type: "string", short: "b" } },
    allowPositionals: false,
  });
  assert.equal(values.budget, "-1");
});

test("positionals pass through untouched", () => {
  const { positionals } = parseCliArgs({
    args: ["claude", "agents"],
    options: {},
    allowPositionals: true,
  });
  assert.deepEqual(positionals, ["claude", "agents"]);
});

test("a bare -- terminator stops rewriting: everything after it is untouched positionals", () => {
  const { values, positionals } = parseCliArgs({
    args: ["--", "--type", "-x"],
    options: { type: { type: "string" } },
    allowPositionals: true,
  });
  assert.equal(values.type, undefined);
  assert.deepEqual(positionals, ["--type", "-x"]);
});

test("-- is never swallowed as a string option's value", () => {
  // parseArgs itself cannot tell whether the bare "--" is the value for
  // --type or the end-of-options terminator, and genuinely errors either
  // way - the wrapper must not paper over that by treating "--" as a value,
  // it must only turn the resulting parseArgs error into a clean UserError.
  assert.throws(
    () =>
      parseCliArgs({
        args: ["--type", "--", "pitfall"],
        options: { type: { type: "string" } },
        allowPositionals: true,
      }),
    (err) => {
      assert.ok(err instanceof UserError);
      assert.ok(!/at Object|node:internal|\.js:\d+/.test((err as Error).message));
      return true;
    },
  );
});
