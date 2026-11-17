// Layout check for docs/design-directions-2.html in real Chrome: menus, submenus, sheets and the status bar stay inside the window; no menu text, table cell or status item is cut off.
// Usage: npm i puppeteer-core@23 in any folder, then: node scripts/mockup-layout.js <abs path to html> <screenshot dir> [gd|hm|wk]
// Real-browser layout check: menus, submenus and sheets must stay inside the app window, and menu text must not be cut off.
const puppeteer = require("puppeteer-core");
const path = require("path");
const file = process.argv[2], shots = process.argv[3];
(async () => {
  const browser = await puppeteer.launch({ executablePath: "C:/Program Files/Google/Chrome/Application/chrome.exe", headless: "new", args: ["--no-sandbox"] });
  const page = await browser.newPage();
  await page.setViewport({ width: 1600, height: 1100 });
  const errs = []; page.on("pageerror", e => errs.push("pageerror: " + e.message));
  await page.goto("file:///" + file.replace(/\\/g, "/"), { waitUntil: "load" });
  await new Promise(r => setTimeout(r, 600));
  if (process.argv[4]) { await page.evaluate(d => document.querySelector(`.dirtab[data-dir="${d}"]`).click(), process.argv[4]); await new Promise(r => setTimeout(r, 300)); }
  const problems = [];
  const check = async (label) => {
    const res = await page.evaluate(() => {
      const app = document.querySelector(".app"); if (!app) return ["no app"];
      const a = app.getBoundingClientRect(), out = [];
      const inside = (r) => r.left >= a.left - 1 && r.top >= a.top - 1 && r.right <= a.right + 1 && r.bottom <= a.bottom + 1;
      document.querySelectorAll(".app .menu, .app .sheet").forEach(m => {
        const r = m.getBoundingClientRect(); const name = m.className + (m.dataset.ctx ? "[ctx]" : "");
        if (!inside(r)) out.push(`${name} outside app: menu ${Math.round(r.left)},${Math.round(r.top)}-${Math.round(r.right)},${Math.round(r.bottom)} app ${Math.round(a.left)},${Math.round(a.top)}-${Math.round(a.right)},${Math.round(a.bottom)}`);
        m.querySelectorAll(":scope > button, :scope .row, :scope .trow, :scope .tbr .tn").forEach(b => { if (b.scrollWidth > b.clientWidth + 1) out.push(`${name}: text cut "${b.textContent.trim().slice(0, 40)}"`); });
      });
      document.querySelectorAll(".app td, .app th").forEach(c => { const k = c.dataset.c || c.dataset.col; if (["name", "path", "dpath", "tracker", "hash1"].includes(k)) return; if (c.scrollWidth > c.clientWidth + 1) out.push(`cell cut: ${k} "${c.textContent.trim().slice(0, 30)}"`); });
      const zz = parseFloat(app.style.zoom) || 1; document.querySelectorAll(".app .sb > span:not([hidden]):not(.sp)").forEach(x => { if (x.getBoundingClientRect().height / zz > 22 || x.scrollWidth > x.clientWidth + 2) out.push(`status bar item wraps or is cut: "${x.textContent.trim().slice(0, 30)}"`); });
      const sbb = document.querySelector(".app .sb"); if (sbb && sbb.scrollWidth > sbb.clientWidth + 1) out.push("status bar overflows");
      document.querySelectorAll(".desk > .menu").forEach(m => { const r = m.getBoundingClientRect(), dr = m.parentElement.getBoundingClientRect(); if (r.right > dr.right + 1 || r.bottom > dr.bottom + 1) out.push("mac menu outside desk"); });
      return out;
    });
    res.forEach(x => problems.push(`[${label}] ${x}`));
  };
  const click = (sel) => page.evaluate(s => { const e = document.querySelector(s); if (e) e.click(); return !!e; }, sel);
  const combos = [["win", 2, "cozy"], ["win", 0, "compact"], ["win", 5, "comfortable"], ["mac", 2, "cozy"], ["mac", 5, "compact"]];
  let n = 0;
  for (const [plat, scale, dens] of combos) {
    await click(`[data-v="plat:${plat}"]`); await click(`[data-v="scale:${scale}"]`); await click(`[data-v="dens:${dens}"]`);
    await new Promise(r => setTimeout(r, 150));
    const tag = `${plat} ${[80, 90, 100, 110, 125, 150][scale]}% ${dens}`;
    // right-click the last visible row near the right edge of the list
    for (const where of ["bottom-right", "top-left"]) {
      const pt = await page.evaluate(w => { const l = document.querySelector(".app [data-list]").getBoundingClientRect(); const rows = [...document.querySelectorAll(".app tbody tr")]; const vis = rows.filter(r => { const b = r.getBoundingClientRect(); return b.bottom < l.bottom && b.top > l.top; }); const r = (w === "bottom-right" ? vis[vis.length - 1] : vis[0]).getBoundingClientRect(); return { x: w === "bottom-right" ? Math.min(l.right, window.innerWidth) - 8 : r.left + 20, y: r.top + r.height / 2 }; }, where);
      await page.mouse.click(pt.x, pt.y, { button: "right" }); await new Promise(r => setTimeout(r, 260));
      await check(`${tag} ctx ${where}`);
      for (const sub of ["cat", "tags", "queue", "copy"]) {
        const ok = await page.evaluate(s => { const b = document.querySelector(`.menu[data-ctx] [data-sub="${s}"]`); if (!b) return false; b.dispatchEvent(new MouseEvent("mouseover", { bubbles: true })); return true; }, sub);
        if (ok) { await new Promise(r => setTimeout(r, 240)); await check(`${tag} ctx ${where} › ${sub}`); }
      }
      if (where === "bottom-right" && plat === "win" && scale === 5) await page.screenshot({ path: path.join(shots, `ctx-${++n}.png`) });
      await page.keyboard.press("Escape");
    }
    // column menu from the header, toolbar editor, settings, shortcuts
    const th = await page.evaluate(() => { const t = [...document.querySelectorAll(".app th")].pop().getBoundingClientRect(); return { x: Math.min(t.left + 10, window.innerWidth - 10), y: t.top + 10 }; });
    await page.mouse.click(th.x, th.y, { button: "right" }); await new Promise(r => setTimeout(r, 260)); await check(`${tag} columns menu`); await page.keyboard.press("Escape");
    await click('[data-act="cust-tb"]'); await new Promise(r => setTimeout(r, 260)); await check(`${tag} toolbar editor`); await page.keyboard.press("Escape");
    await click('[data-act="prefs"]'); await new Promise(r => setTimeout(r, 450)); await check(`${tag} settings`);
    if (scale === 5) await page.screenshot({ path: path.join(shots, `settings-${plat}-${dens}.png`) });
    await click("[data-closesheet]");
    await click('[data-act="keys"]'); await new Promise(r => setTimeout(r, 450)); await check(`${tag} shortcuts`); await click("[data-closesheet]");
    const mb = plat === "mac" ? '.macbar [data-menu="3"]' : '.app [data-menu="2"]';
    await click(mb); await new Promise(r => setTimeout(r, 260)); await check(`${tag} menubar`); await page.keyboard.press("Escape");
  }
  await browser.close();
  const all = [...errs, ...problems];
  console.log(all.length ? all.join("\n") + `\n\n${all.length} problem(s)` : "layout: no overflow or clipping found");
})();
