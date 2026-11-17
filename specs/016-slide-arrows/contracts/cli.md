# CLI contract: `qd arrow` (016)

Every command takes the global `--deck`, `--expect VERSION`, `--dry-run`
and `--human`, prints JSON, and is one undo step. `REF` is an arrow's
element ref (`SLIDE/i`, or any ref `qd show` accepts). Coordinates are slide
pixels, `X,Y`, whole numbers (fractions are rounded).

| command | does | prints |
|---|---|---|
| `qd arrow add SLIDE --from X,Y --to X,Y [--color C] [--width N] [--dash D] [--head H] [--heads start\|end\|both\|none] [--label T] [--label-position P] [--fragment] [--no-download]` | adds an arrow line after the slide's last content block (before its notes); installs the arrows extension into the deck if it isn't there | `{"ok","ref","diff","version","installed":[...]}` |
| `qd arrow move REF (--from X,Y \| --to X,Y \| --from X,Y --to X,Y \| --by DX,DY)` | moves the start, the end, both, or the whole arrow (control points move with `--by`) | `{"ok","ref","from":[x,y],"to":[x,y],"diff","version"}` |
| `qd arrow bend REF --through X,Y` \| `--straight` | writes `control1`/`control2` so the curve passes through the point (research R6), or removes them | `{"ok","ref","diff","version"}` |
| `qd arrow set REF KEY=VALUE…` | sets options; `KEY=` removes one; values equal to the extension's default are removed rather than written | `{"ok","ref","diff","version"}` |
| `qd arrow rm REF` | removes the arrow (the same as `qd block rm REF`) | `{"ok","diff","version"}` |
| `qd arrow show REF` | reads one arrow | `{"ok","ref","from":[x,y],"to":[x,y],"control":[[x,y],[x,y]]\|null,"options":{...},"editable":true\|{"reason","at"}}` |
| `qd arrow list SLIDE` | the slide's arrows, for agents and the editor | `{"ok","arrows":[<show>...]}` |

`set` accepts the keys in data-model.md. `--heads` maps to `head-start` and
`head-end`.

## Errors

| code | when |
|---|---|
| `E_NOT_FOUND` | `REF` isn't an arrow (not a paragraph holding one arrow shortcode); `SLIDE` doesn't exist |
| `E_UNSAFE_EDIT` | the arrow isn't `position="absolute"`, or its shortcode can't be read safely; the message gives the reason, and the arrow stays editable as text |
| `E_INVALID_ARG` | a bad coordinate, key or value (an unknown head, a label-position other than start/middle/end) |
| `E_DOWNLOAD` | the extension isn't cached and can't be downloaded (offline, `--no-download`/`QD_NO_DOWNLOAD`, or a checksum mismatch); nothing is changed |
| `E_CONFLICT` | `--expect` doesn't match the file |
