//! Default app for `.torrent` files and `magnet:` links, and start at sign-in.
//!
//! Each system decides defaults its own way, so the same three calls do
//! different things (details: `docs/associations.md`):
//!
//! | | `register` | `make_default` | `status` |
//! |---|---|---|---|
//! | Windows | ProgIDs, Capabilities and RegisteredApplications in HKCU (no admin) | `register`, then Settings › Default apps opened on tTorrent's page; Windows only lets the user pick | UserChoice (or the class default when the user never picked) |
//! | macOS | Nothing: the app bundle's Info.plist declares both | NSWorkspace sets both; macOS may ask the user to confirm | NSWorkspace's handler for each |
//! | Linux | `ttorrent.desktop` and its icon in `$XDG_DATA_HOME` | `xdg-mime default` for both types | `xdg-mime query default` (or `mimeapps.list`) |

use std::path::Path;

/// Which of the two tTorrent opens today.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AssocStatus {
    pub torrent_files: bool,
    pub magnet_links: bool,
}

impl AssocStatus {
    pub fn both(self) -> bool {
        self.torrent_files && self.magnet_links
    }
}

/// What `make_default` did, so the window can say what happens next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MakeDefault {
    /// tTorrent is now the default for both.
    Done,
    /// The system's own settings page is open; the user picks tTorrent there.
    SettingsOpened,
    /// The system asked the user to confirm (macOS).
    SystemAsked,
    /// Can't be done from here; the text says why.
    Unsupported(String),
}

/// The desktop-file id on Linux, and the RegisteredApplications name on Windows.
pub const APP_ID: &str = "tTorrent";
#[cfg(windows)]
const PROGID_TORRENT: &str = "tTorrent.File.Torrent";
#[cfg(windows)]
const PROGID_MAGNET: &str = "tTorrent.Url.Magnet";

/// Make tTorrent available for `.torrent` files and magnet links, for this
/// user. On Windows and Linux it also becomes the handler where no other app
/// was chosen; it never overrides a choice the user made.
pub fn register(exe: &Path) -> anyhow::Result<()> {
    imp::register(exe)
}

/// What opens `.torrent` files and magnet links right now.
pub fn status(exe: &Path) -> AssocStatus {
    imp::status(exe)
}

/// Ask for tTorrent to be the default for both, the way the system allows.
pub fn make_default(exe: &Path) -> anyhow::Result<MakeDefault> {
    imp::make_default(exe)
}

/// Start (minimised to the tray) when the user signs in.
pub fn set_autostart(exe: &Path, on: bool) -> anyhow::Result<()> {
    imp::set_autostart(exe, on)
}

// ───────────────────────────── Windows ─────────────────────────────

