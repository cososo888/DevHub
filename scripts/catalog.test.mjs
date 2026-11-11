import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const source = readFileSync(fileURLToPath(new URL("../src/catalog.ts", import.meta.url)), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 } }).outputText;
const { SOFTWARE, CATEGORIES, STAGE_LABEL } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);

test("curated catalog has exactly 100 unique Homebrew packages in declared categories", () => {
  assert.equal(SOFTWARE.length, 100);
  assert.equal(new Set(SOFTWARE.map(item => item.id)).size, 100);
  assert.equal(new Set(SOFTWARE.map(item => `${item.kind}:${item.package}`)).size, 100);
  for (const item of SOFTWARE) {
    assert.ok(CATEGORIES.includes(item.category), item.id);
    assert.ok(item.package.length > 0, item.id);
    assert.ok(item.kind === "formula" || item.kind === "cask", item.id);
    assert.ok(item.name.length > 0 && item.description.length > 0, item.id);
  }
});

test("all eight categories have curated software and stage labels remain valid", () => {
  for (const category of CATEGORIES) {
    assert.ok(SOFTWARE.some(item => item.category === category), category);
  }
  assert.equal(STAGE_LABEL.first, "首批支持");
  assert.equal(STAGE_LABEL.planned, "Homebrew 安装");
});
