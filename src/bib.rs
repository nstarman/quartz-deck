//! `qd bib --zotero KEY` (spec 010 US5): write the References list from a
//! Zotero collection into the deck, between `<!-- bibliography:start … -->`
//! and `<!-- bibliography:end -->`. Port of the quarto-revealjs skill's
//! `bibliography.py`. Reads the running Zotero 7's local API on this machine
//! only; nothing leaves it.

use crate::deck::{Deck, Span};
use crate::error::{QdError, Result};
use crate::ops::Patch;
use serde_json::Value;
use std::collections::HashMap;

const START: &str = "<!-- bibliography:start";
const END: &str = "<!-- bibliography:end -->";

/// Zotero's local API (user 0 is the local user); `QD_ZOTERO_URL` replaces the host in tests.
fn api() -> String {
    std::env::var("QD_ZOTERO_URL").unwrap_or_else(|_| "http://localhost:23119".into())
}

/// The collection's top-level items, without attachments and notes (pages of 100).
pub fn fetch(key: &str) -> Result<Vec<Value>> {
    // it goes into a URL: Zotero keys are 8 letters and digits
    if key.len() != 8 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(QdError::InvalidArg(format!(
            "'{key}' is not a Zotero collection key (8 letters and digits, from the collection's URL)"
        )));
    }
    let mut items = Vec::new();
    for start in (0..).step_by(100) {
        let url = format!(
            "{}/api/users/0/collections/{key}/items/top?format=json&limit=100&start={start}",
            api()
        );
        let fail = |e: &dyn std::fmt::Display| QdError::Zotero(format!("{url}: {e}"));
        let text = ureq::get(&url)
            .call()
            .and_then(|mut r| r.body_mut().read_to_string())
            .map_err(|e| fail(&e))?;
        let page: Vec<Value> = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        let n = page.len();
        items.extend(
            page.into_iter()
                .map(|i| i["data"].clone())
                .filter(|d| !matches!(d["itemType"].as_str(), Some("attachment" | "note"))),
        );
        if n < 100 {
            break;
        }
    }
    Ok(items)
}

fn s<'a>(d: &'a Value, k: &str) -> &'a str {
    d[k].as_str().unwrap_or("")
}

/// "Name", "A & B" or "A et al.", and the year.
fn label(d: &Value) -> (String, String) {
    let names: Vec<&str> = d["creators"]
        .as_array()
        .into_iter()
        .flatten()
        // software lists programmers
        .filter(|c| {
            matches!(
                c["creatorType"].as_str().unwrap_or("author"),
                "author" | "programmer"
            )
        })
        .map(|c| {
            let last = c["lastName"].as_str().filter(|n| !n.is_empty());
            last.or(c["name"].as_str()).unwrap_or("")
        })
        .collect();
    let date = s(d, "date");
    let year = date
        .as_bytes()
        .windows(4)
        .position(|w| w.iter().all(u8::is_ascii_digit))
        .map_or("n.d.".to_string(), |i| date[i..i + 4].to_string());
    let who = match names[..] {
        [] => s(d, "title").chars().take(30).collect(),
        [a] => a.to_string(),
        [a, b] => format!("{a} & {b}"),
        [a, ..] => format!("{a} et al."),
    };
    (who, year)
}

/// The DOI, except arXiv's 10.48550 DOIs, where the item's URL is better.
fn link(d: &Value) -> String {
    let doi = s(d, "DOI");
    if !doi.is_empty() && !doi.to_lowercase().starts_with("10.48550") {
        return format!("https://doi.org/{doi}");
    }
    match s(d, "url") {
        "" if !doi.is_empty() => format!("https://doi.org/{doi}"),
        u => u.to_string(),
    }
}

/// Text for a shortcode argument: no tags, entities decoded, one line, no `"`.
fn clean(t: &str) -> String {
    let mut out = String::new();
    let mut tag = false;
    for c in t.chars() {
        match c {
            '<' => tag = true,
            '>' if tag => tag = false,
            _ if !tag => out.push(c),
            _ => {}
        }
    }
    crate::inline::unescape(&out)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'")
}

/// One `- {{< paper … >}}` line per item, sorted by label; equal labels get
/// 2021a, 2021b. Also the warnings (items with no link).
pub fn lines(items: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut items: Vec<(&Value, (String, String))> = items.iter().map(|d| (d, label(d))).collect();
    items.sort_by(|(a, la), (b, lb)| {
        (la.0.to_lowercase(), &la.1, s(a, "title")).cmp(&(
            lb.0.to_lowercase(),
            &lb.1,
            s(b, "title"),
        ))
    });
    let mut count: HashMap<&(String, String), usize> = HashMap::new();
    for (_, l) in &items {
        *count.entry(l).or_default() += 1;
    }
    let mut seen: HashMap<&(String, String), u8> = HashMap::new();
    let (mut out, mut warnings) = (Vec::new(), Vec::new());
    for (d, l) in &items {
        let (who, mut year) = l.clone();
        if count[l] > 1 {
            let n = seen.entry(l).or_default();
            year.push((b'a' + *n) as char);
            *n += 1;
        }
        let url = link(d);
        if url.is_empty() {
            warnings.push(format!(
                "{who} ({year}) has no DOI or URL; its entry links nowhere. Add one in Zotero."
            ));
        }
        out.push(format!(
            r#"- {{{{< paper url="{url}" who="{}" year="{year}" title="{}" >}}}}"#,
            clean(&who),
            clean(s(d, "title"))
        ));
    }
    (out, warnings)
}

