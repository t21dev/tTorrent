# tTorrent website

This is the `landing` branch of [t21dev/tTorrent](https://github.com/t21dev/tTorrent). It holds **only** the website at <https://ttorrent.t21.dev>. The app lives on `main`.

This branch is an orphan. Never merge it with `main`.

## Stack

- React 19 + TypeScript + Vite, prerendered to static HTML at build time (`src/entry-server.tsx` → `scripts/prerender.mjs`), then hydrated.
- Plain CSS with tokens in `src/styles/tokens.css`, taken from the app's Dark theme (`src/gui/dose/theme/themes/dark.json` on `main`) with the default Emerald accent, as described in `ttorrent/DESIGN.md`. Instrument Sans and DM Mono, the fonts the app bundles, are bundled here too, so the page makes no third-party requests.
- Cloudflare Workers static assets (`wrangler.jsonc`), custom domain `ttorrent.t21.dev`.

## Commands

```bash
npm install
npm run dev       # local dev server
npm run build     # fetch latest release, typecheck, build, prerender
npm run check     # verify SEO tags, OG image, sitemap, alt text in dist/
npm run og        # re-render public/og.png from the release data
npm run deploy    # build and `wrangler deploy` (needs `wrangler login`)
```

## Content that comes from the app

- **Release data.** `scripts/release.mjs` asks the GitHub API for the latest release at build time and writes `src/release.json`: the version, date, and the installer, portable zip and checksum links with sizes. If GitHub is unreachable, the committed `release.json` is used. tTorrent ships for Windows only; on macOS and Linux the page says so instead of offering a file.
- **Copy.** Every sentence is from `README.md` on `main`. Do not add a feature that is not listed there.
- **Numbers.** Every figure is from the README's "Compared with qBittorrent" section, measured on one PC. Do not add a number that is not measured there.
- **Screenshots.** `public/screenshots/*.webp` are WebP copies of `ttorrent/docs/screenshots/*.png` on `main` (full width, plus a 1000px copy when the original is wider). Their sizes are in `SHOTS` in `src/content.ts`. When they change, re-export them with ffmpeg:
  `ffmpeg -i main-dark.png -vf "scale='min(2000,iw)':-2" -c:v libwebp -quality 82 main-dark.webp`
- **Icons.** `public/favicon.ico` is `ttorrent/brand/ttorrent.ico`; the PNG icons are `ttorrent/brand/icon.svg` rendered with resvg.

## Deploying

For now deploys are manual: `npm run deploy`. To deploy on push with Workers Builds, connect this repository, set the production branch to `landing`, build command `npm run build`, deploy command `npx wrangler deploy`. Then a push here deploys. Re-run the build after each app release so the version and download links update (or trigger a deploy hook from the release workflow on `main`).

## SEO

`index.html` carries the title, description, canonical, Open Graph and Twitter tags and a JSON-LD `SoftwareApplication`; `scripts/prerender.mjs` fills in the version and size, and writes `404.html` and `sitemap.xml`. `public/robots.txt` points at the sitemap. `npm run check` fails if any of that is missing.

## Licence

tTorrent is GPL and began as a fork of [qBittorrent](https://www.qbittorrent.org/). This website is GPL-3.0-or-later too.
