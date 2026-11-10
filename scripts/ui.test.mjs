import test, { afterEach, before } from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { JSDOM } from "jsdom";

const dom = new JSDOM("<!doctype html><html><body></body></html>", { url: "http://localhost" });
for (const key of ["window", "document", "HTMLElement", "Node", "MutationObserver", "Event", "KeyboardEvent"]) {
  globalThis[key] = key === "window" ? dom.window : key === "document" ? dom.window.document : dom.window[key];
}
Object.defineProperty(globalThis, "navigator", { value: dom.window.navigator, configurable: true });
globalThis.IS_REACT_ACT_ENVIRONMENT = true;
globalThis.__REDUCE__ = false;
globalThis.requestAnimationFrame = callback => setTimeout(() => callback(Date.now()), 0);
globalThis.cancelAnimationFrame = id => clearTimeout(id);
Object.defineProperty(window, "matchMedia", {
  configurable: true,
  value: query => ({
    matches: query.includes("reduce") ? globalThis.__REDUCE__ === true : false,
    media: query,
    addEventListener() {},
    removeEventListener() {}
  })
});

registerHooks({
  resolve(specifier, context, next) {
    if (specifier.startsWith(".") && context.parentURL?.includes("/src/")) {
      const url = new URL(specifier, context.parentURL);
      for (const suffix of [".ts", ".tsx"]) {
        if (existsSync(fileURLToPath(url) + suffix)) return { url: url.href + suffix, shortCircuit: true };
      }
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.includes("/src/") && /\.tsx?$/.test(url)) {
      const source = ts.transpileModule(readFileSync(fileURLToPath(url), "utf8"), {
        compilerOptions: { jsx: ts.JsxEmit.ReactJSX, module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 }
      }).outputText;
      return { format: "module", source, shortCircuit: true };
    }
    return next(url, context);
  }
});

const React = (await import("react")).default;
const { render, screen, cleanup, waitFor, fireEvent } = await import("@testing-library/react");
const userEvent = (await import("@testing-library/user-event")).default;
const { OrbGrid } = await import("../src/components/OrbGrid.tsx");
const { Toolbar } = await import("../src/components/Toolbar.tsx");
const { DetailOverlay, anchoredCard } = await import("../src/components/DetailOverlay.tsx");
const { SOFTWARE, CATEGORIES } = await import("../src/catalog.ts");

afterEach(() => {
  cleanup();
  globalThis.__REDUCE__ = false;
  document.body.innerHTML = "";
});

test("catalog exposes a complete, unique, categorized first-phase list", () => {
  assert.equal(SOFTWARE.length, 14);
  assert.equal(new Set(SOFTWARE.map(item => item.id)).size, SOFTWARE.length);
  assert.deepEqual(CATEGORIES, ["语言与运行时", "开发基础", "数据库与服务", "开发工具"]);
  for (const id of ["git", "node", "jdk", "maven", "gradle", "python", "go"]) {
    assert.ok(SOFTWARE.some(item => item.id === id), id);
  }
  assert.equal(SOFTWARE.find(item => item.id === "git").stage, "first");
  assert.equal(SOFTWARE.find(item => item.id === "node").stage, "first");
});

test("circle grid renders every software once and reports the orb origin", async () => {
  let opened = null;
  render(React.createElement(OrbGrid, { items: SOFTWARE, installed: new Set(["git"]), onOpen: (item, origin) => { opened = { item, origin }; } }));
  const buttons = screen.getAllByRole("button");
  assert.equal(buttons.length, SOFTWARE.length);
  const git = screen.getByRole("button", { name: "Git" });
  assert.match(git.className, /is-installed/);
  assert.ok(!screen.getByRole("button", { name: "Python" }).className.includes("is-installed"));
  await userEvent.setup().click(git);
  assert.equal(opened.item.id, "git");
  assert.ok("width" in opened.origin && "left" in opened.origin);
});

