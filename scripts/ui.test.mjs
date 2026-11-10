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
  await user.click(screen.getByRole("button", { name: "开发基础" }));
  assert.equal(screen.getByRole("button", { name: "开发基础" }).getAttribute("aria-pressed"), "true");
  await user.click(screen.getByRole("button", { name: "深色" }));
  assert.equal(screen.getByRole("button", { name: "深色" }).getAttribute("aria-pressed"), "true");
  assert.equal(screen.getByTestId("state").textContent, "node|开发基础|dark");
  await user.click(screen.getByRole("button", { name: "收起搜索" }));
  assert.equal(screen.queryByRole("textbox"), null);
  assert.equal(screen.getByTestId("state").textContent, "|开发基础|dark");
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

const versionRow = (overrides = {}) => ({
  version: "1.0.0", installed: false, installable: false, canUninstall: false,
  methods: [], formula: null, instances: [], reason: null, ...overrides
});

test("detail is reduced to icon, name, description and the version list", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions: [] }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  await waitFor(() => assert.equal(dialog.style.width, "380px"));
  assert.equal(dialog.style.borderRadius, "24px");
  assert.equal(document.querySelector(".detail-backdrop"), null);
  assert.match(dialog.textContent, /Git/);
  assert.match(dialog.textContent, /版本控制/);
  assert.match(dialog.textContent, /暂无可安装版本/);
  assert.equal(dialog.textContent.includes("Homebrew"), false);
  assert.equal(dialog.textContent.includes("/usr/bin"), false);
  for (const removed of ["已安装版本", "可安装版本", "安装来源", "Homebrew 安装", "后续支持", "安装路径"]) {
    assert.equal(screen.queryByText(removed, { exact: true }), null, removed);
  }
});

test("version rows install only after explicit confirmation and uninstall only when owned", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  const versions = [
    versionRow({ version: "2.56.0", installable: true, formula: "git" }),
    versionRow({ version: "2.50.1", installed: true, canUninstall: true, instances: [
      { source: "path", path: "/usr/bin/git", health: "ok", managedBy: "system" },
      { source: "devhubV1", path: "/tmp/git", health: "ok", managedBy: "devhub" }
    ] })
  ];
  let installed = "";
  let uninstalled = "";
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, onInstall: value => { installed = value; }, onUninstall: value => { uninstalled = value; } }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  const versionsText = dialog.querySelector(".version-list").textContent;
  assert.ok(versionsText.indexOf("2.56.0") < versionsText.indexOf("2.50.1"));
  assert.equal(dialog.querySelectorAll(".version-row").length, 2);
  assert.equal(dialog.textContent.includes("/usr/bin/git"), false);

  await userEvent.setup().click(screen.getByRole("button", { name: "安装 2.56.0" }));
  assert.match(dialog.textContent, /安装 2\.56\.0？/);
  assert.equal(installed, "");
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.equal(installed, "2.56.0");

  await userEvent.setup().click(screen.getByRole("button", { name: "卸载 2.50.1" }));
  assert.match(dialog.textContent, /卸载 2\.50\.1？/);
  assert.match(dialog.textContent, /其他来源的同名版本不会被修改/);
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.equal(uninstalled, "2.50.1");
});

test("installed versions without DevHub ownership have no uninstall action", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  const versions = [versionRow({ version: "2.50.1", installed: true, canUninstall: false })];
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  assert.match(dialog.textContent, /已安装/);
  assert.equal(screen.queryByRole("button", { name: /卸载/ }), null);
});

test("version query failure has a short retry action", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  let retries = 0;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, error: "查询失败", onRetry: () => { retries += 1; } }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(dialog.textContent, /版本查询失败/);
  await userEvent.setup().click(screen.getByRole("button", { name: "重试" }));
  assert.equal(retries, 1);
});

test("external Homebrew installation can be uninstalled through its formula", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  const versions = [versionRow({ version: "1.27.3", installed: true, canUninstall: true, formula: "go" })];
  let target = null;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, onUninstall: (version, formula) => { target = { version, formula }; } }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  const remove = screen.getByRole("button", { name: "卸载 1.27.3" });
  assert.equal(remove.disabled, false);
  await userEvent.setup().click(remove);
  assert.match(dialog.textContent, /卸载 1\.27\.3？/);
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.deepEqual(target, { version: "1.27.3", formula: "go" });
});

test("indeterminate progress stays a single stable track across remounts", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  const versions = [versionRow({ version: "1.27.3", installable: true })];
  const task = { jobId: 2, softwareId: "go", version: "1.27.3", action: "install", stage: "installing", message: "正在安装…", percent: null };
  for (let index = 0; index < 5; index += 1) {
    const view = render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, task }));
    const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
    const track = dialog.querySelector(".indeterminate");
    assert.ok(track);
    assert.equal(track.children.length, 0);
    view.unmount();
  }
});

