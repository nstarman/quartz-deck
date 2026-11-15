# zotero (`qd-zotero`)

A source plugin: it feeds `qd bib` with references and renders nothing. It
prints a Zotero collection as a CSL-JSON list, read from the running Zotero
7's local API on this machine only.

```bash
cargo install --git https://github.com/nstarman/quartz-deck qd-zotero
qd bib pull --from zotero S376YTIE
```

`qd bib pull --from zotero KEY` runs `qd-zotero KEY` from `PATH` and merges
what it prints into the deck's references, keeping the keys slides already
cite.
- **KEY:** the collection's 8 letters and digits, from its Zotero URL.
- **Zotero:** Zotero 7 must be running, with Settings > Advanced > "Allow
  other applications on this computer to communicate with Zotero" ticked.
- **Keys:** an item's key is Zotero 7's own citation key, else Better
  BibTeX's (`Citation Key:` in Extra). An item with neither gets one from
  qd (`surname` + year).
- **arXiv:** for arXiv's own DOIs (10.48550), the item's arXiv page is the
  link.
- **Errors** go to stderr as `CODE message` and a hint, with exit 2:
  `E_ZOTERO` when Zotero isn't reachable (within 5 s), and
  `E_INVALID_ARG` for a key that isn't one. `qd` reports them as
  `E_SOURCE`.

**The source plugin protocol:** a program `qd-NAME` on `PATH`. It takes its
arguments, prints a CSL-JSON array on stdout and exits 0. An entry's `id`
is the key qd keeps for a new entry. It may reach the network only while it
runs; `qd` never does. `qd-plugin.json` documents the command.
`QD_ZOTERO_URL` replaces `http://localhost:23119`, for tests.
