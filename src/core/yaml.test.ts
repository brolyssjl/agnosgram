import assert from "node:assert/strict";
import { test } from "node:test";
import { parseYaml, stringifyYaml } from "./yaml.js";

test("parses scalars with correct types", () => {
  const v = parseYaml("version: 1\nname: hello\nflag: true\noff: false\nempty:\n") as Record<
    string,
    unknown
  >;
  assert.equal(v.version, 1);
  assert.equal(v.name, "hello");
  assert.equal(v.flag, true);
  assert.equal(v.off, false);
  assert.equal(v.empty, null);
});

test("parses nested maps", () => {
  const v = parseYaml("journal:\n  committed: true\nbudgets:\n  a.md: 100\n") as {
    journal: Record<string, unknown>;
    budgets: Record<string, unknown>;
  };
  assert.equal(v.journal.committed, true);
  assert.equal(v.budgets["a.md"], 100);
});

test("parses block sequences of scalars", () => {
  const v = parseYaml("scope:\n  - backend\n  - auth\n") as Record<string, unknown>;
  assert.deepEqual(v.scope, ["backend", "auth"]);
});

test("ignores comments and blank lines", () => {
  const v = parseYaml("# a comment\n\nversion: 1  # inline\n") as Record<string, unknown>;
  assert.equal(v.version, 1);
});

test("round-trips a config-shaped object", () => {
  const obj = {
    version: 1,
    journal: { committed: true },
    budgets: { "state/status.md": 400 },
    adapters: { claude: "on", cursor: "off" },
    sdd: { openspec: "auto" },
  };
  const round = parseYaml(stringifyYaml(obj as never));
  assert.deepEqual(round, obj);
});

test("parses inline flow sequences of scalars", () => {
  const v = parseYaml("scope: [core, tooling]\nsupersedes: [DEC-0001, DEC-0002]\n") as Record<
    string,
    unknown
  >;
  assert.deepEqual(v.scope, ["core", "tooling"]);
  assert.deepEqual(v.supersedes, ["DEC-0001", "DEC-0002"]);
});

test("parses an empty flow sequence and empty flow map", () => {
  const v = parseYaml("a: []\nb: {}\n") as Record<string, unknown>;
  assert.deepEqual(v.a, []);
  assert.deepEqual(v.b, {});
});

test("flow sequence respects quoted commas", () => {
  const v = parseYaml('tags: ["a, b", c]\n') as Record<string, unknown>;
  assert.deepEqual(v.tags, ["a, b", "c"]);
});

test("quotes values that would otherwise reparse wrong", () => {
  const out = stringifyYaml({ a: "true", b: "123", c: "x: y" } as never);
  const round = parseYaml(out) as Record<string, unknown>;
  assert.equal(round.a, "true");
  assert.equal(round.b, "123");
  assert.equal(round.c, "x: y");
});