test("toolbar expands search, filters categories, clears on close and exposes themes", async () => {
  function Harness() {
    const [query, setQuery] = React.useState("");
    const [category, setCategory] = React.useState("全部");
    const [searchOpen, setSearchOpen] = React.useState(false);
    const [theme, setTheme] = React.useState("system");
    return React.createElement(React.Fragment, null,
      React.createElement(Toolbar, { query, onQuery: setQuery, category, onCategory: setCategory, searchOpen, onSearchToggle: () => setSearchOpen(value => !value), theme, onTheme: setTheme }),
      React.createElement("output", { "data-testid": "state" }, `${query}|${category}|${theme}`)
    );
  }
  render(React.createElement(Harness));
  const user = userEvent.setup();
  assert.equal(screen.queryByRole("textbox"), null);
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
  const input = await screen.findByRole("textbox", { name: "搜索软件" });
  await user.type(input, "node");
  assert.equal(screen.getByTestId("state").textContent, "node|全部|system");
  await user.click(screen.getByRole("button", { name: "开发基础" }));
  assert.equal(screen.getByRole("button", { name: "开发基础" }).getAttribute("aria-pressed"), "true");
  await user.click(screen.getByRole("button", { name: "深色" }));
  assert.equal(screen.getByRole("button", { name: "深色" }).getAttribute("aria-pressed"), "true");
  assert.equal(screen.getByTestId("state").textContent, "node|开发基础|dark");
  await user.click(screen.getByRole("button", { name: "收起搜索" }));
  assert.equal(screen.queryByRole("textbox"), null);
  assert.equal(screen.getByTestId("state").textContent, "|开发基础|dark");
});

test("detail card morphs from the orb, explains the disabled phase-one state and closes with Escape", async () => {
  let closed = 0;
  const item = SOFTWARE.find(entry => entry.id === "git");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => { closed += 1; } }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  await waitFor(() => assert.equal(dialog.style.width, "380px"));
  assert.equal(dialog.style.borderRadius, "24px");
  assert.equal(dialog.style.left, "16px");
  assert.equal(dialog.style.top, "16px");
  assert.equal(document.querySelector(".detail-backdrop"), null);
  assert.ok(document.querySelector(".detail-catcher"));
  assert.match(dialog.textContent, /尚未检测/);
  assert.match(dialog.textContent, /不会安装、卸载或修改系统/);
  assert.equal(dialog.textContent.includes("安装路径"), false);
  assert.deepEqual([...dialog.querySelectorAll(".detail-block h3")].map(node => node.textContent), ["已安装版本", "可安装版本", "安装来源"]);
  assert.equal(screen.getByRole("button", { name: /安装/ }).disabled, true);
  assert.equal(screen.getByRole("button", { name: "卸载" }).disabled, true);
  assert.equal(screen.getByRole("combobox", { name: "选择版本" }).disabled, true);
  await userEvent.setup().keyboard("{Escape}");
  await waitFor(() => assert.equal(dialog.style.width, "90px"));
  await waitFor(() => assert.equal(closed, 1), { timeout: 2000 });
});

test("anchored geometry keeps the orb as a corner and stays inside the viewport", () => {
  // A card taller than the space below is capped to that space; the orb stays at the top-left corner.
  const upper = anchoredCard({ left: 52, top: 236, width: 123, height: 123 }, 380, 569, 1200, 713);
  assert.equal(upper.left, 52);
  assert.equal(upper.top, 128);
  assert.equal(upper.width, 380);
  assert.equal(upper.height, 569);
  assert.ok(upper.top + upper.height <= 713 - 16);

  // A card near the bottom-right grows up and left; its bottom/right corner is the orb.
  const lower = anchoredCard({ left: 900, top: 650, width: 90, height: 90 }, 380, 360, 1024, 768);
  assert.equal(lower.left, 610);
  assert.equal(lower.top, 380);
  assert.equal(lower.left + lower.width, 990);
  assert.equal(lower.top + lower.height, 740);

  // A card near the top-right grows down, anchored at its top-right corner.
  const right = anchoredCard({ left: 885, top: 236, width: 123, height: 123 }, 380, 550, 1200, 713);
  assert.equal(right.left, 628);
  assert.equal(right.top, 147);
  assert.equal(right.left + right.width, 1008);
  assert.ok(right.top + right.height <= 713 - 16);
});

