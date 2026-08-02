import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runCli } from "../conformance/harness.js";

let root: string;

function rec(id: string, type: string, scope: string, extra = ""): string {
  return `---\nid: ${id}\ntype: ${type}\nscope: [${scope}]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\n${extra || `Body of ${id}.`}\n`;
}

beforeEach(() => {
  root = mkdtempSync(join(tmpdir(), "agnos-pack-"));
  assert.equal(runCli(["init", "--adapt", "none"], { cwd: root }).status, 0);
  writeFileSync(
    join(root, ".agnosgram", "lessons", "pitfalls.md"),
    `# Pitfalls\n\n${rec("LES-001", "pitfall", "core")}\n${rec("LES-002", "pitfall", "backend")}\n`,
  );
  writeFileSync(
    join(root, ".agnosgram", "lessons", "conventions.md"),
    `# Conventions\n\n${rec("CON-001", "convention", "core")}\n`,
  );
  writeFileSync(
    join(root, ".agnosgram", "decisions", "0003-example.md"),
    rec("DEC-0003", "decision", "core"),
  );
});
afterEach(() => {
  rmSync(root, { recursive: true, force: true });
});

test("pack includes status.md verbatim and lessons, but not decisions, when unscoped", () => {
  const res = runCli(["pack"], { cwd: root });
  assert.equal(res.status, 0);
  const status = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8").trim();
  assert.ok(res.stdout.includes(status));
  assert.ok(res.stdout.includes("LES-001"));
  assert.ok(res.stdout.includes("CON-001"));
  assert.ok(!res.stdout.includes("DEC-0003"));
});

test("pack --scope includes matching decisions and filters lessons by scope", () => {
  const res = runCli(["pack", "--scope", "core"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("LES-001"));
  assert.ok(!res.stdout.includes("LES-002")); // scope backend, excluded
  assert.ok(res.stdout.includes("CON-001"));
  assert.ok(res.stdout.includes("DEC-0003"));
});

test("pack --budget greedily drops whole records and lists them in an Omitted section", () => {
  const res = runCli(["pack", "--budget", "60"], { cwd: root });
  assert.equal(res.status, 0);
  assert.ok(res.stdout.includes("## Omitted (budget)"));
});

test("pack --json returns the pinned shape", () => {
  const res = runCli(["pack", "--json"], { cwd: root });
  assert.equal(res.status, 0);
  const parsed = JSON.parse(res.stdout);
  assert.equal(parsed.version, 1);
  assert.equal(typeof parsed.budget, "number");
  assert.equal(typeof parsed.tokens, "number");
  assert.equal(typeof parsed.status, "string");
  assert.ok(Array.isArray(parsed.records));
  assert.ok(Array.isArray(parsed.omitted));
});

test("pack --budget overrides config pack_budget, which overrides the 2000 default", () => {
  const configPath = join(root, ".agnosgram", "config.yml");
  const config = readFileSync(configPath, "utf8");
  writeFileSync(configPath, config + "pack_budget: 60\n");

  let res = runCli(["pack", "--json"], { cwd: root });
  assert.equal(JSON.parse(res.stdout).budget, 60);

  res = runCli(["pack", "--json", "--budget", "80"], { cwd: root });
  assert.equal(JSON.parse(res.stdout).budget, 80);
});

test("pack default budget is 2000 when nothing overrides it", () => {
  const res = runCli(["pack", "--json"], { cwd: root });
  assert.equal(JSON.parse(res.stdout).budget, 2000);
});

test("pack exits non-zero with guidance when state/status.md is missing", () => {
  unlinkSync(join(root, ".agnosgram", "state", "status.md"));
  const res = runCli(["pack"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /status\.md.*doctor|init/s);
});

test("pack tokens stay within budget for a normal case, accounting for headings/joiners/footer", () => {
  // Enough records that the greedy loop must omit several, exercising the
  // heading + joiner + footer accounting, not just a single dropped record.
  const many = Array.from({ length: 20 }, (_, i) => {
    const id = `LES-1${String(i).padStart(2, "0")}`;
    return rec(id, "pitfall", "core", `Body of ${id}, padded so records cost a realistic number of tokens each.`);
  }).join("\n");
  writeFileSync(join(root, ".agnosgram", "lessons", "pitfalls.md"), `# Pitfalls\n\n${many}\n`);

  const res = runCli(["pack", "--budget", "200", "--json"], { cwd: root });
  const parsed = JSON.parse(res.stdout);
  assert.ok(parsed.tokens <= parsed.budget, `expected tokens (${parsed.tokens}) <= budget (${parsed.budget})`);
  assert.ok(parsed.omitted.length > 0, "expected this store to overflow a 200-token budget");
});

test("pack never surfaces meta/friction.md content, even when it exists", () => {
  mkdirSync(join(root, ".agnosgram", "meta"), { recursive: true });
  writeFileSync(
    join(root, ".agnosgram", "meta", "friction.md"),
    `# Friction\n\n${rec("FRI-001", "friction", "cli", "This is tool friction, not host-project memory.")}\n`,
  );
  const res = runCli(["pack", "--json"], { cwd: root });
  assert.ok(!res.stdout.includes("FRI-001"));
  assert.ok(!res.stdout.includes("tool friction, not host-project memory"));
});

test("pack's omitted footer is capped and summarizes the rest instead of listing every record", () => {
  const many = Array.from({ length: 20 }, (_, i) => {
    const id = `LES-2${String(i).padStart(2, "0")}`;
    return rec(id, "pitfall", "core", `Body of ${id}, padded so records cost a realistic number of tokens each.`);
  }).join("\n");
  writeFileSync(join(root, ".agnosgram", "lessons", "pitfalls.md"), `# Pitfalls\n\n${many}\n`);

  const res = runCli(["pack", "--budget", "150"], { cwd: root });
  assert.ok(res.stdout.includes("## Omitted (budget)"));
  assert.ok(/\.\.\.and \d+ more/.test(res.stdout), "expected a capped omitted footer with an '...and N more' tail");
});

test("FRI-001: pack --budget -1 gives the existing validation error, not a raw parseArgs crash", () => {
  const res = runCli(["pack", "--budget", "-1"], { cwd: root });
  assert.notEqual(res.status, 0);
  assert.match(res.stderr, /--budget must be a positive integer, got "-1"/);
  assert.ok(!/at Object|node:internal|ERR_PARSE_ARGS/.test(res.stderr));
});
