# Extensions contract: `cite-links` and `refs-slide` (014)

Both are Quarto filter extensions at `at: post-quarto`. Each works alone and
renders with plain `quarto render`. Before anything else, each runs citeproc
once per document (R1). The CSS and the icon are bundled. They make no
network request.

## `cite-links`

**Markup** (Pandoc's own):

| Markup | Renders |
|---|---|
| `@key` | `<a class="cite-link" href="DOI-or-URL" title="Label: Title" aria-label="…"><span class="cite-icon"></span> Name et al. (2020)</a>` |
| `[@key]`, `[@a; @b]`, `[see @a, p. 3]` | citeproc's text, with each key's part wrapped as above. Prefixes, suffixes and locators stay outside the links |
| `[[@key]]{.corner}` | the icon alone, `class="cite-link cite-corner"`, top right. Its paragraph takes no height |
| a key without DOI or URL | citeproc's text unlinked. Warning on stderr: `cite-links: no DOI or URL for 'key'` |

**Options** (front matter):

```yaml
cite-links:
  icon: true          # false: no icon
  link: paper         # references: keep citeproc's link to the list instead
csl: my-style.csl     # optional; default is the extension's talk.csl
```

**Link rule**: the DOI as `https://doi.org/DOI`, unless the DOI starts with
10.48550 and the entry has a URL, in which case the URL is used. Otherwise
the URL.

**Hover text**: "Label: Title". The label is the citation's rendered text
with the parentheses dropped from around the year ("Bonaca et al. 2020").

## `refs-slide`

**Markup**: the `#refs` div, with attributes:

```markdown
## References {.scrollable}

::: {#refs refs-style="compact" refs-columns="3" refs-size="smaller"}
:::
```

| Attribute | Values | Default |
|---|---|---|
| `refs-style` | `compact`, `short`, `full` | `compact` |
| `refs-columns` | `1`, `2`, `3` | `3` |
| `refs-size` | `normal`, `smaller`, `smallest` | `smaller` |

Scrolling is reveal.js's own `.scrollable` on the slide.

**Styles**:
- `compact`: `icon Label` links, with the labels the same as the citations'
  (R3), sorted by label, then year, then title;
- `short`: "Authors (year), *Venue*", linked;
- `full`: citeproc's entries in the document's CSL, each with its DOI or
  URL linked.

**Output classes**: `refs-slide refs-compact|refs-short|refs-full
refs-cols-N refs-size-…` on the `#refs` div. Each entry keeps citeproc's
`id="ref-KEY"`, so `link: references` still works.

## Source plugin protocol

- **The program**: `qd-NAME`, on `PATH`, called as `qd-NAME ARGS…` with no
  stdin.
- **Success**: a CSL-JSON array on stdout, exit 0. An entry's `id`, if set,
  is the key qd keeps for a new entry.
- **Failure**: a message on stderr, non-zero exit.
- **The network**: a plugin may use it only while it runs. qd itself makes
  no request.
- **The manifest**: `plugins/NAME/qd-plugin.json` documents it (data-model).
