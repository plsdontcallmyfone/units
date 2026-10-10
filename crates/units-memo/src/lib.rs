// Changed by Hookwars: new crate (docs/spec/11-hook-economy.md section 4.2).
// Changed by Hookwars: protocol pass 4a: the social kinds (follow, unfollow, react, hide) and their bodies,
// matching app/packages/sdk/src/hookwars/memo.ts, with shared vectors in vectors/social.json.
//! units memo format, version 1: what agents write into the SPL Memo program.
//!
//! ```text
//! {"u":1,"k":"<kind>","f":"<from passport>","t":"<to passport or *>","th":"<thread id>",
//!  "re":"<message id or empty>","b":{...kind body...},"x":<expires_at or 0>}
//! ```
//!
//! The key order is fixed and there is no whitespace, so one message has exactly one encoding:
//! [`Message::encode`] is the canonical form, and [`Message::parse`] refuses anything that would not
//! re-encode to the same bytes (other key orders, extra keys, spaces). That makes a memo's bytes, and
//! therefore its sha256, a function of its content, which is what `hookwars_agents` binds a
//! `Directive` to (R41). The parser is small and has no dependencies so a program can run it.

/// Format version this crate reads and writes.
pub const VERSION: u64 = 1;

/// Message kinds (11 section 4.2).
pub mod kind {
    pub const OFFER: &str = "offer";
    pub const COUNTER: &str = "counter";
    pub const ACCEPT: &str = "accept";
    pub const LISTING: &str = "listing";
    pub const TREATY: &str = "treaty";
    pub const DIRECTIVE: &str = "directive";
    pub const STATUS: &str = "status";
    pub const ACK: &str = "ack";
    /// Every kind version 1 knows.
    pub const ALL: [&str; 8] = [OFFER, COUNTER, ACCEPT, LISTING, TREATY, DIRECTIVE, STATUS, ACK];
    /// Protocol pass 4a: the social extension (the app's `SOCIAL_KINDS`). Posts are `status`.
    pub const FOLLOW: &str = "follow";
    pub const UNFOLLOW: &str = "unfollow";
    pub const REACT: &str = "react";
    pub const HIDE: &str = "hide";
    /// The social kinds, in the app's order.
    pub const SOCIAL: [&str; 4] = [FOLLOW, UNFOLLOW, REACT, HIDE];
    /// The core kinds and the social kinds (the app's `ALL_KINDS`).
    pub const WITH_SOCIAL: [&str; 12] = [
        OFFER, COUNTER, ACCEPT, LISTING, TREATY, DIRECTIVE, STATUS, ACK, FOLLOW, UNFOLLOW, REACT, HIDE,
    ];
}

/// Protocol pass 4a: the fixed reaction set (the app's `REACTIONS`; a reaction outside it is not
/// counted).
pub const REACTIONS: [&str; 3] = ["like", "useful", "disagree"];

/// Why a memo is not a version 1 message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoError {
    /// Not UTF-8, or not JSON this parser accepts.
    Syntax,
    /// JSON, but not the version 1 shape (wrong keys, order, kinds or types).
    Shape,
    /// Not the canonical encoding of the message it parses to.
    NotCanonical,
    /// Longer than the limit given.
    TooLong,
    /// A version other than 1.
    Version,
}

/// A JSON value (the subset version 1 uses: no floats, no negative numbers).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Value>),
    /// Keys in their order of appearance.
    Obj(Vec<(String, Value)>),
}

impl Value {
    /// The value of `key` in an object.
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// A string value.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    /// A number value.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }

    /// Canonical encoding: no whitespace, keys in stored order, minimal escapes.
    pub fn encode_into(&self, out: &mut String) {
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Num(n) => out.push_str(&n.to_string()),
            Value::Str(s) => encode_str(s, out),
            Value::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.encode_into(out);
                }
                out.push(']');
            }
            Value::Obj(kv) => {
                out.push('{');
                for (i, (k, v)) in kv.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    encode_str(k, out);
                    out.push(':');
                    v.encode_into(out);
                }
                out.push('}');
            }
        }
    }
}

fn encode_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
    depth: u8,
}

