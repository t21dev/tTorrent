import { useEffect, useRef, useState } from "react";
import {
  RELEASE,
  SITE,
  SPECS,
  SHOTS,
  TOUR,
  FEATURES,
  AGENT_FACTS,
  RUN_30,
  START_2,
  LEFT_OUT,
  mb,
  type Asset,
} from "./content";
import { Mark, GitHub, ArrowRight, Download } from "./components/Icons";
import { useDownload, useSystem } from "./download";

function Nav() {
  const [scrolled, setScrolled] = useState(false);
  useEffect(() => {
    const on = () => setScrolled(window.scrollY > 8);
    on();
    window.addEventListener("scroll", on, { passive: true });
    return () => window.removeEventListener("scroll", on);
  }, []);
  return (
    <header className="nav" data-scrolled={scrolled}>
      <div className="nav-inner">
        <a className="brand" href="#top" aria-label="tTorrent, back to top">
          <Mark /> <span>tTorrent</span>
        </a>
        <nav aria-label="Sections" className="nav-links">
          <a href="#tour">Tour</a>
          <a href="#features">Features</a>
          <a href="#compare">Compared with qBittorrent</a>
          <a href={SITE.releases}>Releases</a>
        </nav>
        <a className="icon-link" href={SITE.repo} aria-label="tTorrent on GitHub">
          <GitHub />
        </a>
        <a className="btn btn-solid btn-sm" href="#download">
          Download
        </a>
      </div>
    </header>
  );
}

