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
  assert.ok(filterCurated(SOFTWARE, "NODE", "开发工具").some(item => item.id === "node"));
  assert.ok(filterCurated(SOFTWARE, "GIT", "开发工具").some(item => item.id === "git"));
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
  await user.click(screen.getByRole("button", { name: "清空搜索" }));
  assert.equal(screen.getByTestId("state").textContent, "|全部|system");
  assert.equal(document.activeElement, input);
  assert.ok(screen.getByRole("textbox", { name: "搜索软件" }));
  await user.type(input, "node");
  await user.click(screen.getByRole("button", { name: "开发工具" }));
  assert.equal(screen.getByRole("button", { name: "开发工具" }).getAttribute("aria-pressed"), "true");
  assert.equal(screen.queryByRole("textbox"), null);
  assert.equal(screen.queryByRole("menu"), null);
  await user.click(screen.getByRole("button", { name: "外观设置" }));
  await user.click(screen.getByRole("menuitemradio", { name: "夜间" }));
  assert.equal(screen.queryByRole("menu"), null);
  assert.equal(screen.getByTestId("state").textContent, "node|开发工具|dark");
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
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
  assert.match(dialog.textContent, /下载或网络连接出现问题/);
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
  assert.equal(document.querySelectorAll(".orb.is-installed").length, 0);
});

test("changing the search hides previous remote results before the next request completes", async () => {
  const previous = window.__TAURI_INTERNALS__;
  const previousEvents = window.__TAURI_EVENT_PLUGIN_INTERNALS__;
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__TAURI_INTERNALS__ = {
    transformCallback: () => 1,
    invoke: async (command, args) => {
      if (command === "get_installed_packages") return { formulae: [], casks: [] };
      if (command === "get_detection_snapshot") return null;
      if (command === "get_app_status") return { phase: 3, product: "DevHub V2" };
      if (command === "search_homebrew") {
        if (args.query === "fixture") return [{ package: "remote-fixture", kind: "formula", name: "RemoteFixture", description: "test only", version: null }];
        return new Promise(() => {});
      }
      return false;
    }
  };
  try {
    const { default: App } = await import("../src/App.tsx");
    render(React.createElement(App));
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "搜索软件" }));
    const input = screen.getByRole("textbox", { name: "搜索软件" });
    await user.type(input, "fixture");
    await screen.findByRole("button", { name: "RemoteFixture" });
    await user.click(screen.getByRole("button", { name: "清空搜索" }));
    await user.type(input, "memcached");
    assert.equal(screen.queryByRole("button", { name: "RemoteFixture" }), null);
    assert.ok(screen.getByRole("button", { name: "Memcached" }));
  } finally {
    cleanup();
    await new Promise(resolve => setTimeout(resolve, 0));
    window.__TAURI_INTERNALS__ = previous;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = previousEvents;
  }
});


test("failure guidance distinguishes verification from network errors and preserves cautious advice", async () => {
  const { failureFeedback } = await import("../src/lib/taskFeedback.ts");
  assert.match(failureFeedback("安装命令结束，但无法确认 Homebrew 状态", "curl: failed").summary, /尚未确认/);
  assert.match(failureFeedback("failed", "SHA256 checksum mismatch").summary, /校验/);
  assert.match(failureFeedback("failed", "Permission denied").summary, /权限/);
  assert.match(failureFeedback("failed", "curl: (6) Could not resolve host").summary, /网络/);
  assert.match(failureFeedback("检测命令执行超时").advice, /并不保证/);
  assert.equal(failureFeedback("private ".repeat(100)).summary.includes("private"), false);
});


