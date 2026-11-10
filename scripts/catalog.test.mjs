import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const source = readFileSync(fileURLToPath(new URL("../src/catalog.ts", import.meta.url)), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { SOFTWARE, CATEGORIES, STAGE_LABEL } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
test("software list is unique and every category is declared", () => {
  assert.equal(SOFTWARE.length, 14);
  assert.equal(new Set(SOFTWARE.map(item => item.id)).size, 14);
  for (const item of SOFTWARE) assert.ok(CATEGORIES.includes(item.category), item.id);
});
test("first phase only claims Git and Node as first-batch tools", () => {
  assert.deepEqual(SOFTWARE.filter(item => item.stage === "first").map(item => item.id).sort(), ["git", "node"]);
  assert.equal(STAGE_LABEL.first, "首批支持");
});