#[cfg(windows)]
mod imp {
    use super::*;

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let exe = exe.to_string_lossy();
        let open = format!("\"{exe}\" \"%1\"");
        let icon = format!("\"{exe}\",0");
        let caps = format!(r"Software\{APP_ID}\Capabilities");
        let t = format!(r"Software\Classes\{PROGID_TORRENT}");
        let m = format!(r"Software\Classes\{PROGID_MAGNET}");
        let keys: Vec<(String, &str, &str)> = vec![
            // The two ProgIDs, as qBittorrent's installer writes them, per user.
            (t.clone(), "", "Torrent file"),
            (format!(r"{t}\DefaultIcon"), "", &icon),
            (format!(r"{t}\shell\open\command"), "", &open),
            (m.clone(), "", "Magnet link"),
            (m.clone(), "URL Protocol", ""),
            (format!(r"{m}\DefaultIcon"), "", &icon),
            (format!(r"{m}\shell\open\command"), "", &open),
            // Listed under "Open with" for .torrent files.
            (
                r"Software\Classes\.torrent\OpenWithProgids".into(),
                PROGID_TORRENT,
                "",
            ),
            (
                r"Software\Classes\.torrent".into(),
                "Content Type",
                "application/x-bittorrent",
            ),
            (r"Software\Classes\magnet".into(), "URL Protocol", ""),
            // So tTorrent appears in Settings › Default apps.
            (caps.clone(), "ApplicationName", APP_ID),
            (
                caps.clone(),
                "ApplicationDescription",
                "Download and share files over BitTorrent",
            ),
            (
                format!(r"{caps}\FileAssociations"),
                ".torrent",
                PROGID_TORRENT,
            ),
            (format!(r"{caps}\UrlAssociations"), "magnet", PROGID_MAGNET),
            (
                r"Software\RegisteredApplications".into(),
                APP_ID,
                caps.as_str(),
            ),
        ];
        for (key, name, value) in &keys {
            win::set_string(key, name, value)?;
        }
        // Where the user never chose an app, take the class defaults. Windows
        // ignores these once a UserChoice exists, which is the user's call.
        if win::user_choice(win::TORRENT_CHOICE).is_none()
            && win::get_string(win::Root::User, r"Software\Classes\.torrent", "")
                .is_none_or(|v| v.is_empty())
        {
            win::set_string(r"Software\Classes\.torrent", "", PROGID_TORRENT)?;
        }
        if win::user_choice(win::MAGNET_CHOICE).is_none()
            && win::get_string(win::Root::Classes, r"magnet\shell\open\command", "").is_none()
        {
            win::set_string(r"Software\Classes\magnet", "", "URL:Magnet link")?;
            win::set_string(r"Software\Classes\magnet\shell\open\command", "", &open)?;
        }
        win::assoc_changed();
        Ok(())
    }

    pub fn status(exe: &Path) -> AssocStatus {
        let exe = exe.to_string_lossy().to_lowercase();
        let torrent_files = match win::user_choice(win::TORRENT_CHOICE) {
            Some(p) => p.eq_ignore_ascii_case(PROGID_TORRENT),
            None => win::get_string(win::Root::Classes, ".torrent", "")
                .is_some_and(|p| p.eq_ignore_ascii_case(PROGID_TORRENT)),
        };
        let magnet_links = match win::user_choice(win::MAGNET_CHOICE) {
            Some(p) => p.eq_ignore_ascii_case(PROGID_MAGNET),
            None => win::get_string(win::Root::Classes, r"magnet\shell\open\command", "")
                .is_some_and(|c| c.to_lowercase().contains(&exe)),
        };
        AssocStatus {
            torrent_files,
            magnet_links,
        }
    }

    pub fn make_default(exe: &Path) -> anyhow::Result<MakeDefault> {
        register(exe)?;
        if status(exe).both() {
            return Ok(MakeDefault::Done);
        }
        // Windows 10 and 11 let only the user change a default. Open
        // Settings › Default apps on tTorrent's own page (Windows 11 from the
        // April 2023 update; older builds show the Default apps list).
        let uri = format!("ms-settings:defaultapps?registeredAppUser={APP_ID}");
        std::process::Command::new("explorer").arg(uri).spawn()?;
        Ok(MakeDefault::SettingsOpened)
    }

    pub fn set_autostart(exe: &Path, on: bool) -> anyhow::Result<()> {
        let key = r"Software\Microsoft\Windows\CurrentVersion\Run";
        if on {
            win::set_string(
                key,
                APP_ID,
                &format!("\"{}\" --minimized", exe.to_string_lossy()),
            )
        } else {
            win::delete_value(key, APP_ID)
        }
    }

    mod win {
        use windows::Win32::System::Registry::{
            HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, REG_SZ, RRF_RT_REG_SZ, RegDeleteKeyValueW,
            RegGetValueW, RegSetKeyValueW,
        };
        use windows::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};
        use windows::core::PCWSTR;

        /// Windows 11 24H2 writes `UserChoiceLatest`; older builds `UserChoice`.
        pub const TORRENT_CHOICE: &str =
            r"Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\.torrent";
        pub const MAGNET_CHOICE: &str =
            r"Software\Microsoft\Windows\Shell\Associations\UrlAssociations\magnet";

        pub enum Root {
            User,
            /// HKCU and HKLM classes merged, as Explorer sees them.
            Classes,
        }

        fn wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(Some(0)).collect()
        }

        pub fn user_choice(base: &str) -> Option<String> {
            ["UserChoiceLatest", "UserChoice"].iter().find_map(|k| {
                get_string(Root::User, &format!(r"{base}\{k}"), "ProgId").filter(|p| !p.is_empty())
            })
        }

        pub fn get_string(root: Root, key: &str, name: &str) -> Option<String> {
            let hkey: HKEY = match root {
                Root::User => HKEY_CURRENT_USER,
                Root::Classes => HKEY_CLASSES_ROOT,
            };
            let (k, n) = (wide(key), wide(name));
            let name_ptr = if name.is_empty() {
                PCWSTR::null()
            } else {
                PCWSTR(n.as_ptr())
            };
            let mut buf = vec![0u16; 1024];
            let mut bytes = (buf.len() * 2) as u32;
            // SAFETY: NUL-terminated key and name buffers that outlive the
            // call; `buf` is writable for `bytes` bytes, which the call reads
            // and updates to the length written.
            let r = unsafe {
                RegGetValueW(
                    hkey,
                    PCWSTR(k.as_ptr()),
                    name_ptr,
                    RRF_RT_REG_SZ,
                    None,
                    Some(buf.as_mut_ptr().cast()),
                    Some(&mut bytes),
                )
            };
            if r.is_err() {
                return None;
            }
            let len = (bytes as usize / 2).min(buf.len());
            let s = String::from_utf16_lossy(&buf[..len]);
            Some(s.trim_end_matches('\0').to_owned())
        }

        pub fn set_string(key: &str, name: &str, value: &str) -> anyhow::Result<()> {
            let (k, n, v) = (wide(key), wide(name), wide(value));
            // SAFETY: all three buffers are NUL-terminated UTF-16 that outlive the
            // call; the byte count includes the terminator, as REG_SZ requires.
            let r = unsafe {
                RegSetKeyValueW(
                    HKEY_CURRENT_USER,
                    PCWSTR(k.as_ptr()),
                    if name.is_empty() {
                        PCWSTR::null()
                    } else {
                        PCWSTR(n.as_ptr())
                    },
                    REG_SZ.0,
                    Some(v.as_ptr().cast()),
                    (v.len() * 2) as u32,
                )
            };
            r.ok().map_err(|e| anyhow::anyhow!("registry {key}: {e}"))
        }

        pub fn delete_value(key: &str, name: &str) -> anyhow::Result<()> {
            let (k, n) = (wide(key), wide(name));
            // SAFETY: NUL-terminated buffers that outlive the call.
            let r = unsafe {
                RegDeleteKeyValueW(HKEY_CURRENT_USER, PCWSTR(k.as_ptr()), PCWSTR(n.as_ptr()))
            };
            // Already absent is fine.
            if r.is_ok() || r.0 == 2 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("registry {key}: {}", r.0))
            }
        }

        /// Tell Explorer the associations changed so icons and "Open with" update.
        pub fn assoc_changed() {
            // SAFETY: SHCNE_ASSOCCHANGED takes no items; both pointers are null.
            unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, None, None) };
        }
    }
}