test("detail keeps the source radius and toggles introduction only on click without removing its summary", async () => {
  globalThis.__REDUCE__ = true;
  const item = SOFTWARE.find(item => item.id === "memcached");
  render(React.createElement(DetailOverlay, { item, origin: { left: 30, top: 30, width: 100, height: 100 }, onClose() {}, homebrewReady: true }));
  const dialog = screen.getByRole("dialog", { name: "Memcached 详情" });
  assert.equal(dialog.style.borderRadius, "50px");
  const info = screen.getByRole("button", { name: "详细介绍" });
  const user = userEvent.setup();
  await user.hover(info);
  assert.equal(screen.queryByRole("region", { name: "软件详细介绍" }), null);
  info.focus();
  assert.equal(screen.queryByRole("region", { name: "软件详细介绍" }), null);
  assert.equal(info.querySelectorAll("circle").length, 1);
  assert.ok(dialog.querySelector(".detail-head").textContent.includes(item.description));
  await user.click(info);
  await user.unhover(info);
  assert.ok(screen.getByRole("region", { name: "软件详细介绍" }));
  assert.ok(dialog.querySelector(".detail-head").textContent.includes(item.description));
  assert.ok(screen.getByRole("heading", { name: "使用条件" }));
  await user.click(info);
  assert.equal(screen.queryByRole("region", { name: "软件详细介绍" }), null);
});

test("opening a software dismisses search without losing the selected detail", async () => {
  const { default: App } = await import("../src/App.tsx");
  render(React.createElement(App));
  const user = userEvent.setup();
  assert.equal(document.querySelector(".brand"), null);
  await user.click(screen.getByRole("button", { name: "搜索软件" }));
  await user.type(screen.getByRole("textbox", { name: "搜索软件" }), "memcached");
  await user.click(screen.getByRole("button", { name: "Memcached" }));
  assert.equal(screen.queryByRole("textbox"), null);
  assert.ok(screen.getByRole("dialog", { name: "Memcached 详情" }));
});


test("Memcached introduction explains the software without repeated summaries or generic operation copy", async () => {
  globalThis.__REDUCE__ = true;
  const item = SOFTWARE.find(entry => entry.id === "memcached");
  render(React.createElement(DetailOverlay, {
    item, origin: { left: 30, top: 30, width: 100, height: 100 },
    onClose() {}, homebrewReady: true
  }));
  await userEvent.setup().click(screen.getByRole("button", { name: "详细介绍" }));
  const introduction = screen.getByRole("region", { name: "软件详细介绍" });
  assert.match(introduction.textContent, /缓存|键值/);
  assert.match(introduction.textContent, /内存|分布式/);
  assert.equal(introduction.textContent.includes(item.description), false);
  for (const unwanted of [
    "由 Homebrew 提供当前版本",
    "DevHub 会在操作前请求确认",
    "安装结束后重新核对包状态",
    "检测到的已有安装可能来自系统、官网或其他工具",
    "只有 Homebrew 管理的软件包提供卸载操作"
  ]) {
    assert.equal(introduction.textContent.includes(unwanted), false, unwanted);
  }
});