/// Replace what is between the deck's one marker pair with `rows`.
pub fn patch(d: &Deck, rows: &[String]) -> Result<Patch> {
    let starts: Vec<usize> = d.text.match_indices(START).map(|(i, _)| i).collect();
    let ends: Vec<usize> = d.text.match_indices(END).map(|(i, _)| i).collect();
    let hint = format!("the deck needs one `{START} -->` … `{END}` pair on its References slide");
    match (starts.len(), ends.len()) {
        (0, _) | (_, 0) => return Err(QdError::NotFound(hint)),
        (1, 1) => {}
        _ => return Err(QdError::Ambiguous(hint)),
    }
    let line_end = d.text[starts[0]..].find('\n').map(|i| starts[0] + i + 1);
    let Some(from) = line_end.filter(|&f| f <= ends[0]) else {
        return Err(QdError::NotFound(hint));
    };
    let body: String = rows.iter().map(|r| format!("{r}{}", d.nl)).collect();
    Ok(Patch {
        op: "bib".into(),
        replacements: vec![(
            Span {
                start: from,
                end: ends[0],
            },
            body,
        )],
        assigned_ids: vec![],
        target: "bibliography".into(),
        converted_units: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn item(last: &[&str], year: &str, title: &str, doi: &str, url: &str) -> Value {
        let creators: Vec<Value> = last
            .iter()
            .map(|n| json!({"creatorType": "author", "lastName": n}))
            .collect();
        json!({"itemType": "journalArticle", "creators": creators, "date": year,
               "title": title, "DOI": doi, "url": url})
    }

    #[test]
    fn labels_links_and_sorting() {
        let items = vec![
            item(&["Zed"], "2020-01-02", "Z", "10.1/z", ""),
            item(
                &["Bonaca", "Price-Whelan"],
                "2025",
                "Streams <i>in</i> &amp; \"Gaia\"",
                "10.1/b",
                "",
            ),
            item(
                &["Ab", "C", "D"],
                "March 2021",
                "Second",
                "10.48550/arXiv.1",
                "http://arxiv.org/abs/1",
            ),
            item(&["Ab", "X", "Y"], "2021", "First", "", ""),
            json!({"itemType": "computerProgram", "creators": [{"creatorType": "programmer", "name": "Astropy"}], "date": "", "title": "astropy", "url": "https://astropy.org"}),
        ];
        let (rows, warnings) = lines(&items);
        assert_eq!(
            rows,
            vec![
                r#"- {{< paper url="" who="Ab et al." year="2021a" title="First" >}}"#,
                r#"- {{< paper url="http://arxiv.org/abs/1" who="Ab et al." year="2021b" title="Second" >}}"#,
                r#"- {{< paper url="https://astropy.org" who="Astropy" year="n.d." title="astropy" >}}"#,
                r#"- {{< paper url="https://doi.org/10.1/b" who="Bonaca & Price-Whelan" year="2025" title="Streams in & 'Gaia'" >}}"#,
                r#"- {{< paper url="https://doi.org/10.1/z" who="Zed" year="2020" title="Z" >}}"#,
            ]
        );
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].starts_with("Ab et al. (2021a)"));
    }

    #[test]
    fn writes_between_the_markers_only() {
        let text = "## R {#refs}\r\n\r\n<!-- bibliography:start (by qd) -->\r\n- old\r\n<!-- bibliography:end -->\r\n\r\n## Next\r\n";
        let d = Deck::parse(text.to_string());
        let p = patch(&d, &["- a".into(), "- b".into()]).unwrap();
        let out = crate::ops::apply(text, &p).unwrap();
        assert_eq!(out, text.replace("- old\r\n", "- a\r\n- b\r\n"));
        let empty = crate::ops::apply(text, &patch(&d, &[]).unwrap()).unwrap();
        assert_eq!(empty, text.replace("- old\r\n", ""));
    }

    #[test]
    fn needs_exactly_one_marker_pair() {
        let one = "<!-- bibliography:start -->\n<!-- bibliography:end -->\n";
        let code = |t: &str| patch(&Deck::parse(t.to_string()), &[]).unwrap_err().code();
        assert_eq!(code("## R\n"), "E_NOT_FOUND");
        assert_eq!(code("<!-- bibliography:start -->\n"), "E_NOT_FOUND");
        assert_eq!(code(&one.repeat(2)), "E_AMBIGUOUS");
        assert_eq!(
            code("<!-- bibliography:end -->\n<!-- bibliography:start -->\n"),
            "E_NOT_FOUND"
        );
        assert!(patch(&Deck::parse(one.to_string()), &[]).is_ok());
    }

    #[test]
    fn rejects_keys_that_are_not_zotero_keys() {
        for k in ["", "ABC", "ABCD/EFG", "ABCDEFGHI"] {
            assert_eq!(fetch(k).unwrap_err().code(), "E_INVALID_ARG", "{k}");
        }
    }
}