// ───────────────────────────── Linux and other Unix ─────────────────────────────

#[cfg(all(unix, not(target_os = "macos")))]
mod imp {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    const DESKTOP: &str = "ttorrent.desktop";
    const TYPES: [&str; 2] = ["application/x-bittorrent", "x-scheme-handler/magnet"];
    const ICON: &[u8] = include_bytes!("../../../assets/brand/ttorrent-256.png");

    fn home() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_default()
    }
    fn data_home() -> PathBuf {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home().join(".local/share"))
    }
    fn config_home() -> PathBuf {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home().join(".config"))
    }

    /// `Exec=` quoting from the Desktop Entry spec: quote, and escape `"`, `` ` ``, `$`, `\`.
    pub(super) fn exec_quote(path: &str) -> String {
        let mut s = String::from("\"");
        for c in path.chars() {
            if matches!(c, '"' | '`' | '$' | '\\') {
                s.push('\\');
            }
            s.push(c);
        }
        s.push('"');
        s
    }

    pub(super) fn desktop_entry(exe: &Path) -> String {
        format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=tTorrent\n\
             GenericName=BitTorrent client\n\
             Comment=Download and share files over BitTorrent\n\
             Exec={} %U\n\
             Icon=ttorrent\n\
             Terminal=false\n\
             Categories=Network;FileTransfer;P2P;\n\
             MimeType={};\n\
             StartupWMClass=ttorrent-ui\n",
            exec_quote(&exe.to_string_lossy()),
            TYPES.join(";"),
        )
    }

    pub fn register(exe: &Path) -> anyhow::Result<()> {
        let apps = data_home().join("applications");
        std::fs::create_dir_all(&apps)?;
        std::fs::write(apps.join(DESKTOP), desktop_entry(exe))?;
        let icons = data_home().join("icons/hicolor/256x256/apps");
        if std::fs::create_dir_all(&icons).is_ok() {
            let _ = std::fs::write(icons.join("ttorrent.png"), ICON);
        }
        // Refresh the MIME cache where the tool exists; desktops also watch the folder.
        let _ = Command::new("update-desktop-database").arg(&apps).output();
        // Take the types nobody else handles yet.
        for ty in TYPES {
            if query(ty).is_none() {
                set_default(ty)?;
            }
        }
        Ok(())
    }

    fn query(ty: &str) -> Option<String> {
        if let Ok(o) = Command::new("xdg-mime")
            .args(["query", "default", ty])
            .output()
            && o.status.success()
        {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_owned();
            return Some(s).filter(|s| !s.is_empty());
        }
        mimeapps_default(
            &std::fs::read_to_string(config_home().join("mimeapps.list")).ok()?,
            ty,
        )
    }

    fn set_default(ty: &str) -> anyhow::Result<()> {
        // One type per call: some xdg-mime versions mishandle several.
        let ok = Command::new("xdg-mime")
            .args(["default", DESKTOP, ty])
            .status()
            .is_ok_and(|s| s.success());
        if ok {
            return Ok(());
        }
        let path = config_home().join("mimeapps.list");
        let old = std::fs::read_to_string(&path).unwrap_or_default();
        std::fs::create_dir_all(config_home())?;
        std::fs::write(&path, mimeapps_set(&old, ty, DESKTOP))?;
        Ok(())
    }

    pub fn status(_exe: &Path) -> AssocStatus {
        AssocStatus {
            torrent_files: query(TYPES[0]).as_deref() == Some(DESKTOP),
            magnet_links: query(TYPES[1]).as_deref() == Some(DESKTOP),
        }
    }

    pub fn make_default(exe: &Path) -> anyhow::Result<MakeDefault> {
        if std::env::var_os("FLATPAK_ID").is_some() {
            return Ok(MakeDefault::Unsupported(
                "Inside Flatpak, choose tTorrent in your desktop's Default applications settings."
                    .into(),
            ));
        }
        register(exe)?;
        for ty in TYPES {
            set_default(ty)?;
        }
        Ok(if status(exe).both() {
            MakeDefault::Done
        } else {
            MakeDefault::Unsupported(
                "Your desktop kept another app. Choose tTorrent in its Default applications settings."
                    .into(),
            )
        })
    }

    pub fn set_autostart(exe: &Path, on: bool) -> anyhow::Result<()> {
        let dir = config_home().join("autostart");
        let file = dir.join(DESKTOP);
        if on {
            std::fs::create_dir_all(&dir)?;
            let entry = desktop_entry(exe).replace(" %U\n", " --minimized\n");
            std::fs::write(file, entry)?;
        } else if file.exists() {
            std::fs::remove_file(file)?;
        }
        Ok(())
    }
}