function Hero() {
  const dl = useDownload();
  return (
    <section className="hero wrap" id="top" aria-labelledby="hero-title">
      <div className="hero-copy">
        <a className="release-pill" href={RELEASE.page}>
          <span className="dot" aria-hidden="true" />v{RELEASE.version} · Windows 10 and 11 · GPL
        </a>
        <h1 id="hero-title" className="hero-title">
          <span className="hero-name">tTorrent</span>
          <span className="hero-line">A fast, modern torrent client for Windows with a polished look and nothing in the way.</span>
        </h1>
        <p className="hero-sub">
          It stays light with hundreds of torrents, and it handles what usually needs scripts or add-ons: disk space,
          webhooks, unpacking, backups and AI agents.
        </p>
        <div className="cta-row">
          <a className="btn btn-solid" href={dl.href}>
            <Download />
            {dl.label}
            {dl.size && <span className="btn-meta">{dl.size} MB</span>}
          </a>
          <a className="btn btn-ghost" href={SITE.repo}>
            <GitHub /> View on GitHub
          </a>
        </div>
        {dl.other && (
          <p className="cta-note" role="note">
            tTorrent runs on 64-bit Windows 10 and 11 only. There is no {dl.sys?.os === "mac" ? "macOS" : "Linux"} build.
          </p>
        )}
        {!dl.other && dl.href !== "#download" && (
          <p className="cta-note">
            Want the portable zip? <a href="#download">All downloads</a>
          </p>
        )}
      </div>

      <div className="spec-card">
        <table className="specs">
          <caption className="sr-only">tTorrent against qBittorrent 5.3.0</caption>
          <thead>
            <tr>
              <th scope="col">
                <span className="sr-only">Measure</span>
              </th>
              <th scope="col">qBittorrent</th>
              <th scope="col">tTorrent</th>
            </tr>
          </thead>
          <tbody>
            {SPECS.map((s) => (
              <tr key={s.label}>
                <th scope="row">{s.label}</th>
                <td className="mono">
                  {s.qbt} <small>{s.unit}</small>
                </td>
                <td className="mono ours">
                  {s.ours} <small>{s.unit}</small>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <p className="source">
          qBittorrent 5.3.0 against tTorrent, one PC with about 735 torrents, 18 November 2026. Single runs vary by a
          few MB. <a href={SITE.method}>Method</a>
        </p>
      </div>
    </section>
  );
}

function Shot({ name, alt, eager, sizes = "(min-width: 1240px) 1200px, 100vw" }: { name: string; alt: string; eager?: boolean; sizes?: string }) {
  const [w, h] = SHOTS[name];
  return (
    <img
      src={`/screenshots/${name}.webp`}
      srcSet={w > 1000 ? `/screenshots/${name}-1000.webp 1000w, /screenshots/${name}.webp ${w}w` : undefined}
      sizes={w > 1000 ? sizes : undefined}
      width={w}
      height={h}
      alt={alt}
      loading={eager ? "eager" : "lazy"}
      decoding={eager ? "sync" : "async"}
    />
  );
}

function Stage() {
  return (
    <div className="stage wrap">
      <figure className="stage-shot">
        <Shot name="main-dark" eager alt="tTorrent's main window in the Dark theme, with three open-movie torrents seeding" />
      </figure>
      <p className="stage-note">Screenshots use open movies from the Blender Foundation as sample torrents.</p>
    </div>
  );
}

function Principles() {
  return (
    <section className="wrap principles" aria-label="What it is">
      <div className="rise">
        <h2>Built on qBittorrent.</h2>
        <p>
          tTorrent began as a fork of qBittorrent 5.3 and takes in each qBittorrent release. It is now built on 5.3.1,
          with the same core torrent features and Web UI.
        </p>
      </div>
      <div className="rise">
        <h2>Seen as qBittorrent by trackers.</h2>
        <p>
          Trackers see tTorrent as qBittorrent: the peer ID and user agent are qBittorrent's, so a tracker that allows
          qBittorrent accepts tTorrent too. Only people see tTorrent's own version.
        </p>
      </div>
      <div className="rise">
        <h2>Your qBittorrent, brought over.</h2>
        <p>
          Settings › Backup and import › Import from qBittorrent copies torrents with their progress, categories, tags,
          RSS and settings, with no recheck. Otherwise tTorrent never reads or changes qBittorrent's settings.
        </p>
      </div>
    </section>
  );
}

function Tour() {
  const [active, setActive] = useState(TOUR[0].id);
  const refs = useRef<(HTMLElement | null)[]>([]);
  useEffect(() => {
    const io = new IntersectionObserver(
      (entries) => {
        for (const e of entries) if (e.isIntersecting) setActive((e.target as HTMLElement).dataset.id!);
      },
      { rootMargin: "-45% 0px -50% 0px" },
    );
    refs.current.forEach((el) => el && io.observe(el));
    return () => io.disconnect();
  }, []);
  return (
    <section className="tour wrap" id="tour" aria-labelledby="tour-title">
      <div className="section-head rise">
        <h2 id="tour-title">Everyday, without the clicks.</h2>
        <p>The real app, on Windows. Scroll through it.</p>
      </div>
      <div className="tour-grid">
        <div className="tour-steps">
          {TOUR.map((t, i) => (
            <article
              key={t.id}
              data-id={t.id}
              ref={(el) => {
                refs.current[i] = el;
              }}
              className="tour-step"
              data-active={active === t.id}
            >
              <figure className="tour-inline">
                <Shot name={t.shot} alt={t.alt} sizes="100vw" />
              </figure>
              <h3>{t.title}</h3>
              <p>{t.body}</p>
              {t.points.length > 0 && (
              <ul>
                {t.points.map((p) => (
                  <li key={p}>{p}</li>
                ))}
              </ul>
              )}
            </article>
          ))}
        </div>
        <div className="tour-viewer" aria-hidden="true">
          <div className="tour-frame">
            {TOUR.map((t) => {
              const [w, h] = SHOTS[t.shot];
              return (
                <img
                  key={t.id}
                  src={`/screenshots/${t.shot}.webp`}
                  srcSet={w > 1000 ? `/screenshots/${t.shot}-1000.webp 1000w, /screenshots/${t.shot}.webp ${w}w` : undefined}
                  sizes={w > 1000 ? "(min-width: 1240px) 760px, 62vw" : undefined}
                  width={w}
                  height={h}
                  alt=""
                  loading="lazy"
                  decoding="async"
                  data-on={active === t.id}
                />
              );
            })}
          </div>
        </div>
      </div>
    </section>
  );
}

function Features() {
  return (
    <section className="wrap features" id="features" aria-labelledby="features-title">
      <div className="section-head rise">
        <h2 id="features-title">What usually needs a script.</h2>
        <p>Things a torrent client is often asked to do with add-ons, built in.</p>
      </div>
      <div className="feature-grid">
        {FEATURES.map((g) => (
          <div className="feature-col rise" key={g.title}>
            <h3>{g.title}</h3>
            <ul className="facts">
              {g.items.map(([name, body]) => (
                <li key={name}>
                  <b>{name}.</b> {body}
                </li>
              ))}
            </ul>
          </div>
        ))}
        <div className="feature-col agents rise">
          <h3>AI agents</h3>
          <p className="agents-lead">
            A built-in <a href={SITE.mcp}>Model Context Protocol</a> server lets Claude Code, Codex or Cursor see and
            manage your torrents.
          </p>
          <ul className="facts">
            {AGENT_FACTS.map((f) => (
              <li key={f}>{f}</li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}

function Table({ head, rows, caption }: { head: string[]; rows: string[][]; caption: string }) {
  return (
    <div className="table-wrap">
      <table className="data">
        <caption>{caption}</caption>
        <thead>
          <tr>
            {head.map((h, i) => (
              <th key={i} scope="col">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r[0]}>
              <th scope="row">{r[0]}</th>
              {r.slice(1).map((c, i) => (
                <td key={i} className={head.length === 3 ? "mono" : undefined}>
                  {c}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Compare() {
  return (
    <section className="wrap compare" id="compare" aria-labelledby="compare-title">
      <div className="section-head rise">
        <h2 id="compare-title">Compared with qBittorrent.</h2>
        <p>
          Measured on 18 November 2026 on one PC (AMD Ryzen 9 9900X, 62 GB, Windows 11 Pro), using a copy of a real
          profile with about 735 torrents. Each app ran alone, minimized, with downloads limited to 1 KiB/s and the Web
          UI off. Memory is Task Manager's Memory column; private bytes is the memory reserved for the app.
        </p>
      </div>
      <div className="compare-grid">
        <div className="rise">
          <Table
            caption="30 minutes running: one run each, sampled every 15 s, a development build between 0.1.0 and 0.2.0"
            head={["", "qBittorrent 5.3.0", "tTorrent"]}
            rows={RUN_30}
          />
        </div>
        <div className="rise">
          <Table
            caption="First 2 minutes after start: three starts each, sampled every second, tTorrent 0.3.0"
            head={["", "qBittorrent 5.3.0", "tTorrent"]}
            rows={START_2}
          />
        </div>
      </div>
      <ul className="facts short rise">
        <li>Settled, tTorrent uses about 10 MB less memory, about 11 MB less reserved memory and about 30 % less CPU.</li>
        <li>At start-up it reads about a quarter less from disk. Peak memory at start-up is about the same.</li>
        <li>
          These are single-machine measurements, and single runs vary by a few MB. <a href={SITE.method}>Method</a>
        </li>
      </ul>

      <div className="left-out rise">
        <h3>Left out or changed</h3>
        <Table caption="qBittorrent features and what tTorrent does with them" head={["Feature", "In tTorrent"]} rows={LEFT_OUT} />
        <p className="hint">
          These features are switched off in one place, not deleted, so tTorrent stays easy to merge with new
          qBittorrent releases.
        </p>
      </div>
    </section>
  );
}

function DlLink({ a, label }: { a?: Asset; label: string }) {
  if (!a) return null;
  return (
    <a className="dl" href={a.url}>
      <span className="dl-label">{label}</span>
      <span className="dl-meta mono">{mb(a.size)} MB</span>
      <ArrowRight />
    </a>
  );
}

function Downloads() {
  const a = RELEASE.assets;
  const sys = useSystem();
  const other = sys && sys.os !== "windows";
  const date = new Date(RELEASE.date + "T00:00:00Z").toLocaleDateString("en-GB", {
    day: "numeric",
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });
  return (
    <section className="wrap downloads" id="download" aria-labelledby="dl-title">
      <div className="section-head rise">
        <h2 id="dl-title">Download tTorrent.</h2>
        <p>
          Version {RELEASE.version}, released {date}. Free and open source under the GPL.{" "}
          <a href={RELEASE.page}>Release notes</a>
        </p>
      </div>
      <div className="dl-grid">
        <div className="dl-col rise" {...(sys?.os === "windows" ? { "data-yours": "" } : {})}>
          <h3>
            Windows
            {sys?.os === "windows" && <span className="yours-tag">Your system</span>}
          </h3>
          <DlLink a={a.windows} label="Installer (.exe)" />
          <DlLink a={a.windowsPortable} label="Portable (.zip)" />
          <p className="dl-note">
            Needs 64-bit Windows 10 or 11. For the portable copy, unzip anywhere and run <code>ttorrent.exe</code>.
          </p>
          <p className="dl-note">
            The installer isn't code-signed yet, so Windows SmartScreen may say “Windows protected your PC”. Click More
            info › Run anyway.
          </p>
        </div>
        <div className="dl-col dl-other rise" {...(other ? { "data-yours": "" } : {})}>
          <h3>
            macOS and Linux
            {other && <span className="yours-tag">Your system</span>}
          </h3>
          <p>There is no tTorrent for macOS or Linux. It is built for Windows only.</p>
        </div>
      </div>
      {a.checksums && (
        <p className="checksums">
          Compare what you downloaded with the SHA-256 in <a href={a.checksums.url}>{a.checksums.name}</a>.
        </p>
      )}
      <p className="checksums">
        Uninstalling keeps your torrents and settings (in <code>%APPDATA%\tTorrent</code> and{" "}
        <code>%LOCALAPPDATA%\tTorrent</code>). Something not working? <a href={SITE.troubleshooting}>Troubleshooting</a>
      </p>
    </section>
  );
}

function Footer() {
  return (
    <footer className="footer">
      <div className="wrap footer-inner">
        <p className="footer-statement">
          <Mark size={32} /> tTorrent
        </p>
        <p className="footer-by">
          By <a href="https://github.com/t21dev">t21 dev</a>. tTorrent began as a fork of{" "}
          <a href={SITE.upstream}>qBittorrent</a>. Thanks to the qBittorrent project and its contributors, whose work it
          started from, and to <a href={SITE.libtorrent}>libtorrent</a>.
        </p>
        <p className="footer-by">
          tTorrent is GPL: source files are GPLv2 or later, binaries GPLv3 or later. Its custom title bar uses QWindowKit
          (Apache-2.0), QR codes use QR Code generator by Project Nayuki (MIT), the fonts are Instrument Sans and DM Mono
          (OFL), and the Nord theme's colours come from Nord (MIT).
        </p>
        <nav className="footer-links" aria-label="Footer">
          <a href={SITE.repo}>GitHub</a>
          <a href={SITE.releases}>Releases</a>
          <a href={SITE.license}>GPL licence</a>
          <a href={SITE.upstream}>qBittorrent</a>
          <a href="https://t21.dev">t21.dev</a>
        </nav>
      </div>
    </footer>
  );
}

export default function App() {
  return (
    <>
      <a className="skip" href="#main">
        Skip to content
      </a>
      <Nav />
      <main id="main">
        <Hero />
        <Stage />
        <Principles />
        <Tour />
        <Features />
        <Compare />
        <Downloads />
      </main>
      <Footer />
    </>
  );
}
