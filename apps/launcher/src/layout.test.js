// npm test
import { test } from "node:test";
import assert from "node:assert/strict";
import { place, ranks } from "./layout.js";

test("callers sit above what they call", () => {
  const links = [
    { from: "src", to: "crates/search" },
    { from: "crates/search", to: "crates/core" },
    { from: "src", to: "crates/core" },
  ];
  assert.deepEqual(ranks(["src", "crates/search", "crates/core", "tests"], links), {
    src: 0, "crates/search": 1, "crates/core": 2, tests: 0,
  });
});

test("a cycle does not loop forever", () => {
  const r = ranks(["a", "b"], [{ from: "a", to: "b" }, { from: "b", to: "a" }]);
  assert.ok(r.a < 2 && r.b < 2);
});

test("boxes in a row do not overlap", () => {
  const { pos } = place([{ name: "a" }, { name: "b" }, { name: "c" }], [], 900);
  assert.ok(pos.a.x + pos.a.w <= pos.b.x && pos.b.x + pos.b.w <= pos.c.x);
});

test("a wide row widens the drawing instead of spilling out", () => {
  const blocks = Array.from({ length: 8 }, (_, i) => ({ name: "b" + i }));
  const { pos, width } = place(blocks, [], 860);
  assert.ok(Object.values(pos).every((p) => p.x >= 0 && p.x + p.w <= width));
});
