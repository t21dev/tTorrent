//! librqbit's session file. Before the session starts, every torrent in it is
//! marked paused so none starts on its own: the engine starts them in waves
//! (docs/startup.md §2). What the user wants running lives in SQLite.

use anyhow::{Context, Result};
use std::path::Path;

pub fn mark_all_paused(session_dir: &Path) -> Result<usize> {
    let file = session_dir.join("session.json");
    let text = match std::fs::read_to_string(&file) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e).context("reading session.json"),
    };
    let mut v: serde_json::Value = serde_json::from_str(&text).context("session.json")?;
    let mut n = 0;
    if let Some(map) = v.get_mut("torrents").and_then(|t| t.as_object_mut()) {
        for t in map.values_mut() {
            if let Some(obj) = t.as_object_mut()
                && obj.get("is_paused") != Some(&serde_json::Value::Bool(true))
            {
                obj.insert("is_paused".into(), serde_json::Value::Bool(true));
                n += 1;
            }
        }
    }
    if n > 0 {
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec(&v)?)?;
        std::fs::rename(&tmp, &file)?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pauses_everything_once() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("session.json"),
            r#"{"torrents":{"0":{"info_hash":"aa","is_paused":false},"1":{"info_hash":"bb","is_paused":true}}}"#,
        )
        .unwrap();
        assert_eq!(mark_all_paused(d.path()).unwrap(), 1);
        assert_eq!(mark_all_paused(d.path()).unwrap(), 0);
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.path().join("session.json")).unwrap())
                .unwrap();
        assert_eq!(v["torrents"]["0"]["is_paused"], true);
        assert_eq!(v["torrents"]["0"]["info_hash"], "aa");
    }

    #[test]
    fn no_file_is_fine() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(mark_all_paused(d.path()).unwrap(), 0);
    }
}