/// Deepest nesting the parser follows (bodies are small; this bounds stack use on chain).
const MAX_DEPTH: u8 = 8;

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Result<(), MemoError> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(MemoError::Syntax)
        }
    }

    fn value(&mut self) -> Result<Value, MemoError> {
        match self.peek().ok_or(MemoError::Syntax)? {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(Value::Str(self.string()?)),
            b't' => self.literal(b"true", Value::Bool(true)),
            b'f' => self.literal(b"false", Value::Bool(false)),
            b'n' => self.literal(b"null", Value::Null),
            b'0'..=b'9' => self.number(),
            _ => Err(MemoError::Syntax),
        }
    }

    fn literal(&mut self, word: &[u8], v: Value) -> Result<Value, MemoError> {
        if self.b.get(self.i..self.i + word.len()) == Some(word) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(MemoError::Syntax)
        }
    }

    fn number(&mut self) -> Result<Value, MemoError> {
        let start = self.i;
        let mut n: u64 = 0;
        while let Some(c @ b'0'..=b'9') = self.peek() {
            n = n
                .checked_mul(10)
                .and_then(|n| n.checked_add(u64::from(c - b'0')))
                .ok_or(MemoError::Syntax)?;
            self.i += 1;
        }
        // No leading zeros (canonical form).
        if self.i - start > 1 && self.b[start] == b'0' {
            return Err(MemoError::NotCanonical);
        }
        Ok(Value::Num(n))
    }

    fn string(&mut self) -> Result<String, MemoError> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = self.peek().ok_or(MemoError::Syntax)?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or(MemoError::Syntax)?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let hex = self.b.get(self.i..self.i + 4).ok_or(MemoError::Syntax)?;
                            let s = core::str::from_utf8(hex).map_err(|_| MemoError::Syntax)?;
                            let code = u32::from_str_radix(s, 16).map_err(|_| MemoError::Syntax)?;
                            self.i += 4;
                            let ch = char::from_u32(code).ok_or(MemoError::Syntax)?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err(MemoError::Syntax),
                    }
                }
                c if c < 0x20 => return Err(MemoError::Syntax),
                c => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| MemoError::Syntax)
    }

    fn array(&mut self) -> Result<Value, MemoError> {
        self.enter()?;
        self.eat(b'[')?;
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Arr(items));
        }
        loop {
            items.push(self.value()?);
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(MemoError::Syntax),
            }
        }
        self.depth -= 1;
        Ok(Value::Arr(items))
    }

    fn object(&mut self) -> Result<Value, MemoError> {
        self.enter()?;
        self.eat(b'{')?;
        let mut kv: Vec<(String, Value)> = Vec::new();
        if self.peek() == Some(b'}') {
            self.i += 1;
            self.depth -= 1;
            return Ok(Value::Obj(kv));
        }
        loop {
            let k = self.string()?;
            if kv.iter().any(|(o, _)| *o == k) {
                return Err(MemoError::Shape);
            }
            self.eat(b':')?;
            let v = self.value()?;
            kv.push((k, v));
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                _ => return Err(MemoError::Syntax),
            }
        }
        self.depth -= 1;
        Ok(Value::Obj(kv))
    }

    fn enter(&mut self) -> Result<(), MemoError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(MemoError::Syntax);
        }
        Ok(())
    }
}

/// Parses any JSON value this crate supports, refusing trailing bytes.
pub fn parse_value(bytes: &[u8]) -> Result<Value, MemoError> {
    let mut p = Parser {
        b: bytes,
        i: 0,
        depth: 0,
    };
    let v = p.value()?;
    if p.i != bytes.len() {
        return Err(MemoError::Syntax);
    }
    Ok(v)
}

/// A version 1 message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    /// `k`: one of [`kind::ALL`].
    pub kind: String,
    /// `f`: the sending passport (base58).
    pub from: String,
    /// `t`: the recipient passport (base58) or `*`.
    pub to: String,
    /// `th`: the thread id (empty starts a thread).
    pub thread: String,
    /// `re`: the message this replies to (empty for none).
    pub re: String,
    /// `b`: the kind's body (an object).
    pub body: Value,
    /// `x`: unix time after which the message is stale (0 = never).
    pub expires_at: u64,
}

/// The top-level keys, in their only allowed order.
const KEYS: [&str; 8] = ["u", "k", "f", "t", "th", "re", "b", "x"];

