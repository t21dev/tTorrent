// Renders public/og.png (1200x630) from an SVG with resvg: the real main window
// screenshot, the app icon and the release version. Run `npm run og` after a release.
import { readFile, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import { Resvg } from "@resvg/resvg-js";
import wawoff2 from "wawoff2";

const require = createRequire(import.meta.url);
const root = new URL("..", import.meta.url);
const release = JSON.parse(await readFile(new URL("src/release.json", root), "utf8"));

// resvg needs TTF; fontsource ships WOFF2.
const dir = join(tmpdir(), "ttorrent-og-fonts");
await mkdir(dir, { recursive: true });
const fonts = [];
for (const f of [
  "instrument-sans/files/instrument-sans-latin-500-normal",
  "instrument-sans/files/instrument-sans-latin-600-normal",
  "dm-mono/files/dm-mono-latin-400-normal",
]) {
  const src = require.resolve(`@fontsource/${f}.woff2`);
  const out = join(dir, f.split("/").pop() + ".ttf");
  await writeFile(out, await wawoff2.decompress(await readFile(src)));
  fonts.push(out);
}

const shot = (await readFile(new URL("scripts/assets/og-shot.jpg", root))).toString("base64");

const svg = `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="1200" height="630" viewBox="0 0 1200 630">
  <defs>
    <clipPath id="c"><rect x="620" y="110" width="800" height="560" rx="18"/></clipPath>
    <linearGradient id="fade" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0.45" stop-color="#141415" stop-opacity="0"/>
      <stop offset="1" stop-color="#141415" stop-opacity="1"/>
    </linearGradient>
  </defs>
  <rect width="1200" height="630" fill="#141415"/>
  <image x="620" y="110" width="800" height="530" clip-path="url(#c)" xlink:href="data:image/jpeg;base64,${shot}" preserveAspectRatio="xMinYMin slice"/>
  <rect x="620" y="110" width="800" height="560" rx="18" fill="none" stroke="#ffffff" stroke-opacity="0.08"/>
  <rect x="600" y="300" width="620" height="330" fill="url(#fade)"/>
  <g transform="translate(70 66) scale(0.1875)">
    <rect width="256" height="256" rx="58" fill="#10a874"/>
    <g fill="none" stroke="#fff" stroke-width="26" stroke-linecap="round" stroke-linejoin="round">
      <path d="M128 112v94"/><path d="M84 164l44 44 44-44"/>
    </g>
    <path d="M128 116c0-36 22-60 70-60 0 40-22 64-70 60Z M128 116c0-26-18-46-54-46 0 30 18 50 54 46Z" fill="#fff"/>
  </g>
  <text x="128" y="101" font-family="Instrument Sans" font-weight="600" font-size="30" fill="#ececee" letter-spacing="-0.3">tTorrent</text>
  <text font-family="Instrument Sans" font-weight="600" font-size="62" fill="#ececee" letter-spacing="-2">
    <tspan x="66" y="300">A fast, modern</tspan>
    <tspan x="66" y="370">torrent client</tspan>
    <tspan x="66" y="440">for Windows.</tspan>
  </text>
  <text x="70" y="510" font-family="Instrument Sans" font-weight="500" font-size="24" fill="#a1a1a7">Built on qBittorrent. Free and open source.</text>
  <text x="70" y="580" font-family="DM Mono" font-size="18" fill="#8c8c92">ttorrent.t21.dev · v${release.version} · Windows</text>
</svg>`;

const png = new Resvg(svg, {
  fitTo: { mode: "width", value: 1200 },
  font: { fontFiles: fonts, loadSystemFonts: false, defaultFontFamily: "Instrument Sans" },
})
  .render()
  .asPng();
await writeFile(new URL("public/og.png", root), png);
console.log(`og: public/og.png (${(png.length / 1024).toFixed(0)} KB, v${release.version})`);