test("JDK rows expose safe instances, reason for blocked instances and exact targets", async () => {
  const item = SOFTWARE.find(entry => entry.id === "jdk");
  const safeA = { source: "path", path: "/Users/test/Library/Java/JavaVirtualMachines/openjdk-26/Contents/Home/bin/java", health: "ok", managedBy: "system", canUninstall: true, mechanism: "jdkTrash", reason: null, rootPath: "/Users/test/Library/Java/JavaVirtualMachines/openjdk-26" };
  const safeB = { source: "path", path: "/Users/test/jdk-26.jdk/Contents/Home/bin/java", health: "ok", managedBy: "system", canUninstall: true, mechanism: "jdkTrash", reason: null, rootPath: "/Users/test/jdk-26.jdk" };
  const blocked = { source: "path", path: "/Users/test/Library/Java/JavaVirtualMachines/jbr-17.0.14/Contents/Home/bin/java", health: "ok", managedBy: "system", canUninstall: false, mechanism: null, reason: "JetBrains Runtime 可能被 IntelliJ IDEA 使用，暂不提供卸载", rootPath: "/Users/test/Library/Java/JavaVirtualMachines/jbr-17.0.14" };
  const versions = [
    versionRow({ version: "26", installed: true, canUninstall: true, instances: [safeA, blocked] }),
    versionRow({ version: "17.0.14", installed: true, canUninstall: false, uninstallReason: "JetBrains Runtime 可能被 IntelliJ IDEA 使用，暂不提供卸载", instances: [blocked] })
  ];
  let target = null;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, onUninstall: (version, formula, instancePath) => { target = { version, formula, instancePath }; } }));
  const dialog = await screen.findByRole("dialog", { name: "JDK 详情" });
  assert.equal(screen.getByRole("button", { name: "卸载 26" }).disabled, false);
  assert.equal(screen.queryByRole("button", { name: "卸载 17.0.14" }), null);
  await userEvent.setup().click(screen.getByRole("button", { name: "为什么不能卸载 17.0.14" }));
  assert.match(dialog.textContent, /JetBrains Runtime/);
  await userEvent.setup().click(screen.getByRole("button", { name: "卸载 26" }));
  assert.match(dialog.textContent, /openjdk-26/);
  await userEvent.setup().click(screen.getByRole("button", { name: "确认" }));
  assert.deepEqual(target, { version: "26", formula: null, instancePath: safeA.path });
});

test("multiple safe JDK instances require a concrete selection", async () => {
  const item = SOFTWARE.find(entry => entry.id === "jdk");
  const instance = suffix => ({ source: "path", path: `/Users/test/${suffix}/Contents/Home/bin/java`, health: "ok", managedBy: "system", canUninstall: true, mechanism: "jdkTrash", reason: null, rootPath: `/Users/test/${suffix}` });
  const versions = [versionRow({ version: "21", installed: true, canUninstall: true, instances: [instance("jdk-a.jdk"), instance("jdk-b.jdk")] })];
  let target = null;
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, onUninstall: (version, formula, path) => { target = path; } }));
  const dialog = await screen.findByRole("dialog", { name: "JDK 详情" });
  await userEvent.setup().click(screen.getByRole("button", { name: "卸载 21" }));
  const confirm = screen.getByRole("button", { name: "确认" });
  assert.equal(confirm.disabled, true);
  const select = screen.getByRole("combobox", { name: "选择卸载实例" });
  await userEvent.setup().selectOptions(select, "/Users/test/jdk-b.jdk/Contents/Home/bin/java");
  assert.equal(confirm.disabled, false);
  await userEvent.setup().click(confirm);
  assert.equal(target, "/Users/test/jdk-b.jdk/Contents/Home/bin/java");
});

test("install progress is per version and does not invent a percentage", async () => {
  const item = SOFTWARE.find(entry => entry.id === "go");
  const versions = [versionRow({ version: "1.27.3", installable: true })];
  const task = { jobId: 1, softwareId: "go", version: "1.27.3", action: "install", stage: "installing", message: "正在安装 1.27.3…", percent: null };
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, task }));
  const dialog = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(dialog.textContent, /正在安装 1\.27\.3…/);
  assert.ok(dialog.querySelector(".indeterminate"));
  assert.equal(dialog.querySelector(".indeterminate").children.length, 0);
  assert.equal(dialog.textContent.includes("%"), false);
  assert.equal(screen.getByRole("button", { name: "安装 1.27.3" }).disabled, true);
  cleanup();

  const failure = { jobId: 1, softwareId: "go", version: "1.27.3", action: "install", success: false, message: "安装失败：网络不可用", output: "brew stderr", verification: null };
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions, result: failure }));
  const failed = await screen.findByRole("dialog", { name: "Go 详情" });
  assert.match(failed.textContent, /安装失败：网络不可用/);
});

test("detail keeps keyboard scrolling while the scrollbar is visually hidden", async () => {
  const item = SOFTWARE.find(entry => entry.id === "git");
  render(React.createElement(DetailOverlay, { item, origin: { left: 10, top: 10, width: 90, height: 90 }, onClose: () => {}, versions: [versionRow({ version: "2.50.1", installed: true }), versionRow({ version: "2.56.0", installable: true })] }));
  const dialog = await screen.findByRole("dialog", { name: "Git 详情" });
  const inner = dialog.querySelector(".morph-card .detail-inner");
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
  assert.equal(screen.getAllByRole("button").filter(button => button.getAttribute("aria-label") && SOFTWARE.some(item => item.name === button.getAttribute("aria-label"))).length, 14);
  const user = userEvent.setup();
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
  await user.type(await screen.findByRole("textbox", { name: "搜索软件" }), "git");
  await waitFor(() => assert.equal(screen.queryByRole("button", { name: "Python" }), null));
  assert.ok(screen.getByRole("button", { name: "Git" }));
  assert.equal(screen.queryByText(/已安装/), null);
});
