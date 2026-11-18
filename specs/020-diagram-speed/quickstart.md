# Quickstart: check that diagrams keep up

Uses a made-up deck only.

## Setup

```bash
cargo build --release
QD=$PWD/target/release/qd
mkdir -p /tmp/dot-speed && cd /tmp/dot-speed
printf -- '---\ntitle: Speed\nformat: revealjs\n---\n\n## Diagram {#d}\n\n' > deck.qmd
$QD dot new --on d
$QD serve --deck deck.qmd
```

Add a few boxes with Insert ▸ Box in diagram, and some slides with text and
images so a render takes several seconds.

## Scenarios

1. **Drop**: drag a box and let go. Its arrows are routed at once. The
   status shows "Rendering…" and then "Up to date", and nothing on the
   slide moves when it does.
2. **Drag again at once**: drag three boxes one after another as fast as
   possible. Each drag starts at once, and each drop draws at once.
3. **Live routing**: while dragging, the arrows follow the box (the
   default). With Settings ▸ "Route arrows while dragging" off, they are
   straight stand-in lines, as before.
4. **Other diagram edits**: connect two boxes, rename one, change its
   colour in Properties, delete an arrow, and nudge a box with the arrow
   keys. Each shows at once.
5. **Undo**: ⌘Z after a move puts the box back at once.
6. **Pictures**: give a box a picture (Properties ▸ Picture). The first
   time, the preview reloads, since this installs `dot-image`. After that,
   moving the box moves its picture at once.
7. **Other edits unchanged**: edit a paragraph. The preview reloads after
   the render, as before.
