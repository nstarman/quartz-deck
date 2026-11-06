# CLI contract additions (008)

## `qd images`

Lists the images beside the deck: `{"images": [{"path","kind","size","modified"}]}`
(the same as `GET /api/images`).

## `qd image add <file> [--slide S] [--after REF | --before REF | --end]`

Copies `<file>` into `images/` beside the deck under a free name (same checks
as the HTTP upload). With `--slide`, also inserts `![](images/<name>)` there
(as `qd block add`) and returns the edit; without, returns `{"path"}` only.

## `qd move <slide>... (--after S | --before S)` and `qd rm <slide>...`

Several slides at once, kept in their deck order, as one patch (one undo
step). One slide works as today.
