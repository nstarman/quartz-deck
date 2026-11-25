# Contract: CLI changes

| Command | Change |
|---|---|
| `qd block move EL --before/--after REF` | EL and REF may be nested and in different lists of one slide |
| `qd block move EL --into COLUMN` | new: EL goes last in COLUMN |
| `qd block add SLIDE --into COLUMN --md …` | new: added last in COLUMN |
| `qd block add SLIDE --before/--after REF` | REF may be nested |
| `qd block rm / dup EL` | EL may be nested; a column's last element leaves the column empty |
| `qd block place EL …` | EL may be nested: it leaves its column and becomes a box |
| `qd block attr EL key=value… [--unset key]` | new (P4) |
| `qd column add COLUMNS` | new (P4): an empty column at the end |

Results report `target`: the element's ref after the change.
