/**
 * `.agnosgram/meta/` - tool-friction feedback, strictly separate from
 * host-project memory (lessons/, decisions/, context/). This is an additive
 * extension to the frozen format (DEC-0002): a store without `meta/` is
 * fully valid, and nothing here changes the meaning of anything else. See
 * `.agnosgram/decisions/0003-human-in-the-loop-reflexive-loop.md`.
 *
 * Friction records reuse the record shape (id, type, scope, confidence,
 * created, last_verified, source, supersedes?) and `frontmatter.ts`'s
 * field-by-field validation, just against a different type enum - so the
 * frozen lessons/decisions validator (`KNOWN_TYPES`) never has to know
 * `meta/` exists.
 */
import { extractRecords, validateRecord } from "./frontmatter.js";
import type { RawRecord } from "./frontmatter.js";
import type { StoreRecord } from "./records.js";
import { readStore } from "./store.js";

/** The only friction record type today; kept as a list so `validateRecord`
 * can share its `allowedTypes` shape with the frozen lessons/decisions enum. */
export const KNOWN_META_TYPES = ["friction"] as const;
export const FRICTION_TYPE = "friction";

/** Store-relative path of the single friction-entries file. */
export const FRICTION_FILE = "meta/friction.md";

/** Validate one friction record's frontmatter (id/type/scope/confidence/dates/source). */
export function validateFrictionRecord(raw: RawRecord) {
  return validateRecord(raw, KNOWN_META_TYPES);
}

/** Every schema-valid friction record in `meta/friction.md`, if it exists. */
export function loadFrictionRecords(root: string): StoreRecord[] {
  const out: StoreRecord[] = [];
  for (const file of readStore(root)) {
    if (file.storeRel !== FRICTION_FILE) continue;
    for (const raw of extractRecords(file.text)) {
      const { frontmatter } = validateFrictionRecord(raw);
      if (!frontmatter) continue;
      out.push({
        frontmatter,
        body: raw.body,
        file: file.rel,
        storeRel: file.storeRel,
        line: raw.line,
      });
    }
  }
  return out;
}
