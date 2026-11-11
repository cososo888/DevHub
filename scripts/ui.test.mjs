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
const { filterCurated } = await import("../src/lib/catalogFilter.ts");

afterEach(() => {
  cleanup();
  globalThis.__REDUCE__ = false;
  document.body.innerHTML = "";
});

test("catalog exposes 100 unique, real Homebrew packages across eight categories", () => {
  assert.equal(SOFTWARE.length, 100);
  assert.equal(new Set(SOFTWARE.map(item => item.id)).size, SOFTWARE.length);
  assert.equal(new Set(SOFTWARE.map(item => `${item.kind}:${item.package}`)).size, SOFTWARE.length);
  assert.deepEqual(CATEGORIES, ["语言与运行时", "开发工具", "终端工具", "数据库与服务", "网络与云", "容器与云", "效率工具", "设计媒体"]);
  const counts = Object.fromEntries(CATEGORIES.map(category => [category, SOFTWARE.filter(item => item.category === category).length]));
  assert.equal(Object.values(counts).reduce((a, b) => a + b, 0), 100);
  assert.equal(counts["语言与运行时"], 20);
  assert.equal(counts["开发工具"], 20);
  for (const item of SOFTWARE) {
    assert.ok(item.package && item.kind === "formula" || item.kind === "cask", item.id);
    assert.ok(item.name && item.description);
  }
});

test("search is global and ignores the selected category", () => {
  const underDev = filterCurated(SOFTWARE, "node", "开发工具");
  assert.ok(underDev.some(item => item.id === "node"));
  const noQuery = filterCurated(SOFTWARE, "", "开发工具");
  assert.ok(noQuery.length > 0);
  assert.ok(noQuery.every(item => item.category === "开发工具"));
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

test("an installing orb exposes an indeterminate busy state", () => {
  render(React.createElement(OrbGrid, { items: SOFTWARE.slice(0, 2), installed: new Set(), installing: new Set(["node"]), onOpen: () => {} }));
  const node = screen.getByRole("button", { name: "Node.js" });
  assert.match(node.className, /is-installing/);
  assert.equal(node.getAttribute("aria-busy"), "true");
});

test("the source orb is hidden while its card is open", () => {
  render(React.createElement(OrbGrid, { items: SOFTWARE.slice(0, 2), installed: new Set(), sourceId: "node", onOpen: () => {} }));
  const node = document.querySelector('button[aria-label="Node.js"]');
  assert.match(node.className, /is-source/);
  assert.equal(node.getAttribute("aria-hidden"), "true");
  assert.equal(node.tabIndex, -1);
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
  await user.click(screen.getByRole("button", { name: "开发工具" }));
  assert.equal(screen.getByRole("button", { name: "开发工具" }).getAttribute("aria-pressed"), "true");
  await user.click(screen.getByRole("button", { name: "深色" }));
  assert.equal(screen.getByRole("button", { name: "深色" }).getAttribute("aria-pressed"), "true");
  assert.equal(screen.getByTestId("state").textContent, "node|开发工具|dark");
  await user.click(screen.getByRole("button", { name: "收起搜索" }));
  assert.equal(screen.queryByRole("textbox"), null);
  assert.equal(screen.getByTestId("state").textContent, "|开发工具|dark");
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

const instance = (id = "git", overrides = {}) => ({
  softwareId: id,
  version: id === "go" ? "1.27.3" : "2.56.0",
  source: "homebrewFormula",
  sources: ["homebrewFormula"],
  path: id === "go" ? "/opt/homebrew/Cellar/go/1.27.3/bin/go" : "/opt/homebrew/Cellar/git/2.56.0/bin/git",
  kind: "cli",
  executable: true,
  managedBy: "homebrew",
  health: "ok",
  detail: null,
  lastDetectedAt: 1,
  ...overrides
});

test("detail shows status and one install action without a version picker", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [], homebrewReady: true }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  await waitFor(() => assert.equal(dialog.style.width, "380px"));
  assert.match(dialog.textContent, /Go/);
  assert.match(dialog.textContent, /未安装/);
  assert.ok(screen.getByRole("button", { name: "安装 Go" }));
  assert.equal(screen.queryByRole("button", { name: /卸载/ }), null);
  assert.equal(dialog.textContent.includes("版本列表"), false);
});

test("install requires confirmation and shows the fixed brew command", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  let installed = 0;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [], homebrewReady: true, onInstall: () => { installed += 1; } }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  await userEvent.setup().click(screen.getByRole("button", { name: "安装 Go" }));
  assert.match(dialog.textContent, /安装 Go？/);
  assert.match(dialog.textContent, /brew install go/);
  assert.equal(installed, 0);
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.equal(installed, 1);
});

