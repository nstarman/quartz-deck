//! `qd-zotero COLLECTION`: the source plugin behind `qd bib pull --from
//! zotero` (spec 014 US3, research R8). It prints the collection's items as
//! a CSL-JSON array, read from the running Zotero 7's local API on this
//! machine only. Errors go to stderr as `CODE message` and a hint, exit 2.

use serde_json::{Value, json};
use std::time::Duration;

const HINT: &str = "start Zotero 7 and tick Settings > Advanced > \"Allow other applications on this computer to communicate with Zotero\"";

fn fail(code: &str, message: &str, hint: &str) -> ! {
    eprintln!("{code} {message}\nhint: {hint}");
    std::process::exit(2)
}

/// The key a deck cites the item by: Zotero 7's own, else Better BibTeX's line in Extra.
fn citation_key(data: &Value) -> Option<&str> {
    let own = data["citationKey"].as_str().filter(|k| !k.is_empty());
    own.or_else(|| {
        data["extra"]
            .as_str()?
            .lines()
            .find_map(|l| Some(l.strip_prefix("Citation Key:")?.trim()).filter(|k| !k.is_empty()))
    })
}

/// One item as a deck's reference: its CSL-JSON, keyed by its citation key.
fn entry(item: &Value) -> Value {
    let mut e = item["csljson"].clone();
    let obj = e.as_object_mut().expect("Zotero's CSL-JSON is an object");
    // Zotero's own ids ("1234/ABCD") are not citation keys: without a key, qd makes one
    obj.remove("id");
    if let Some(k) = citation_key(&item["data"]) {
        obj.insert("id".into(), json!(k));
    }
    // arXiv's 10.48550 DOIs: the item's page is the better link
    let c = &item["csljson"];
    if c["DOI"]
        .as_str()
        .is_some_and(|d| d.to_lowercase().starts_with("10.48550"))
        && c["URL"].as_str().is_some_and(|u| !u.is_empty())
    {
        obj.remove("DOI");
    }
    e
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [key] = &args[..] else {
        fail(
            "E_USAGE",
            "usage: qd-zotero COLLECTION",
            "the collection's 8-character key, from its Zotero URL",
        );
    };
    // it goes into a URL: Zotero keys are 8 letters and digits
    if key.len() != 8 || !key.chars().all(|c| c.is_ascii_alphanumeric()) {
        fail(
            "E_INVALID_ARG",
            &format!("'{key}' is not a Zotero collection key"),
            "the collection's 8 letters and digits, from its Zotero URL",
        );
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(5)))
        .build()
        .into();
    // Zotero's local API (user 0 is the local user); `QD_ZOTERO_URL` replaces it in tests
    let base = std::env::var("QD_ZOTERO_URL").unwrap_or_else(|_| "http://localhost:23119".into());
    let mut out = Vec::new();
    for start in (0..).step_by(100) {
        let url = format!(
            "{}/api/users/0/collections/{key}/items/top?include=data,csljson&limit=100&start={start}",
            base
        );
        let text = agent
            .get(&url)
            .call()
            .and_then(|mut r| r.body_mut().read_to_string())
            .unwrap_or_else(|e| {
                fail(
                    "E_ZOTERO",
                    &format!("Zotero's local API is not reachable: {url}: {e}"),
                    HINT,
                )
            });
        let page: Vec<Value> = serde_json::from_str(&text).unwrap_or_else(|e| {
            fail(
                "E_ZOTERO",
                &format!("{url}: not a list of items: {e}"),
                HINT,
            )
        });
        let n = page.len();
        out.extend(
            page.iter()
                .filter(|i| !matches!(i["data"]["itemType"].as_str(), Some("attachment" | "note")))
                .filter(|i| i["csljson"].is_object())
                .map(entry),
        );
        if n < 100 {
            break;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&out).expect("JSON values serialize")
    );
}
