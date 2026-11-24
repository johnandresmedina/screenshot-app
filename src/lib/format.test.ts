import { test } from "node:test";
import assert from "node:assert/strict";
import { dayLabel, groupByDay, sizeLabel } from "./format.ts";

const now = new Date(2026, 10, 24, 15, 0).getTime();

test("dayLabel names today and yesterday", () => {
  assert.equal(dayLabel(new Date(2026, 10, 24, 0, 5).getTime(), now), "Today");
  assert.equal(dayLabel(new Date(2026, 10, 23, 23, 59).getTime(), now), "Yesterday");
  assert.notEqual(dayLabel(new Date(2026, 10, 20).getTime(), now), "Yesterday");
});

test("sizeLabel picks a sensible unit", () => {
  assert.equal(sizeLabel(512), "512 B");
  assert.equal(sizeLabel(2048), "2 KB");
  assert.equal(sizeLabel(3.5 * 1024 * 1024), "3.5 MB");
});

test("groupByDay keeps order and splits on day changes", () => {
  const items = [
    { id: "a", createdAt: new Date(2026, 10, 24, 14).getTime() },
    { id: "b", createdAt: new Date(2026, 10, 24, 9).getTime() },
    { id: "c", createdAt: new Date(2026, 10, 23, 18).getTime() },
  ];
  const groups = groupByDay(items, now);
  assert.deepEqual(groups.map((g) => [g.label, g.items.map((i) => i.id)]), [
    ["Today", ["a", "b"]],
    ["Yesterday", ["c"]],
  ]);
});
