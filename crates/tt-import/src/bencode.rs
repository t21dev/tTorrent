//! A small bencode reader: enough for .torrent and .fastresume files. It keeps
//! each value's raw bytes so an info dict can be copied exactly.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Int(i64, &'a [u8]),
    Bytes(&'a [u8], &'a [u8]),
    List(Vec<Value<'a>>, &'a [u8]),
    Dict(BTreeMap<&'a [u8], Value<'a>>, &'a [u8]),
}

impl<'a> Value<'a> {
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        match self {
            Value::Dict(m, _) => m.get(key.as_bytes()),
            _ => None,
        }
    }
    pub fn int(&self) -> Option<i64> {
        match self {
            Value::Int(i, _) => Some(*i),
            _ => None,
        }
    }
    pub fn bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::Bytes(b, _) => Some(b),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&'a str> {
        self.bytes().and_then(|b| std::str::from_utf8(b).ok())
    }
    pub fn list(&self) -> Option<&[Value<'a>]> {
        match self {
            Value::List(l, _) => Some(l),
            _ => None,
        }
    }
    /// The exact encoded bytes of this value.
    pub fn raw(&self) -> Option<&'a [u8]> {
        Some(match self {
            Value::Int(_, r) | Value::Bytes(_, r) | Value::List(_, r) | Value::Dict(_, r) => r,
        })
    }
}

const MAX_DEPTH: usize = 64;

pub fn parse(input: &[u8]) -> Option<Value<'_>> {
    let (v, rest) = value(input, 0)?;
    rest.is_empty().then_some(v)
}

fn value(s: &[u8], depth: usize) -> Option<(Value<'_>, &[u8])> {
    if depth > MAX_DEPTH {
        return None;
    }
    match *s.first()? {
        b'i' => {
            let end = s.iter().position(|&c| c == b'e')?;
            let n: i64 = std::str::from_utf8(&s[1..end]).ok()?.parse().ok()?;
            Some((Value::Int(n, &s[..=end]), &s[end + 1..]))
        }
        b'l' => {
            let mut rest = &s[1..];
            let mut items = Vec::new();
            while *rest.first()? != b'e' {
                let (v, r) = value(rest, depth + 1)?;
                items.push(v);
                rest = r;
            }
            let used = s.len() - rest.len() + 1;
            Some((Value::List(items, &s[..used]), &rest[1..]))
        }
        b'd' => {
            let mut rest = &s[1..];
            let mut map = BTreeMap::new();
            while *rest.first()? != b'e' {
                let (k, r) = value(rest, depth + 1)?;
                let Value::Bytes(k, _) = k else { return None };
                let (v, r) = value(r, depth + 1)?;
                map.insert(k, v);
                rest = r;
            }
            let used = s.len() - rest.len() + 1;
            Some((Value::Dict(map, &s[..used]), &rest[1..]))
        }
        b'0'..=b'9' => {
            let colon = s.iter().position(|&c| c == b':')?;
            let n: usize = std::str::from_utf8(&s[..colon]).ok()?.parse().ok()?;
            let start = colon + 1;
            let end = start.checked_add(n)?;
            if end > s.len() {
                return None;
            }
            Some((Value::Bytes(&s[start..end], &s[..end]), &s[end..]))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested() {
        let v = parse(b"d1:ai42e1:bl3:xyzi-1eee").unwrap();
        assert_eq!(v.get("a").and_then(Value::int), Some(42));
        let l = v.get("b").and_then(Value::list).unwrap();
        assert_eq!(l[0].str(), Some("xyz"));
        assert_eq!(l[1].int(), Some(-1));
        assert_eq!(v.get("b").and_then(Value::raw), Some(&b"l3:xyzi-1ee"[..]));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"").is_none());
        assert!(parse(b"5:abc").is_none());
        assert!(parse(b"i1ex").is_none());
        assert!(parse(&[b'l'; 200]).is_none());
    }

    proptest::proptest! {
        #[test]
        fn never_panics(bytes in proptest::collection::vec(proptest::num::u8::ANY, 0..300)) {
            let _ = parse(&bytes);
        }
    }
}
