import release from "./release.json";

const GH = "https://github.com/t21dev/tTorrent";

export const SITE = {
  url: "https://ttorrent.t21.dev",
  repo: GH,
  readme: `${GH}#readme`,
  releases: `${GH}/releases`,
  compare: `${GH}#compared-with-qbittorrent`,
  method: `${GH}/blob/main/ttorrent/VERIFY.md`,
  troubleshooting: `${GH}#troubleshooting`,
  license: `${GH}/blob/main/COPYING`,
  upstream: "https://www.qbittorrent.org/",
  libtorrent: "https://www.libtorrent.org/",
  mcp: "https://modelcontextprotocol.io",
};

export type Asset = { name: string; url: string; size: number };
type Assets = Partial<Record<"windows" | "windowsPortable" | "checksums", Asset>>;
export const RELEASE = release as { version: string; date: string; page: string; assets: Assets };

/** Bytes to MB the way Windows Explorer shows it (MiB), one decimal. */
export const mb = (bytes: number) => (bytes / 1048576).toFixed(1);

export const installerMB = RELEASE.assets.windows ? mb(RELEASE.assets.windows.size) : "22.4";

/** Screenshot sizes, from ttorrent/docs/screenshots on main. */
export const SHOTS: Record<string, [number, number]> = {
  "main-dark": [1400, 863],
  "main-light": [1400, 863],
  "add-dialog": [1400, 1057],
  "command-palette": [1163, 788],
  "keyboard-shortcuts": [1400, 917],
  "theme-editor": [1400, 965],
  "memory-card": [814, 486],
};

// Every figure below is from the README's "Compared with qBittorrent" section:
// measured on 18 November 2026 on one PC with a copy of a real profile of about 735 torrents.
export const SPECS: { label: string; qbt: string; ours: string; unit: string }[] = [
  { label: "Memory, last 10 of 30 minutes", qbt: "127", ours: "117", unit: "MB" },
  { label: "Private bytes at 30 minutes", qbt: "198", ours: "187", unit: "MB" },
  { label: "CPU, average over 30 minutes", qbt: "0.17", ours: "0.12", unit: "%" },
  { label: "Read from disk, first 2 minutes", qbt: "31", ours: "23", unit: "MB" },
];

export const RUN_30: [string, string, string][] = [
  ["Memory, average", "141 MB", "129 MB"],
  ["Memory, last 10 minutes", "127 MB", "117 MB"],
  ["Private bytes at 30 min", "198 MB", "187 MB"],
  ["CPU, average", "0.17 %", "0.12 %"],
  ["Read from disk", "77 MB", "72 MB"],
];

export const START_2: [string, string, string][] = [
  ["Memory at 2 min", "162–190 MB", "173–177 MB"],
  ["Peak", "270–286 MB", "263–272 MB"],
  ["Private bytes at 2 min", "196–199 MB", "179–184 MB"],
  ["Read from disk", "31 MB", "23 MB"],
];

export const LEFT_OUT: [string, string][] = [
  ["qBittorrent's update checker", "Replaced by tTorrent's own update check"],
  ["Email on completion", "Replaced by webhooks"],
  ["Search engine and search plugins", "Left out"],
  ["Embedded tracker", "Left out"],
  ["I2P", "Left out"],
  ["Peer host names", "Left out"],
  ["Alternative Web UI", "Left out"],
  ["Donate menu item", "Removed"],
  ["Legal notice on first start", "Removed"],
  ["Execution log", "Keeps 2,000 messages instead of 20,000"],
  ["Known peers per torrent", "1,500 by default instead of 3,000 (Settings › Advanced, up to 3,000)"],
];

export type TourItem = {
  id: string;
  shot: string;
  alt: string;
  title: string;
  body: string;
  points: string[];
};

export const TOUR: TourItem[] = [
  {
    id: "palette",
    shot: "command-palette",
    alt: "The command palette listing commands with their shortcuts",
    title: "Command palette",
    body: "Press Ctrl+K and run any command by typing part of its name.",
    points: ["Saved views: the current filters, search and sort as a sidebar entry"],
  },
  {
    id: "shortcuts",
    shot: "keyboard-shortcuts",
    alt: "The keyboard shortcuts window, grouped by menu",
    title: "Keyboard shortcuts",
    body: "Ctrl+/ shows every shortcut in one window, and you can change any of them.",
    points: [],
  },
  {
    id: "add",
    shot: "add-dialog",
    alt: "The Add dialog with seeds and peers from the trackers before adding",
    title: "Health in the Add dialog",
    body: "Seeds and peers from the trackers before you add a torrent.",
    points: ["Private trackers are never asked: they count every request on your passkey"],
  },
  {
    id: "memory",
    shot: "memory-card",
    alt: "The memory card with a graph since start",
    title: "Memory card",
    body: "Click Memory in the status bar to see memory now, the peak and a graph since start.",
    points: ["tTorrent hands freed memory back to Windows when it's in the background"],
  },
  {
    id: "light",
    shot: "main-light",
    alt: "The main window in the Light theme",
    title: "Themes",
    body: "17 of them, plus Follow system and your own from the theme editor.",
    points: [
      "Light: Light, Paper, Latte, Solarized, Blossom",
      "Dark: Dark, Ink, Charcoal, Nord, Dracula, Mocha, Tokyo, Rosé Pine, Gruvbox, Vampire, Aurora, Ember",
    ],
  },
  {
    id: "editor",
    shot: "theme-editor",
    alt: "The theme editor with a live preview and contrast ratios",
    title: "Theme editor",
    body: "Make your own theme with a live preview. Any accent colour, with its lightness adjusted to 3:1 contrast on the theme.",
    points: [
      "Density: Dense, Compact or Standard. Interface size: 80 to 150 %",
      "Toolbar: slim or regular, choose which buttons show",
      "Progress bars: Line, Bar, or Filled with the percentage inside",
    ],
  },
];

export const FEATURES: { title: string; items: [string, string][] }[] = [
  {
    title: "Looking after itself",
    items: [
      ["Low disk space guard", "Stops downloads before a drive fills, and starts them again when there's room."],
      ["Backup and restore", "Settings and the torrent list with each torrent's progress. Back up now, restore later, or back up every day automatically."],
      ["Update check", "From GitHub releases. Nothing is downloaded without you."],
    ],
  },
  {
    title: "Connecting",
    items: [
      ["Webhooks", "A URL is called with JSON when a torrent is added, finishes or fails."],
      ["Unpack when finished", ".zip, .rar and .7z files are extracted next to the archive."],
      ["Pair a phone", "A QR code opens the Web UI on your phone."],
    ],
  },
  {
    title: "Everyday",
    items: [
      ["Relocate", "Files you moved or renamed outside the app? Point the torrent at them and it rechecks instead of downloading again."],
      ["Notification cards", "With Open folder and Show when a torrent finishes or fails."],
      ["Rename categories and tags", "With subcategories and every torrent following."],
    ],
  },
];

export const AGENT_FACTS: string[] = [
  "Off by default, and reachable from this computer only.",
  "One key, which can look only, or look and manage.",
  "Agents can't delete downloaded files.",
];
