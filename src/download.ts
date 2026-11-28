// The download that fits the visitor's computer. Worked out in the browser after the page loads,
// so the prerendered page (and anyone without JavaScript) keeps the link to #download.
import { useEffect, useState } from "react";
import { RELEASE, mb } from "./content";

export type OS = "windows" | "mac" | "linux";
/** `arm` is null when the browser doesn't say (Safari, Firefox). */
export type System = { os: OS; arm: boolean | null };

type UaData = { getHighEntropyValues?: (hints: string[]) => Promise<{ architecture?: string }> };

async function detect(): Promise<System | null> {
  const ua = navigator.userAgent;
  if (/Android|iPhone|iPad|iPod|Mobile/i.test(ua)) return null;
  const os: OS | null = /Windows/i.test(ua) ? "windows" : /Mac OS X|Macintosh/i.test(ua) ? "mac" : /Linux|X11|CrOS/i.test(ua) ? "linux" : null;
  if (!os) return null;
  // An iPad asking for the desktop site says "Macintosh" but has a touch screen.
  if (os === "mac" && navigator.maxTouchPoints > 1) return null;
  let arm: boolean | null = /aarch64|arm64/i.test(ua) ? true : os === "linux" && /x86_64/i.test(ua) ? false : null;
  const uad = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  if (uad?.getHighEntropyValues) {
    try {
      const v = await uad.getHighEntropyValues(["architecture"]);
      if (v.architecture) arm = v.architecture.toLowerCase().startsWith("arm");
    } catch {
      // Keep what the user agent said.
    }
  }
  return { os, arm };
}

export function useSystem(): System | null {
  const [sys, setSys] = useState<System | null>(null);
  useEffect(() => {
    let live = true;
    detect().then((s) => live && setSys(s));
    return () => {
      live = false;
    };
  }, []);
  return sys;
}

/**
 * The hero's main button. tTorrent is Windows only, so on Windows it goes straight to the installer;
 * on macOS and Linux it says so instead of offering a file that won't run; otherwise it goes to #download.
 */
export function useDownload() {
  const sys = useSystem();
  const file = RELEASE.assets.windows;
  if (sys?.os === "windows" && file) return { href: file.url, label: "Download for Windows", size: mb(file.size), sys, other: false };
  if (sys) return { href: "#download", label: "Download for Windows", size: null, sys, other: true };
  return { href: "#download", label: "Download", size: null, sys, other: false };
}
