# HTTP Contract additions: 002-offline-export

No new endpoints. `POST /api/op` additionally allows `export`, `pdf` and
`check` with `{"offline": true}` (not `browser --install/--remove`, not
`--html`/`--out` pointing outside the deck folder). Replies gain `href`s for
the exported HTML/PDF when they lie inside the deck folder (as for `render`).

The editor's Export dialog shows the `CheckReport`; each problem with a
`slide` links to that preview anchor.
