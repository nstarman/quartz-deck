//! Pandoc attribute blocks: `{#id .class key=value key="quoted value"}`.
//! An unmodified block prints back byte-for-byte (constitution II).

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Id(String),
    Class(String),
    Kv(String, String),
    /// Tokens we don't interpret (e.g. `-` for `.unnumbered`), kept verbatim.
    Other(String),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs {
    pub items: Vec<Item>,
    orig: Option<String>,
}

impl Attrs {
    /// Parse `{...}` (braces included). `None` if it is not a valid attribute block.
    pub fn parse(s: &str) -> Option<Attrs> {
        let inner = s.strip_prefix('{')?.strip_suffix('}')?;
        let mut items = Vec::new();
        let mut chars = inner.chars().peekable();
        loop {
            while chars.next_if(|c| c.is_whitespace()).is_some() {}
            if chars.peek().is_none() {
                break;
            }
            let mut tok = String::new();
            let mut value: Option<String> = None;
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                chars.next();
                if c == '=' && value.is_none() && !tok.starts_with(['#', '.']) {
                    let mut v = String::new();
                    match chars.peek() {
                        Some(&q @ ('"' | '\'')) => {
                            chars.next();
                            let mut closed = false;
                            while let Some(c) = chars.next() {
                                if c == '\\' && q == '"' {
                                    v.push(chars.next()?);
                                } else if c == q {
                                    closed = true;
                                    break;
                                } else {
                                    v.push(c);
                                }
                            }
                            if !closed {
                                return None;
                            }
                        }
                        _ => {
                            while let Some(c) = chars.next_if(|c| !c.is_whitespace()) {
                                v.push(c);
                            }
                        }
                    }
                    value = Some(v);
                    break;
                }
                if c == '{' || c == '}' {
                    return None;
                }
                tok.push(c);
            }
            items.push(match value {
                Some(v) if !tok.is_empty() => Item::Kv(tok, v),
                Some(_) => return None,
                None if tok.len() > 1 && tok.starts_with('#') => Item::Id(tok[1..].into()),
                None if tok.len() > 1 && tok.starts_with('.') => Item::Class(tok[1..].into()),
                None => Item::Other(tok),
            });
        }
        Some(Attrs {
            items,
            orig: Some(s.to_string()),
        })
    }

    pub fn id(&self) -> Option<&str> {
        self.items.iter().find_map(|i| match i {
            Item::Id(id) => Some(id.as_str()),
            _ => None,
        })
    }

    pub fn classes(&self) -> Vec<&str> {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Class(c) => Some(c.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn has_class(&self, c: &str) -> bool {
        self.classes().contains(&c)
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.kv()
            .into_iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v)
    }

    pub fn kv(&self) -> Vec<(&str, &str)> {
        self.items
            .iter()
            .filter_map(|i| match i {
                Item::Kv(k, v) => Some((k.as_str(), v.as_str())),
                _ => None,
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    fn touch(&mut self) {
        self.orig = None;
    }

    pub fn set_id(&mut self, id: &str) {
        self.touch();
        match self.items.iter_mut().find(|i| matches!(i, Item::Id(_))) {
            Some(item) => *item = Item::Id(id.into()),
            None => self.items.insert(0, Item::Id(id.into())),
        }
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.touch();
        match self
            .items
            .iter_mut()
            .find(|i| matches!(i, Item::Kv(k, _) if k == key))
        {
            Some(item) => *item = Item::Kv(key.into(), value.into()),
            None => self.items.push(Item::Kv(key.into(), value.into())),
        }
    }

    pub fn unset(&mut self, key: &str) -> bool {
        let n = self.items.len();
        self.items
            .retain(|i| !matches!(i, Item::Kv(k, _) if k == key));
        let changed = self.items.len() != n;
        if changed {
            self.touch();
        }
        changed
    }

    pub fn add_class(&mut self, class: &str) {
        if !self.has_class(class) {
            self.touch();
            let pos = self
                .items
                .iter()
                .take_while(|i| matches!(i, Item::Id(_) | Item::Class(_)))
                .count();
            self.items.insert(pos, Item::Class(class.into()));
        }
    }

    pub fn remove_class(&mut self, class: &str) -> bool {
        let n = self.items.len();
        self.items
            .retain(|i| !matches!(i, Item::Class(c) if c == class));
        let changed = self.items.len() != n;
        if changed {
            self.touch();
        }
        changed
    }

    /// `{...}` text: the original when untouched, else a fresh rendering.
    pub fn render(&self) -> String {
        if let Some(o) = &self.orig {
            return o.clone();
        }
        let parts: Vec<String> = self
            .items
            .iter()
            .map(|i| match i {
                Item::Id(id) => format!("#{id}"),
                Item::Class(c) => format!(".{c}"),
                Item::Kv(k, v) => format!("{k}={}", quote(v)),
                Item::Other(o) => o.clone(),
            })
            .collect();
        format!("{{{}}}", parts.join(" "))
    }
}

fn quote(v: &str) -> String {
    let bare = !v.is_empty()
        && v.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_.%-".contains(c));
    if bare {
        v.to_string()
    } else {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

/// Valid attribute key or class/id name for user input.
pub fn valid_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || "_-:.".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_all_item_kinds() {
        let a = Attrs::parse(
            r##"{#intro .smaller .center background-color="#283b95" top=100 x='a b' -}"##,
        )
        .unwrap();
        assert_eq!(a.id(), Some("intro"));
        assert_eq!(a.classes(), vec!["smaller", "center"]);
        assert_eq!(a.get("background-color"), Some("#283b95"));
        assert_eq!(a.get("top"), Some("100"));
        assert_eq!(a.get("x"), Some("a b"));
        assert!(a.items.contains(&Item::Other("-".into())));
    }

    #[test]
    fn escaped_quotes() {
        let a = Attrs::parse(r#"{title="say \"hi\""}"#).unwrap();
        assert_eq!(a.get("title"), Some(r#"say "hi""#));
    }

    #[test]
    fn rejects_invalid() {
        assert!(Attrs::parse("no braces").is_none());
        assert!(Attrs::parse(r#"{k="unterminated}"#).is_none());
        assert!(Attrs::parse("{a {b}").is_none());
        assert!(Attrs::parse("{=v}").is_none());
    }

    #[test]
    fn untouched_round_trips_exactly() {
        for s in [
            r#"{#a   .b  k="v"}"#,
            "{.absolute top=200 left=0 width=\"350\"}",
            "{}",
            "{ #x }",
        ] {
            assert_eq!(Attrs::parse(s).unwrap().render(), s);
        }
    }

    #[test]
    fn edits_preserve_order_of_untouched_items() {
        let mut a = Attrs::parse("{.c1 k1=v1 k2=v2}").unwrap();
        a.set("k1", "new value");
        assert_eq!(a.render(), r#"{.c1 k1="new value" k2=v2}"#);
        a.set_id("s-1");
        a.add_class("c2");
        assert_eq!(a.render(), r#"{#s-1 .c1 .c2 k1="new value" k2=v2}"#);
        assert!(a.unset("k2"));
        assert!(!a.unset("missing"));
        assert!(a.remove_class("c1"));
        assert_eq!(a.render(), r#"{#s-1 .c2 k1="new value"}"#);
        a.set("color", "#fff");
        assert_eq!(a.get("color"), Some("#fff"));
        assert_eq!(
            Attrs::parse(&a.render()).unwrap().get("color"),
            Some("#fff")
        );
    }
}
