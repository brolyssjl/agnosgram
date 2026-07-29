import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, beforeEach, test } from "node:test";
import { runInit } from "./init.js";
import { runPack } from "./pack.js";

let root: string;
let cwd: string;
let out: string;
let origWrite: typeof process.stdout.write;

function rec(id: string, type: string, scope: string, extra = ""): string {
  return `---\nid: ${id}\ntype: ${type}\nscope: [${scope}]\nconfidence: high\ncreated: 2026-07-21\nlast_verified: 2026-07-21\nsource: journal/2026-07.md\n---\n${extra || `Body of ${id}.`}\n`;
}

beforeEach(() => {
  cwd = process.cwd();
  root = mkdtempSync(join(tmpdir(), "agnos-pack-"));
  process.chdir(root);
  runInit(["--adapt", "none"]);
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
  out = "";
  origWrite = process.stdout.write.bind(process.stdout);
  process.stdout.write = ((chunk: string | Uint8Array) => {
    out += chunk.toString();
    return true;
  }) as typeof process.stdout.write;
});
afterEach(() => {
  process.stdout.write = origWrite;
  process.chdir(cwd);
  rmSync(root, { recursive: true, force: true });
  process.exitCode = 0;
});

test("pack includes status.md verbatim and lessons, but not decisions, when unscoped", () => {
  runPack([]);
  const status = readFileSync(join(root, ".agnosgram", "state", "status.md"), "utf8").trim();
  assert.ok(out.includes(status));
  assert.ok(out.includes("LES-001"));
  assert.ok(out.includes("CON-001"));
  assert.ok(!out.includes("DEC-0003"));
});

test("pack --scope includes matching decisions and filters lessons by scope", () => {
  runPack(["--scope", "core"]);
  assert.ok(out.includes("LES-001"));
  assert.ok(!out.includes("LES-002")); // scope backend, excluded
  assert.ok(out.includes("CON-001"));
  assert.ok(out.includes("DEC-0003"));
});

test("pack --budget greedily drops whole records and lists them in an Omitted section", () => {
  runPack(["--budget", "60"]);
  assert.ok(out.includes("## Omitted (budget)"));
});

test("pack --json returns the pinned shape", () => {
  runPack(["--json"]);
  const parsed = JSON.parse(out);
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

  runPack(["--json"]);
  let parsed = JSON.parse(out);
  assert.equal(parsed.budget, 60);

  out = "";
  runPack(["--json", "--budget", "80"]);
  parsed = JSON.parse(out);
  assert.equal(parsed.budget, 80);
});

test("pack default budget is 2000 when nothing overrides it", () => {
  runPack(["--json"]);
  const parsed = JSON.parse(out);
  assert.equal(parsed.budget, 2000);
});

test("pack throws a guidance-carrying UserError when state/status.md is missing", () => {
  unlinkSync(join(root, ".agnosgram", "state", "status.md"));
  assert.throws(() => runPack([]), /status\.md.*doctor|init/s);
});

test("pack tokens stay within budget for a normal case, accounting for headings/joiners/footer", () => {
  // Enough records that the greedy loop must omit several, exercising the
  // heading + joiner + footer accounting, not just a single dropped record.
  const many = Array.from({ length: 20 }, (_, i) => {
    const id = `LES-1${String(i).padStart(2, "0")}`;
    return rec(id, "pitfall", "core", `Body of ${id}, padded so records cost a realistic number of tokens each.`);
  }).join("\n");
  writeFileSync(join(root, ".agnosgram", "lessons", "pitfalls.md"), `# Pitfalls\n\n${many}\n`);

  runPack(["--budget", "200", "--json"]);
  const parsed = JSON.parse(out);
  assert.ok(parsed.tokens <= parsed.budget, `expected tokens (${parsed.tokens}) <= budget (${parsed.budget})`);
  assert.ok(parsed.omitted.length > 0, "expected this store to overflow a 200-token budget");
});

test("pack's omitted footer is capped and summarizes the rest instead of listing every record", () => {
  const many = Array.from({ length: 20 }, (_, i) => {
    const id = `LES-2${String(i).padStart(2, "0")}`;
    return rec(id, "pitfall", "core", `Body of ${id}, padded so records cost a realistic number of tokens each.`);
  }).join("\n");
  writeFileSync(join(root, ".agnosgram", "lessons", "pitfalls.md"), `# Pitfalls\n\n${many}\n`);

  runPack(["--budget", "150"]);
  assert.ok(out.includes("## Omitted (budget)"));
  assert.ok(/\.\.\.and \d+ more/.test(out), "expected a capped omitted footer with an '...and N more' tail");
});