/// The `[Default Applications]` entry for `ty` in a `mimeapps.list`.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn mimeapps_default(text: &str, ty: &str) -> Option<String> {
    let mut in_defaults = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_defaults = line == "[Default Applications]";
        } else if in_defaults
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == ty
        {
            return v
                .split(';')
                .map(str::trim)
                .find(|s| !s.is_empty())
                .map(Into::into);
        }
    }
    None
}

/// `text` with `ty=app` set under `[Default Applications]`, everything else kept.
#[cfg_attr(any(windows, target_os = "macos"), allow(dead_code))]
fn mimeapps_set(text: &str, ty: &str, app: &str) -> String {
    let mut out = Vec::new();
    let mut in_defaults = false;
    let mut found_section = false;
    let mut written = false;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            if in_defaults && !written {
                out.push(format!("{ty}={app};"));
                written = true;
            }
            in_defaults = l == "[Default Applications]";
            found_section |= in_defaults;
            out.push(line.to_owned());
            continue;
        }
        if in_defaults && l.split_once('=').is_some_and(|(k, _)| k.trim() == ty) {
            if !written {
                out.push(format!("{ty}={app};"));
                written = true;
            }
            continue;
        }
        out.push(line.to_owned());
    }
    if !written {
        if !found_section {
            out.push("[Default Applications]".into());
        }
        out.push(format!("{ty}={app};"));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

// ───────────────────────────── macOS ─────────────────────────────

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use objc2_app_kit::NSWorkspace;
    use objc2_foundation::{NSBundle, NSString, NSURL};
    use objc2_uniform_type_identifiers::UTType;

    /// The `.torrent` type the bundle's Info.plist imports (docs/associations.md).
    const TORRENT_UTI: &str = "org.bittorrent.torrent";

    fn bundle_url() -> Option<objc2::rc::Retained<NSURL>> {
        let url = NSBundle::mainBundle().bundleURL();
        // A bare binary outside an .app has no Info.plist, so it can't be a handler.
        let path = url.path()?.to_string();
        path.ends_with(".app").then_some(url)
    }

    fn same(a: Option<objc2::rc::Retained<NSURL>>, b: &NSURL) -> bool {
        match (a.and_then(|a| a.path()), b.path()) {
            (Some(a), Some(b)) => a.to_string() == b.to_string(),
            _ => false,
        }
    }

    pub fn register(_exe: &Path) -> anyhow::Result<()> {
        // Launch Services reads CFBundleDocumentTypes and CFBundleURLTypes
        // from the bundle when it is first opened; nothing to write.
        Ok(())
    }

    pub fn status(_exe: &Path) -> AssocStatus {
        let Some(me) = bundle_url() else {
            return AssocStatus::default();
        };
        let ws = NSWorkspace::sharedWorkspace();
        let magnet_links = NSURL::URLWithString(&NSString::from_str("magnet:?xt=urn:btih:0"))
            .is_some_and(|u| same(ws.URLForApplicationToOpenURL(&u), &me));
        let torrent_files = UTType::typeWithIdentifier(&NSString::from_str(TORRENT_UTI))
            .is_some_and(|ty| same(ws.URLForApplicationToOpenContentType(&ty), &me));
        AssocStatus {
            torrent_files,
            magnet_links,
        }
    }

    pub fn make_default(_exe: &Path) -> anyhow::Result<MakeDefault> {
        let Some(me) = bundle_url() else {
            return Ok(MakeDefault::Unsupported(
                "Open tTorrent from its app in Applications to make it the default.".into(),
            ));
        };
        let ws = NSWorkspace::sharedWorkspace();
        // macOS 12+: the system applies these and may ask the user first.
        ws.setDefaultApplicationAtURL_toOpenURLsWithScheme_completionHandler(
            &me,
            &NSString::from_str("magnet"),
            None,
        );
        if let Some(ty) = UTType::typeWithIdentifier(&NSString::from_str(TORRENT_UTI)) {
            ws.setDefaultApplicationAtURL_toOpenContentType_completionHandler(&me, &ty, None);
        }
        Ok(MakeDefault::SystemAsked)
    }

    pub fn set_autostart(_exe: &Path, _on: bool) -> anyhow::Result<()> {
        // SMAppService.mainApp (macOS 13+) needs the signed bundle; Phase 5.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mimeapps_reads_the_default_section_only() {
        let t = "[Added Associations]\napplication/x-bittorrent=other.desktop;\n\
                 [Default Applications]\nx-scheme-handler/magnet=ttorrent.desktop;other.desktop;\n";
        assert_eq!(
            mimeapps_default(t, "x-scheme-handler/magnet").as_deref(),
            Some("ttorrent.desktop")
        );
        assert_eq!(mimeapps_default(t, "application/x-bittorrent"), None);
    }

    #[test]
    fn mimeapps_set_replaces_or_adds() {
        let t = "[Default Applications]\nx-scheme-handler/magnet=old.desktop;\ntext/html=firefox.desktop;\n\
                 [Added Associations]\nfoo=bar;\n";
        let s = mimeapps_set(t, "x-scheme-handler/magnet", "ttorrent.desktop");
        assert_eq!(
            mimeapps_default(&s, "x-scheme-handler/magnet").as_deref(),
            Some("ttorrent.desktop")
        );
        assert!(s.contains("text/html=firefox.desktop;"));
        assert!(s.contains("foo=bar;"));
        assert_eq!(s.matches("x-scheme-handler/magnet=").count(), 1);

        let s = mimeapps_set("", "application/x-bittorrent", "ttorrent.desktop");
        assert_eq!(
            s,
            "[Default Applications]\napplication/x-bittorrent=ttorrent.desktop;\n"
        );
        let s = mimeapps_set(
            "[Added Associations]\na=b;\n",
            "application/x-bittorrent",
            "ttorrent.desktop",
        );
        assert!(
            s.ends_with("[Default Applications]\napplication/x-bittorrent=ttorrent.desktop;\n")
        );
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn desktop_entry_quotes_the_path() {
        let e = imp::desktop_entry(Path::new("/opt/t Torrent/$x/ttorrent"));
        assert!(e.contains("Exec=\"/opt/t Torrent/\\$x/ttorrent\" %U\n"));
        assert!(e.contains("MimeType=application/x-bittorrent;x-scheme-handler/magnet;\n"));
    }
}