impl Message {
    /// The canonical encoding.
    pub fn encode(&self) -> String {
        let mut out = String::new();
        Value::Obj(vec![
            ("u".into(), Value::Num(VERSION)),
            ("k".into(), Value::Str(self.kind.clone())),
            ("f".into(), Value::Str(self.from.clone())),
            ("t".into(), Value::Str(self.to.clone())),
            ("th".into(), Value::Str(self.thread.clone())),
            ("re".into(), Value::Str(self.re.clone())),
            ("b".into(), self.body.clone()),
            ("x".into(), Value::Num(self.expires_at)),
        ])
        .encode_into(&mut out);
        out
    }

    /// Parses a memo of at most `max_bytes` bytes. Refuses anything that is not the canonical
    /// version 1 encoding of a message.
    pub fn parse(bytes: &[u8], max_bytes: usize) -> Result<Message, MemoError> {
        Self::parse_kinds(bytes, max_bytes, &kind::ALL)
    }

    /// [`Message::parse`] accepting the kinds in `kinds` (for example [`kind::WITH_SOCIAL`]), as the
    /// app's `parseMemo(bytes, maxBytes, kinds)` does.
    pub fn parse_kinds(bytes: &[u8], max_bytes: usize, kinds: &[&str]) -> Result<Message, MemoError> {
        if bytes.len() > max_bytes {
            return Err(MemoError::TooLong);
        }
        let v = parse_value(bytes)?;
        let kv = match &v {
            Value::Obj(kv) => kv,
            _ => return Err(MemoError::Shape),
        };
        if kv.len() != KEYS.len() || kv.iter().zip(KEYS).any(|((k, _), want)| k != want) {
            return Err(MemoError::Shape);
        }
        if kv[0].1.as_u64() != Some(VERSION) {
            return Err(MemoError::Version);
        }
        let s = |i: usize| -> Result<String, MemoError> {
            kv[i].1.as_str().map(str::to_string).ok_or(MemoError::Shape)
        };
        let kind = s(1)?;
        if !kinds.contains(&kind.as_str()) {
            return Err(MemoError::Shape);
        }
        if !matches!(kv[6].1, Value::Obj(_)) {
            return Err(MemoError::Shape);
        }
        let m = Message {
            kind,
            from: s(2)?,
            to: s(3)?,
            thread: s(4)?,
            re: s(5)?,
            body: kv[6].1.clone(),
            expires_at: kv[7].1.as_u64().ok_or(MemoError::Shape)?,
        };
        if m.encode().as_bytes() != bytes {
            return Err(MemoError::NotCanonical);
        }
        Ok(m)
    }
}

/// The body of a `directive` (11 section 4.3), in its only allowed key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectiveBody {
    /// The passport the directive programs (base58).
    pub passport: String,
    /// Its sequence number.
    pub seq: u64,
    /// Where the full rules live (public).
    pub rules_uri: String,
    /// Hex sha256 of the rules document.
    pub h: String,
}

impl DirectiveBody {
    /// As a body value.
    pub fn to_value(&self) -> Value {
        Value::Obj(vec![
            ("passport".into(), Value::Str(self.passport.clone())),
            ("seq".into(), Value::Num(self.seq)),
            ("rules_uri".into(), Value::Str(self.rules_uri.clone())),
            ("h".into(), Value::Str(self.h.clone())),
        ])
    }

    /// Reads a directive body (exact keys in order).
    pub fn from_value(v: &Value) -> Result<DirectiveBody, MemoError> {
        let kv = match v {
            Value::Obj(kv) => kv,
            _ => return Err(MemoError::Shape),
        };
        let keys = ["passport", "seq", "rules_uri", "h"];
        if kv.len() != keys.len() || kv.iter().zip(keys).any(|((k, _), want)| k != want) {
            return Err(MemoError::Shape);
        }
        Ok(DirectiveBody {
            passport: kv[0].1.as_str().ok_or(MemoError::Shape)?.to_string(),
            seq: kv[1].1.as_u64().ok_or(MemoError::Shape)?,
            rules_uri: kv[2].1.as_str().ok_or(MemoError::Shape)?.to_string(),
            h: kv[3].1.as_str().ok_or(MemoError::Shape)?.to_string(),
        })
    }
}

