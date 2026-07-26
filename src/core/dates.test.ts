import assert from "node:assert/strict";
import { test } from "node:test";
import { daysBetween, isValidIsoDate, todayIso } from "./dates.js";

test("isValidIsoDate accepts real dates and rejects malformed or impossible ones", () => {
  assert.equal(isValidIsoDate("2026-07-21"), true);
  assert.equal(isValidIsoDate("2026-02-29"), false); // not a leap year
  assert.equal(isValidIsoDate("2026-13-01"), false);
  assert.equal(isValidIsoDate("2026-7-1"), false); // needs zero-padding
  assert.equal(isValidIsoDate("not-a-date"), false);
});

test("daysBetween is signed and timezone-independent", () => {
  assert.equal(daysBetween("2026-07-01", "2026-07-21"), 20);
  assert.equal(daysBetween("2026-07-21", "2026-07-01"), -20);
  assert.equal(daysBetween("2026-07-21", "2026-07-21"), 0);
});

test("todayIso yields a valid ISO date", () => {
  assert.equal(isValidIsoDate(todayIso()), true);
});
