/**
 * Minimal, dependency-free YAML for the narrow subset Agnosgram controls:
 * nested maps (2-space indent), scalars (string / number / boolean / null),
 * and block sequences of scalars (`- item`). It is deliberately NOT a general
 * YAML implementation — it covers `config.yml` and record frontmatter, both of
 * which we author from templates. Anything outside the subset throws.
 */

export type YamlValue =
  | string
  | number
  | boolean
  | null
  | YamlValue[]
  | { [key: string]: YamlValue };

interface Line {
  indent: number;
  content: string;
  lineNo: number;
}

/** Remove a trailing ` # comment`, respecting single/double quotes. */
function stripComment(raw: string): string {
  let inSingle = false;
  let inDouble = false;
  for (let i = 0; i < raw.length; i++) {
    const ch = raw[i];
    if (ch === "'" && !inDouble) inSingle = !inSingle;
    else if (ch === '"' && !inSingle) inDouble = !inDouble;
    else if (ch === "#" && !inSingle && !inDouble) {
      // A comment marker only counts at line start or after whitespace.
      if (i === 0 || raw[i - 1] === " " || raw[i - 1] === "\t") {
        return raw.slice(0, i);
      }
    }
  }
  return raw;
}

function tokenize(text: string): Line[] {
  const out: Line[] = [];
  const rows = text.split(/\r?\n/);
  for (let i = 0; i < rows.length; i++) {
    const raw = stripComment(rows[i] ?? "");
    if (raw.trim() === "") continue;
    if (raw.trim() === "---") continue; // tolerate document markers
    const trimmed = raw.trimStart();
    out.push({ indent: raw.length - trimmed.length, content: trimmed.trimEnd(), lineNo: i + 1 });
  }
  return out;
}

function parseScalar(token: string): YamlValue {
  const t = token.trim();
  if (t === "" || t === "~" || t === "null") return null;
  if (t === "true") return true;
  if (t === "false") return false;
  if (/^-?\d+$/.test(t)) return Number.parseInt(t, 10);
  if (/^-?\d+\.\d+$/.test(t)) return Number.parseFloat(t);
  if (t === "{}") return {};
  if (t.startsWith("[") && t.endsWith("]")) {
    // Flow sequence of scalars, e.g. `scope: [core, tooling]`. Used by record
    // frontmatter; kept deliberately narrow (scalars only, no nested flow).
    const inner = t.slice(1, -1).trim();
    if (inner === "") return [];
    return splitFlow(inner).map((s) => parseScalar(s));
  }
  if ((t.startsWith('"') && t.endsWith('"')) || (t.startsWith("'") && t.endsWith("'"))) {
    return t.slice(1, -1);
  }
  return t;
}

/** Split a flow-sequence body on top-level commas, respecting quotes. */
function splitFlow(inner: string): string[] {
  const parts: string[] = [];
  let inSingle = false;
  let inDouble = false;
  let start = 0;
  for (let i = 0; i < inner.length; i++) {
    const ch = inner[i];
    if (ch === "'" && !inDouble) inSingle = !inSingle;
    else if (ch === '"' && !inSingle) inDouble = !inDouble;
    else if (ch === "," && !inSingle && !inDouble) {
      parts.push(inner.slice(start, i).trim());
      start = i + 1;
    }
  }
  parts.push(inner.slice(start).trim());
  return parts.filter((p) => p !== "");
}

/** Parse a block of lines whose indentation is >= `indent`, starting at `i`. */
function parseBlock(lines: Line[], i: number, indent: number): [YamlValue, number] {
  const first = lines[i];
  if (!first) return [null, i];

  if (first.content.startsWith("- ") || first.content === "-") {
    const arr: YamlValue[] = [];
    while (i < lines.length && lines[i]!.indent === indent && lines[i]!.content.startsWith("-")) {
      const item = lines[i]!.content.slice(1).trim();
      arr.push(parseScalar(item));
      i++;
    }
    return [arr, i];
  }

  const map: { [key: string]: YamlValue } = {};
  while (i < lines.length && lines[i]!.indent === indent) {
    const line = lines[i]!;
    const colon = findColon(line.content);
    if (colon === -1) {
      throw new Error(`Invalid YAML at line ${line.lineNo}: expected "key: value"`);
    }
    const key = line.content.slice(0, colon).trim();
    const rest = line.content.slice(colon + 1).trim();
    i++;
    if (rest !== "") {
      map[key] = parseScalar(rest);
    } else if (i < lines.length && lines[i]!.indent > indent) {
      const [value, next] = parseBlock(lines, i, lines[i]!.indent);
      map[key] = value;
      i = next;
    } else {
      map[key] = null;
    }
  }
  return [map, i];
}

function findColon(content: string): number {
  let inSingle = false;
  let inDouble = false;
  for (let i = 0; i < content.length; i++) {
    const ch = content[i];
    if (ch === "'" && !inDouble) inSingle = !inSingle;
    else if (ch === '"' && !inSingle) inDouble = !inDouble;
    else if (ch === ":" && !inSingle && !inDouble) {
      if (i + 1 >= content.length || content[i + 1] === " ") return i;
    }
  }
  return -1;
}

export function parseYaml(text: string): YamlValue {
  const lines = tokenize(text);
  if (lines.length === 0) return {};
  const [value] = parseBlock(lines, 0, lines[0]!.indent);
  return value;
}

function needsQuote(s: string): boolean {
  if (s === "") return true;
  if (/^[\s]|[\s]$/.test(s)) return true;
  if (/[:#]/.test(s)) return true;
  if (["true", "false", "null", "~"].includes(s)) return true;
  if (/^-?\d/.test(s)) return true; // avoid a string that looks numeric
  return false;
}

function formatScalar(value: string | number | boolean | null): string {
  if (value === null) return "";
  if (typeof value === "boolean" || typeof value === "number") return String(value);
  return needsQuote(value) ? JSON.stringify(value) : value;
}

export function stringifyYaml(value: YamlValue, indent = 0): string {
  const pad = "  ".repeat(indent);

  if (Array.isArray(value)) {
    if (value.length === 0) return `${pad}[]\n`;
    return value.map((item) => `${pad}- ${formatScalar(item as never)}`).join("\n") + "\n";
  }

  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value);
    if (entries.length === 0) return `${pad}{}\n`;
    let out = "";
    for (const [key, v] of entries) {
      if (Array.isArray(v)) {
        if (v.length === 0) {
          out += `${pad}${key}: []\n`;
        } else {
          out += `${pad}${key}:\n${stringifyYaml(v, indent)}`;
        }
      } else if (v !== null && typeof v === "object") {
        out += `${pad}${key}:\n${stringifyYaml(v, indent + 1)}`;
      } else {
        out += `${pad}${key}: ${formatScalar(v)}\n`;
      }
    }
    return out;
  }

  return `${pad}${formatScalar(value)}\n`;
}