test("detail keeps keyboard scrolling while the scrollbar is visually hidden", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {} }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  const inner = dialog.querySelector(".detail-inner");
  assert.equal(inner.tabIndex, 0);
  Object.defineProperty(inner, "clientHeight", { value: 300, configurable: true });
  Object.defineProperty(inner, "scrollHeight", { value: 900, configurable: true });
  let moved = null;
  let ended = null;
  inner.scrollBy = options => { moved = options.top; };
  inner.scrollTo = options => { ended = options.top; };
  fireEvent.keyDown(window, { key: "PageDown" });
  assert.ok(moved > 0);
  fireEvent.keyDown(window, { key: "End" });
  assert.equal(ended, 900);
});

test("detail renders real detection instances, paths and health", async () => {
  const item = SOFTWARE.find(entry => entry.id === "node");
  const instances = [
    { softwareId: "node", version: "26.13.0", source: "devhubV1", sources: ["devhubV1"], path: "/Users/test/Library/Application Support/com.yujian.devhub.validation/storage-v1/versions/node/26.13.0-arm64/bin/node", kind: "cli", executable: true, managedBy: "devhub", health: "ok", detail: null, lastDetectedAt: 1 },
    { softwareId: "node", version: "26.12.1", source: "path", sources: ["path"], path: "/usr/local/bin/node", kind: "cli", executable: true, managedBy: "system", health: "unverified", detail: "可执行文件不在受信任的检测目录", lastDetectedAt: 1 }
  ];
  render(React.createElement(DetailOverlay, {
    item,
    origin: { left: 10, top: 10, width: 90, height: 90 },
    onClose: () => {},
    instances,
    detected: true,
    refreshing: false,
    stale: false,
    homebrew: { available: true, prefix: "/opt/homebrew", version: "7.0.12", error: null },
    errors: [],
    refreshedAt: 1794308394563
  }));
  const dialog = await screen.findByRole("dialog", { name: "Node.js 详情" });
  assert.match(dialog.textContent, /已安装 · 2 个版本/);
  assert.match(dialog.textContent, /26\.13\.0/);
  assert.match(dialog.textContent, /26\.12\.1/);
  assert.match(dialog.textContent, /旧版 DevHub/);
  assert.match(dialog.textContent, /未验证/);
  assert.match(dialog.textContent, /usr\/local\/bin\/node/);
  assert.match(dialog.textContent, /7\.0\.12/);
  assert.equal(screen.getByRole("button", { name: /安装/ }).disabled, true);
  assert.equal(screen.getByRole("button", { name: "卸载" }).disabled, true);
});

test("detail shows uninstalled only after a completed detection", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, detected: true, refreshing: false, stale: false }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(dialog.textContent, /未安装/);
  assert.match(dialog.textContent, /未检测到本机安装/);
  assert.equal(dialog.textContent.includes("安装路径"), false);
});

test("reduced motion opens the card at its final geometry and closes without waiting", async () => {
  globalThis.__REDUCE__ = true;
  let closed = 0;
  const item = SOFTWARE.find(entry => entry.id === "node");
  render(React.createElement(DetailOverlay, { item, origin: { left: 0, top: 0, width: 80, height: 80 }, onClose: () => { closed += 1; } }));
  const dialog = await screen.findByRole("dialog", { name: "Node.js 详情" });
  assert.equal(dialog.style.width, "380px");
  await userEvent.setup().keyboard("{Escape}");
  await waitFor(() => assert.equal(closed, 1));
});

test("app starts with the full grid, filters by search, and never fabricates installed state", async () => {
  const { default: App } = await import("../src/App.tsx");
  render(React.createElement(App));
  await screen.findByRole("heading", { name: "选择你的开发工具" });
  assert.equal(screen.getAllByRole("button").filter(button => button.getAttribute("aria-label") && SOFTWARE.some(item => item.name === button.getAttribute("aria-label"))).length, 14);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
  await user.type(await screen.findByRole("textbox", { name: "搜索软件" }), "git");
  await waitFor(() => assert.equal(screen.queryByRole("button", { name: "Python" }), null));
  assert.ok(screen.getByRole("button", { name: "Git" }));
  assert.equal(screen.queryByText(/已安装/), null);
});