function colorPaints(svg) {
  return new Set([svg, ...svg.querySelectorAll("[fill], [stroke]")]
    .flatMap(element => [element.getAttribute("fill"), element.getAttribute("stroke")])
    .filter(value => value && /^#[0-9a-f]{3,8}$/i.test(value))
    .map(value => value.toLowerCase()));
}

test("brand icons keep color and multi-color artwork across themes instead of black or inverted images", async () => {
  const { SoftwareLogo } = await import("../src/components/SoftwareLogo.tsx");
  for (const id of ["python", "mysql"]) {
    const view = render(React.createElement(SoftwareLogo, { id, name: id }));
    const colors = colorPaints(view.container.querySelector("svg"));
    colors.delete("#fff");
    colors.delete("#ffffff");
    assert.ok(colors.size >= 2, `${id} lost its multiple colored fills`);
    view.unmount();
  }
  const ruby = readFileSync(fileURLToPath(new URL("../public/software-icons/ruby.svg", import.meta.url)), "utf8");
  const document = new dom.window.DOMParser().parseFromString(ruby, "image/svg+xml");
  assert.equal(document.querySelector("parsererror"), null);
  const colors = colorPaints(document.documentElement);
  assert.ok([...colors].some(color => !["#000", "#000000", "#fff", "#ffffff"].includes(color)), "bundled Ruby artwork must carry brand color");
  const styles = readFileSync(fileURLToPath(new URL("../src/styles.css", import.meta.url)), "utf8");
  assert.doesNotMatch(styles, /\.brand-logo[^{}]*\{[^}]*\bfilter\s*:\s*invert\(/);
  const fallback = render(React.createElement(SoftwareLogo, { id: "fixture-without-brand-artwork", name: "Fixture" }));
  const svg = fallback.container.querySelector("svg");
  assert.ok(svg);
  assert.equal(svg.querySelector("text"), null);
  assert.ok([...colorPaints(svg)].some(color => !["#000", "#000000", "#fff", "#ffffff"].includes(color)), "fallback must be a colored tool glyph");
});


test("theme menu exposes one trigger and supports keyboard, escape and outside dismissal", async () => {
  const { ThemePicker } = await import("../src/components/ThemePicker.tsx");
  let selected = "system";
  render(React.createElement("div", null, React.createElement(ThemePicker, { theme: "system", onTheme: value => { selected = value; } }), React.createElement("button", null, "外部操作")));
  const user = userEvent.setup();
  const trigger = screen.getByRole("button", { name: "外观设置" });
  assert.equal(screen.queryByRole("menu"), null);
  await user.click(trigger);
  assert.equal(screen.getAllByRole("menuitemradio").length, 3);
  assert.equal(screen.getByRole("menuitemradio", { name: "自动" }).getAttribute("aria-checked"), "true");
  await user.keyboard("{Home}{ArrowDown}{Enter}");
  assert.equal(selected, "dark");
  assert.equal(screen.queryByRole("menu"), null);
  await user.click(trigger);
  await user.keyboard("{Escape}");
  assert.equal(screen.queryByRole("menu"), null);
  assert.equal(document.activeElement, trigger);
  await user.click(trigger);
  await user.click(screen.getByRole("button", { name: "外部操作" }));
  assert.equal(screen.queryByRole("menu"), null);
  await user.click(trigger);
  fireEvent.wheel(document.body);
  assert.equal(screen.queryByRole("menu"), null);
});

async function withDetectionFixture(options, exercise) {
  const previous = window.__TAURI_INTERNALS__;
  const previousEvents = window.__TAURI_EVENT_PLUGIN_INTERNALS__;
  const callbacks = new Map();
  const listeners = new Map();
  let seq = 0, refreshes = 0;
  const snapshot = {
    schemaVersion: 1, refreshedAt: 1, stale: false, durationMs: 1,
    homebrew: { available: true, prefix: "/fixture", version: "fixture", error: null },
    instances: [{ ...instance("python"), health: "broken", path: "/fixture/python" }], errors: []
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__TAURI_INTERNALS__ = {
    transformCallback(callback) { callbacks.set(++seq, callback); return seq; },
    async invoke(command, args) {
      if (command === "plugin:event|listen") { listeners.set(args.event, args.handler); return args.handler; }
      if (command === "get_detection_snapshot") return options.missing ? null : snapshot;
      if (command === "refresh_detection") {
        refreshes++;
        if (!options.missing) queueMicrotask(() => callbacks.get(listeners.get("detection://updated"))?.({ payload: { ...snapshot, refreshedAt: refreshes + 1 }, id: 1, event: "detection://updated" }));
        return true;
      }
      if (command === "get_installed_packages") {
        if (options.missing || (options.failOnRefresh && refreshes > 1)) throw new Error("fixture query failed");
        return { formulae: [{ package: "memcached", version: "1.6.fixture", path: "/fixture/memcached" }], casks: [] };
      }
      if (command === "search_homebrew") return [];
      return null;
    }
  };
  try {
    const { default: App } = await import("../src/App.tsx");
    render(React.createElement(App));
    await exercise(userEvent.setup());
  } finally {
    cleanup();
    await new Promise(resolve => setTimeout(resolve, 0));
    window.__TAURI_INTERNALS__ = previous;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = previousEvents;
  }
}

test("sidebar combines verified package and non-Homebrew installs with keyword and category filters", async () => {
  await withDetectionFixture({}, async user => {
    await waitFor(() => assert.equal(screen.getByLabelText("未安装数量").textContent, "98"));
    await user.click(screen.getByRole("button", { name: /^已安装/ }));
    assert.equal(document.querySelectorAll(".orb").length, 2);
    assert.ok(screen.getByRole("button", { name: "Python" }));
    assert.ok(screen.getByRole("button", { name: "Memcached" }));
    await user.click(screen.getByRole("button", { name: "数据库与服务" }));
    assert.equal(document.querySelectorAll(".orb").length, 1);
    await user.click(screen.getByRole("button", { name: "搜索软件" }));
    await user.type(screen.getByRole("textbox", { name: "搜索软件" }), "python");
    assert.ok(screen.getByRole("button", { name: "Python" }));
    await user.click(screen.getByRole("button", { name: /^未安装/ }));
    assert.equal(document.querySelectorAll(".orb").length, 0);
    assert.equal(screen.queryByRole("textbox"), null);
    assert.ok(screen.getByRole("button", { name: "清除关键词" }));
    await user.click(screen.getByRole("button", { name: /^全部软件/ }));
    assert.ok(screen.getByRole("button", { name: "Python" }));
    await user.click(screen.getByRole("button", { name: "清除关键词" }));
    await user.click(screen.getByRole("button", { name: "全部", exact: true }));
    assert.equal(document.querySelectorAll(".orb").length, 100);
  });
});

test("unknown installation state is not classified as 100 uninstalled applications", async () => {
  await withDetectionFixture({ missing: true }, async user => {
    await user.click(screen.getByRole("button", { name: /^未安装/ }));
    assert.equal(document.querySelectorAll(".orb").length, 0);
    assert.equal(screen.getByLabelText("未安装数量").textContent, "—");
    assert.ok(screen.getByText(/暂不能确认未安装列表/));
    await user.click(screen.getByRole("button", { name: /^全部软件/ }));
    assert.equal(document.querySelectorAll(".orb").length, 100);
  });
});

test("package query errors preserve positive evidence and never manufacture an uninstalled count", async () => {
  await withDetectionFixture({ failOnRefresh: true }, async user => {
    await waitFor(() => assert.equal(screen.getByLabelText("未安装数量").textContent, "98"));
    await user.click(screen.getByRole("button", { name: "刷新检测" }));
    await waitFor(() => assert.equal(screen.getByLabelText("未安装数量").textContent, "—"));
    await user.click(screen.getByRole("button", { name: /^已安装/ }));
    assert.ok(screen.getByRole("button", { name: "Memcached" }));
    assert.ok(screen.getByRole("button", { name: "Python" }));
    await user.click(screen.getByRole("button", { name: /^未安装/ }));
    assert.equal(document.querySelectorAll(".orb").length, 0);
  });
});


test("every curated software has a substantial introduction and concrete usage conditions", async () => {
  const { softwareDescription } = await import("../src/lib/softwareDescriptions.ts");
  for (const item of SOFTWARE) {
    const description = softwareDescription(item);
    assert.ok(description, `${item.id} has no introduction`);
    assert.ok(description.purpose.length >= 80, `${item.id} reverted to a one-line introduction`);
    assert.ok(description.uses.length > 0, `${item.id} has no use case`);
    assert.ok(description.requirements.length > 0, `${item.id} has no usage condition`);
    assert.ok(description.requirements.every(value => value.trim().length > 0));
  }
});
