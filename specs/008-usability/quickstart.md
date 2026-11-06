# Quickstart: Usability (008)

Prerequisites: `cargo build --release`, Quarto, the audit deck
(`specs/008-usability/audit/deck/talk.qmd`) copied somewhere writable.

1. **Slide list (US1).** `qd serve`, open the link. The title slide is first,
   every slide has a thumbnail. Hover between slides 2 and 3, click +: a slide
   appears there with "Click to add title"; type a title. Drag slide 6 to
   second: a picture follows the pointer and a gap opens; press Escape once to
   see it cancel, then drop. ⌘Z undoes the move in one step.
2. **Canvas (US2).** Select and edit the first element on each slide: no bar
   covers anything. No reveal ☰ or arrows. Move between the title slide and
   slide 1: the canvas stays put.
3. **Readable (US3).** Settings ▸ Theme: dark, then high contrast; every menu,
   select and source colour is readable; the measurement test reports 0
   failures.
4. **Present (US4).** On slide 4, click Present: full screen at slide 4. Esc.
   Present ▾ From the beginning: the title slide.
5. **Sheets and images (US5).** Insert ▸ Image…: pick `chart.svg` from the
   list. Drag a PNG from the desktop onto a slide: it lands there and
   `images/` holds it. Add a link with a bad address: the sheet says why.
6. **Words (US6).** The tab and the top bar show "Quarterly results"; status
   reads "Up to date". One bar above the slides.
7. **Bars (US7).** The formatting bar shows the common few and "Aa"; the
   source bar shows Run all, Stop, "Python · stopped" and ···; Insert has
   Code cell … Quote.
8. **Narrow (US8).** Make the window 420 px wide: a slide picker in the top
   bar, properties in ···, nothing cut off.
9. **Polish (US9).** ⇧-click three slides, drag them: they move together, one
   undo. Edit text: "Saved" shows by it.
10. **The audit again (SC-010).** `uv run specs/008-usability/audit/shoot.py`;
    compare with the original shots; record each finding's status.