test("Homebrew-managed package can be uninstalled only after confirmation", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  let uninstalled = 0;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, brewInstalled: true, brewVersion: "1.27.3", brewPath: "/opt/homebrew/opt/go", homebrewReady: true, onUninstall: () => { uninstalled += 1; } }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(dialog.textContent, /已安装/);
  assert.match(dialog.textContent, /1\.27\.3/);
  assert.match(dialog.textContent, /\/opt\/homebrew\/opt\/go/);
  await userEvent.setup().click(screen.getByRole("button", { name: "卸载 Go" }));
  assert.match(dialog.textContent, /卸载 Go？/);
  assert.match(dialog.textContent, /brew uninstall go/);
  assert.equal(uninstalled, 0);
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.equal(uninstalled, 1);
});

test("system-installed software is shown but never offered uninstall", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  const system = instance("git", { source: "path", sources: ["path"], path: "/usr/bin/git", managedBy: "system" });
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [system], homebrewReady: true }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  assert.match(dialog.textContent, /已安装/);
  assert.match(dialog.textContent, /\/usr\/bin\/git/);
  assert.match(dialog.textContent, /仅查看/);
  assert.equal(screen.queryByRole("button", { name: /卸载/ }), null);
});

test("non-Homebrew install allows a duplicate Homebrew install with a warning", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  const system = instance("git", { source: "path", sources: ["path"], path: "/usr/bin/git", managedBy: "system" });
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [system], homebrewReady: true }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  await userEvent.setup().click(screen.getByRole("button", { name: "安装 Git" }));
  assert.match(dialog.textContent, /已有其他来源的安装/);
  assert.match(dialog.textContent, /brew install git/);
});

test("install without Homebrew is disabled and explains the prerequisite", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [], homebrewReady: false }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.equal(screen.getByRole("button", { name: "安装 Go" }).disabled, true);
  assert.match(dialog.textContent, /需要 Homebrew/);
});

test("indeterminate progress stays a single stable track across remounts", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  const task = { jobId: 2, softwareId: "go", version: "go", action: "install", stage: "installing", message: "正在安装…", percent: null };
  for (let index = 0; index < 5; index += 1) {
    const view = render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [], homebrewReady: true, task }));
    const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
    const track = dialog.querySelector(".indeterminate");
    assert.ok(track);
    assert.equal(track.children.length, 0);
    assert.equal(dialog.textContent.includes("%"), false);
    view.unmount();
  }
});

test("failed install exposes a short reason and technical log", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  const result = { jobId: 1, softwareId: "go", version: "go", action: "install", success: false, message: "安装失败：网络不可用", output: "brew stderr detail", verification: null };
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, instances: [], homebrewReady: true, result }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(dialog.textContent, /安装失败：网络不可用/);
  assert.ok(dialog.querySelector(".task-log pre"));
  assert.match(dialog.querySelector(".task-log pre").textContent, /brew stderr detail/);
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
  await waitFor(() => assert.ok(document.querySelector(".orb-grid")));
  assert.equal(screen.queryByText("选择你的开发工具"), null);
  assert.equal(screen.getAllByRole("button").filter(button => button.getAttribute("aria-label") && SOFTWARE.some(item => item.name === button.getAttribute("aria-label"))).length, 100);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
  await user.type(await screen.findByRole("textbox", { name: "搜索软件" }), "git");
  await waitFor(() => assert.equal(screen.queryByRole("button", { name: "Python" }), null));
  assert.ok(screen.getByRole("button", { name: "Git" }));
  assert.equal(screen.queryByText(/已安装/), null);
});