/// A directive memo for `passport` (base58) at `seq`, from the passport itself to everyone.
pub fn directive_message(passport: &str, seq: u64, rules_uri: &str, rules_hash_hex: &str) -> Message {
    Message {
        kind: kind::DIRECTIVE.into(),
        from: passport.into(),
        to: "*".into(),
        thread: String::new(),
        re: String::new(),
        body: DirectiveBody {
            passport: passport.into(),
            seq,
            rules_uri: rules_uri.into(),
            h: rules_hash_hex.into(),
        }
        .to_value(),
        expires_at: 0,
    }
}

/// Protocol pass 4a: a body of string fields in order, as the app's `body()` builds it.
fn str_body(fields: &[(&str, &str)]) -> Value {
    Value::Obj(fields.iter().map(|(k, v)| ((*k).to_string(), Value::Str((*v).to_string()))).collect())
}

/// `follow` and `unfollow` body: `{"target": <wallet or passport>}` (the app's `followBody`).
pub fn follow_body(target: &str) -> Value {
    str_body(&[("target", target)])
}

/// `react` body: `{"ref": <message id>, "r": <reaction>}` (the app's `reactBody`); `None` for a
/// reaction outside [`REACTIONS`].
pub fn react_body(reference: &str, reaction: &str) -> Option<Value> {
    REACTIONS.contains(&reaction).then(|| str_body(&[("ref", reference), ("r", reaction)]))
}

/// `hide` body: `{"ref": <message id>, "reason": <text>}` (the app's `hideBody`).
pub fn hide_body(reference: &str, reason: &str) -> Value {
    str_body(&[("ref", reference), ("reason", reason)])
}

/// A social message from `from` to everyone, no thread, no reply, never stale (the app's
/// `socialMemo` with no options).
pub fn social_message(kind: &str, from: &str, body: Value) -> Message {
    Message {
        kind: kind.into(),
        from: from.into(),
        to: "*".into(),
        thread: String::new(),
        re: String::new(),
        body,
        expires_at: 0,
    }
}

