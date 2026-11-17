// Smoke test for docs/design-directions-2.html: runs the page in jsdom and clicks through every feature.
// Usage: npm i jsdom@24 in any folder, then: node scripts/mockup-smoke.js docs/design-directions-2.html
// Run it before republishing the mockup; a syntax check alone misses runtime errors.
const fs = require("fs");
const { JSDOM, VirtualConsole } = require("jsdom");
const file = process.argv[2];
const html = fs.readFileSync(file, "utf8");
const errors = [];
const vc = new VirtualConsole();
vc.on("jsdomError", e => errors.push("jsdomError: " + (e.detail?.message || e.message)));
vc.on("error", e => errors.push("console.error: " + e));
const dom = new JSDOM("<!doctype html><html><head></head><body>" + html + "</body></html>", {
  runScripts: "dangerously", pretendToBeVisual: true, virtualConsole: vc,
  beforeParse(w) {
    w.matchMedia = () => ({ matches: false, addEventListener() {}, removeEventListener() {} });
    w.HTMLElement.prototype.scrollIntoView = function () {};
    w.addEventListener("error", e => errors.push("window error: " + e.message));
  },
});
const w = dom.window, d = w.document;
const wait = ms => new Promise(r => setTimeout(r, ms));
const click = (sel, opts = {}) => { const el = typeof sel === "string" ? d.querySelector(sel) : sel; if (!el) { errors.push("missing: " + sel); return null; } el.dispatchEvent(new w.MouseEvent("click", { bubbles: true, ...opts })); return el; };
const key = (target, k, o = {}) => (target || d.body).dispatchEvent(new w.KeyboardEvent("keydown", { key: k, bubbles: true, ...o }));
const step = async (name, fn) => { const before = errors.length; try { await fn(); } catch (e) { errors.push(name + ": threw " + e.message); } await wait(30); const n = errors.length - before; console.log((n ? "FAIL " : "ok   ") + name + (n ? "  -> " + errors.slice(before).join(" | ") : "")); };

(async () => {
  await wait(100);
  await step("initial render", () => {
    const tabs = d.querySelectorAll(".dirtab").length, rows = d.querySelectorAll(".app tbody tr").length, ths = d.querySelectorAll(".app th").length;
    if (!tabs || !rows) errors.push(`tabs=${tabs} rows=${rows} ths=${ths}`);
    console.log(`     dirtabs=${tabs} rows=${rows} columns=${ths}`);
  });
  await step("sort by size", () => { click('th[data-sort="size"]'); click('th[data-sort="size"]'); });
  await step("select row + tabs", () => { click(".app tbody tr:nth-child(3)"); for (let i = 0; i < 6; i++) click(`.tab[data-tab="${i}"]`); });
  await step("sidebar filter + empty state", () => { click('.fl[data-f="c-none"]'); if (!d.querySelector(".empty")) errors.push("no empty state"); click('.fl[data-f="all"]'); });
  await step("row context menu + submenu + command", async () => {
    const tr = d.querySelector(".app tbody tr"); tr.dispatchEvent(new w.MouseEvent("contextmenu", { bubbles: true, clientX: 300, clientY: 300 }));
    if (!d.querySelector(".menu[data-ctx]")) errors.push("no ctx menu");
    click('[data-sub="cat"]'); click('[data-ccmd="cat:Software"]'); });
  await step("ctx keyboard", () => { const tr = d.querySelector(".app tbody tr"); tr.dispatchEvent(new w.MouseEvent("contextmenu", { bubbles: true, clientX: 200, clientY: 200 })); key(d.activeElement, "ArrowDown"); key(d.activeElement, "Escape"); });
  await step("columns menu toggle", async () => { const th = d.querySelector(".app thead"); th.dispatchEvent(new w.MouseEvent("contextmenu", { bubbles: true, clientX: 300, clientY: 200 })); click('[data-tog="cols:tracker"]'); await wait(260); if (!d.querySelector('th[data-col="tracker"]')) errors.push("tracker col not shown"); click('[data-tog="cols:tracker"]'); await wait(260); d.body.click(); });
  await step("toolbar editor", () => { click('[data-act="cust-tb"]'); click('[data-tri="magnet:label"]'); click('[data-tri="rss:icon"]'); click('[data-all="reset"]'); d.body.click(); });
  await step("settings + search + app icon", () => { click('[data-act="prefs"]'); const q = d.getElementById("sq"); if (!q) { errors.push("no settings"); return; } q.value = "tray"; q.dispatchEvent(new w.Event("input", { bubbles: true })); click('[data-icon="0"]'); click(".switch"); click("[data-closesheet]"); });
  await step("missing files callout + locate", async () => { const tr=[...d.querySelectorAll(".app tbody tr")].find(t=>t.textContent.includes("Elephants")); if(!tr){errors.push("no missing row");return;} click(tr); click('.tab[data-tab="0"]'); if(!d.querySelector(".callout")) errors.push("no callout"); click('[data-fix="locate"]'); await wait(1800); });
  await step("shortcuts sheet", () => { key(d.body, "?"); click("[data-closesheet]"); });
  await step("keyboard list nav", () => { const l = d.querySelector("[data-list]"); l.focus(); key(l, "ArrowDown"); key(l, " "); key(l, "End"); key(l, "b", { ctrlKey: true }); key(l, "b", { ctrlKey: true }); });
  await step("menubar (windows)", () => { click('.app [data-menu="0"]'); key(d.activeElement, "ArrowRight"); key(d.activeElement, "Escape"); });
  await step("view controls", () => { for (const v of ["mode:dark", "tint:tinted", "dens:compact", "scale:4", "plat:mac"]) click(`[data-v="${v}"]`); });
  await step("mac menubar", () => { click('.macbar [data-menu="1"]'); key(d.activeElement, "Escape"); });
  await step("replay startup", async () => { click('[data-act="replay"]'); await wait(1200); });
  await step("other directions", () => { for (const id of ["gd", "hm", "wk", "ds"]) click(`.dirtab[data-dir="${id}"]`); });
  await step("logos tab", () => { click('.dirtab[data-dir="logo"]'); if (!d.querySelectorAll(".lg").length) errors.push("no logos"); click('.dirtab[data-dir="ds"]'); });
  await wait(1500);
  console.log(errors.length ? `\n${errors.length} problem(s)` : "\nall steps passed");
  process.exit(errors.length ? 1 : 0);
})();