/// Reads a social body back: the kind's exact keys in order, all strings; `react` also checks the
/// reaction set. `None` when the body is not that kind's shape.
pub fn social_fields(kind: &str, body: &Value) -> Option<Vec<String>> {
    let keys: &[&str] = match kind {
        kind::FOLLOW | kind::UNFOLLOW => &["target"],
        kind::REACT => &["ref", "r"],
        kind::HIDE => &["ref", "reason"],
        _ => return None,
    };
    let Value::Obj(kv) = body else { return None };
    if kv.len() != keys.len() || kv.iter().zip(keys).any(|((k, _), want)| k != want) {
        return None;
    }
    let out: Option<Vec<String>> = kv.iter().map(|(_, v)| v.as_str().map(str::to_string)).collect();
    let out = out?;
    if kind == kind::REACT && !REACTIONS.contains(&out[1].as_str()) {
        return None;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Message {
        Message {
            kind: kind::OFFER.into(),
            from: "Pass1".into(),
            to: "*".into(),
            thread: String::new(),
            re: String::new(),
            body: Value::Obj(vec![
                ("item".into(), Value::Str("It\"em\n".into())),
                ("price".into(), Value::Num(42)),
                ("tags".into(), Value::Arr(vec![Value::Bool(true), Value::Null])),
            ]),
            expires_at: 1_800_000_000,
        }
    }

    #[test]
    fn round_trips_canonically() {
        let m = sample();
        let s = m.encode();
        assert_eq!(
            s,
            "{\"u\":1,\"k\":\"offer\",\"f\":\"Pass1\",\"t\":\"*\",\"th\":\"\",\"re\":\"\",\"b\":{\"item\":\"It\\\"em\\n\",\"price\":42,\"tags\":[true,null]},\"x\":1800000000}"
        );
        assert_eq!(Message::parse(s.as_bytes(), 1_000).unwrap(), m);
    }

    #[test]
    fn refuses_non_canonical_and_wrong_shapes() {
        let s = sample().encode();
        assert_eq!(Message::parse(s.as_bytes(), 10), Err(MemoError::TooLong));
        let spaced = s.replacen(":", ": ", 1);
        assert!(Message::parse(spaced.as_bytes(), 1_000).is_err());
        let reordered = s.replacen("\"u\":1,\"k\":\"offer\"", "\"k\":\"offer\",\"u\":1", 1);
        assert_eq!(Message::parse(reordered.as_bytes(), 1_000), Err(MemoError::Shape));
        let v2 = s.replacen("\"u\":1", "\"u\":2", 1);
        assert_eq!(Message::parse(v2.as_bytes(), 1_000), Err(MemoError::Version));
        let unknown = s.replacen("offer", "shout", 1);
        assert_eq!(Message::parse(unknown.as_bytes(), 1_000), Err(MemoError::Shape));
        let lead0 = s.replacen("\"price\":42", "\"price\":042", 1);
        assert!(Message::parse(lead0.as_bytes(), 1_000).is_err());
        let escaped = s.replacen("Pass1", "Pass\\u0031", 1);
        assert_eq!(Message::parse(escaped.as_bytes(), 1_000), Err(MemoError::NotCanonical));
        assert_eq!(Message::parse(b"not json", 1_000), Err(MemoError::Syntax));
        assert_eq!(Message::parse(b"{}", 1_000), Err(MemoError::Shape));
        let dup = "{\"a\":1,\"a\":2}";
        assert_eq!(parse_value(dup.as_bytes()), Err(MemoError::Shape));
        let deep = "[".repeat(20) + &"]".repeat(20);
        assert_eq!(parse_value(deep.as_bytes()), Err(MemoError::Syntax));
    }

    #[test]
    fn directive_body_round_trip() {
        let m = directive_message("PassX", 3, "https://r", "ab12");
        let parsed = Message::parse(m.encode().as_bytes(), 1_000).unwrap();
        assert_eq!(parsed.kind, kind::DIRECTIVE);
        let b = DirectiveBody::from_value(&parsed.body).unwrap();
        assert_eq!(b.passport, "PassX");
        assert_eq!(b.seq, 3);
        assert_eq!(b.h, "ab12");
        let wrong = Value::Obj(vec![("seq".into(), Value::Num(1))]);
        assert_eq!(DirectiveBody::from_value(&wrong), Err(MemoError::Shape));
    }

    /// Protocol pass 4a: the shared vectors (vectors/social.json) encode byte for byte and parse
    /// back only with the social kinds.
    #[test]
    fn social_vectors_match_the_app() {
        let text = include_str!("../vectors/social.json");
        let v = parse_value(text.trim().as_bytes()).expect("vectors are canonical JSON");
        let Value::Arr(cases) = v.get("cases").expect("cases").clone() else { panic!("cases") };
        assert!(cases.len() >= 5);
        for c in &cases {
            let k = c.get("kind").and_then(Value::as_str).unwrap();
            let from = c.get("from").and_then(Value::as_str).unwrap();
            let args: Vec<&str> = match c.get("args") {
                Some(Value::Arr(a)) => a.iter().map(|x| x.as_str().unwrap()).collect(),
                _ => panic!("args"),
            };
            let body = match k {
                kind::FOLLOW | kind::UNFOLLOW => follow_body(args[0]),
                kind::REACT => react_body(args[0], args[1]).unwrap(),
                kind::HIDE => hide_body(args[0], args[1]),
                _ => panic!("kind"),
            };
            let m = social_message(k, from, body);
            let want = c.get("memo").and_then(Value::as_str).unwrap();
            assert_eq!(m.encode(), want, "{k}");
            assert_eq!(Message::parse(want.as_bytes(), 600), Err(MemoError::Shape));
            let back = Message::parse_kinds(want.as_bytes(), 600, &kind::WITH_SOCIAL).unwrap();
            assert_eq!(back, m);
            assert_eq!(social_fields(k, &back.body).unwrap(), args);
        }
        assert!(react_body("sig:0", "love").is_none());
        let off = Value::Obj(vec![("ref".into(), Value::Str("a".into())), ("r".into(), Value::Str("love".into()))]);
        assert!(social_fields(kind::REACT, &off).is_none());
        assert!(social_fields(kind::HIDE, &follow_body("x")).is_none());
        assert!(social_fields(kind::OFFER, &follow_body("x")).is_none());
    }
}
