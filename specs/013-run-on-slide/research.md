# Research: Run on the slide (013)

Checked on this machine on 2026-11-12, with Quarto 1.11.9, uv 0.13.7 and no
R, Julia or Jupyter on PATH. Each probe is recorded under its decision.
Where a probe needs R or Julia, which this machine lacks, the decision says
so, and the first task of that story runs the probe in CI.

## R1. One cell runs in a live session per deck, not through a render

Decision: as in 004, a run executes one cell in a live session that belongs
to the deck and keeps state between runs. Python and other Jupyter languages
use a Jupyter kernel; R uses one R process. A full `quarto render` still
makes the deliverable (`qd render`, `qd export`).

Rationale: Quarto has no per-cell API. Re-rendering the deck with `freeze`
and picking out one cell's output costs a whole render per click, seconds to
minutes, which breaks SC-002 (under 2 s for a quick cell) and the
edit-and-see loop of US2. A live session is what notebooks do, and 004
already works this way for Python.

Alternatives considered: re-render with `freeze: auto` and read the cell
from the frozen output (too slow, and every other cell's cache must stay
valid); `quarto render --execute-daemon` (Quarto's own Jupyter daemon). The
daemon keeps a kernel warm but still renders the whole document, and it
exists only for Jupyter.

## R2. Which engine: ask Quarto once, then reuse the answer

Decision: qd runs `quarto inspect <deck>` and reads `engines[0]` (`jupyter`,
`knitr`, or `markdown` when no cell executes, which is the OJS-only case).
It also reads the revealjs `execute` options (`fig-width`, `fig-height`,
`fig-dpi`), so plots come out the size a render gives. The answer is cached
in qd's cache, keyed by a hash of the front matter plus the cells' languages,
so it is asked again only when one of those changes. If `inspect` fails, its
message becomes the error (`E_ENGINE`). For a missing kernel that message
already names the kernels that do exist.

Probe:
- `quarto inspect` gave `["knitr"]` for an `{r}` deck, `["jupyter"]` for a
  `{python}` deck and `["markdown"]` for an `{ojs}`-only deck.
- For `jupyter: julia-1.10` with no Julia it failed with "Jupyter kernel
  'julia-1.10' not found. Known kernels: python3."
- It took 3.4 s, too slow per click, but fine once per front-matter change.
- `codeCells` lists each cell's language and source, and
  `formats.revealjs.execute` has `fig-width: 10, fig-height: 5, fig-dpi: 96`.

Rationale: this way qd makes the same choice Quarto makes (FR-010), and its
rules (front matter `engine:`, `jupyter:`, `knitr:`, then the cells'
languages) never need copying. The cache keeps the click fast.

Alternatives considered: re-implementing Quarto's rules in qd. Rejected:
they change between Quarto versions (the native Julia engine is recent),
and a copy would drift.

## R3. Jupyter decks: the named kernel, hosted without a deck environment

Decision: the existing bridge (`kernel_bridge.py`) starts the kernel.
- **Where the bridge runs**: in the deck's locked environment when it has
  one (004), and otherwise with `uv run --no-project --with jupyter_client
  python -I`, which needs no deck files.
- **Kernel choice**, in order:
  1. the kernel the front matter names (`jupyter: NAME`, or `jupyter:
     {kernelspec: {name: NAME}}`);
  2. for a Python deck with an environment, its `qd` kernel, exactly as in
     004;
  3. otherwise the first installed kernelspec whose language matches the
     deck's first cell, which is Quarto's rule.
- **Missing kernel**: `E_ENGINE`, naming the language and the kernels found.

Probe: `uv run -q --no-project --with jupyter_client python -I -c "…
KernelSpecManager().find_kernel_specs()"` listed this machine's own kernels
(`python3` and three others in `~/Library/Jupyter/kernels`). The first run
took 2.5 s (resolving), and later runs 0.7 s. So the person's kernels are
visible from uv's throwaway environment.

Rationale: a Julia or R-via-Jupyter deck shouldn't need a Python environment
of its own just to host the bridge (clarification: R and Julia run with the
person's own install). uv is already required for 004.

Alternatives considered:
- `jupyter` on PATH. Not installed here, and often missing.
- Quarto's own Python (`QUARTO_PYTHON`). It's found through `quarto check
  jupyter`, which takes seconds, and it may lack `jupyter_client`.
- Requiring `qd env init` for every Jupyter deck. That's extra setup the
  clarification ruled out.

## R4. knitr decks: one R process running `evaluate`

Decision: a second bridge, `kernel_bridge.R`, embedded like the Python one,
runs in `R --no-echo --no-restore --no-save` started in the deck's folder.
- **Protocol**: the same JSON lines (R5).
- **Running a cell**: each `execute` request runs the code with
  `evaluate::evaluate()` in the global environment, so state carries over.
- **Outputs**:
  - text, messages and warnings become `text`;
  - errors become `error` with the condition message as the traceback;
  - each recorded plot is saved with `png()` at the deck's `fig-width` ×
    `fig-height` inches and `fig-dpi`, and becomes `image`;
  - a `knit_print` value with an HTML class becomes `html`.
- **Interrupt**: qd sends SIGINT to the R process, which ends the
  evaluation with an interrupt condition, giving status `stopped`.
- **Restart**: R is stopped and started again.

`evaluate` is a dependency of knitr, and `jsonlite` of rmarkdown. Quarto's
knitr engine needs both knitr and rmarkdown, so wherever `quarto render`
runs R, the bridge's packages are there.

Not probed here, because R isn't installed. US3's first task installed R in
CI (`r-lib/actions/setup-r`, with knitr and rmarkdown), and `knitr_cells`
checked it there (Ubuntu runner, 2026-11-12):
- `quarto inspect`, starting R and running three cells took 1.4 s in all;
- `x <- 20` in one cell was there for `x + 22` in the next;
- `plot(1:10)` came out as a 960 × 480 PNG, the deck's 10 × 5 in at 96 dpi;
- `--timeout 2` on `Sys.sleep(30)` ended the run without killing R.

Rationale: `evaluate` is what knitr itself uses to run a chunk, so outputs
split and plots are recorded the way a render does them. An R kernel for
Jupyter (IRkernel) would be a second install the person may not have.

Alternatives considered:
- IRkernel through R3. Not installed by default.
- `knitr::knit()` of a one-chunk document per run. Output comes back as
  markdown to parse, and figure paths change on every run.
- `Rscript` per run. It loses state between cells (FR-012).

## R5. One protocol, one session type

Decision: both bridges speak 004's JSON-lines protocol unchanged:
- `{"op": "execute", "code"}`, `{"op": "interrupt"}`, `{"op": "restart"}`;
- replies `{status, counter, outputs}`;
- output kinds `text`, `html`, `image` (a file path), `error`.

`kernel::Session::start` takes the command to spawn instead of building the
Python one itself. Interrupt sends `{"op": "interrupt"}` to the Python
bridge and SIGINT to R. Outputs, staleness, truncation and storage
(`outputs.rs`) stay as they are.

Rationale: run plans, Run all, Stop, Restart, stored outputs, the CLI and
the server stay one code path for every engine (constitution VII).

## R6. The new output on the slide, in place

Decision: the server already sends `cell_output` when a cell ends. The event
gains `html`, the same block the preview inserts (`render::output_html`, a
`<div class="cell-output qd-output">`), and that block gains
`data-qd-cell="<ref>"`. The editor replaces the block with that ref inside
the preview frame. If the cell had no output yet, it inserts the block
after the cell's element. The full preview re-render that follows a run (004)
still happens, so stale marks and layout catch up, and it keeps the current
slide as it does today.

Rationale: one renderer for outputs, the server's, so the in-place block
and the re-rendered one are identical. The person sees the result as soon
as the cell ends, without waiting 1–2 s for the re-render (SC-002).

Alternatives considered: building the output DOM in the editor from the
items, as the source pane's dock does. That would be a second renderer that
can differ from the preview's.

## R7. ▶ and state on the slide: the editor's overlay

Decision:
- **The ▶**: drawn in the editor's overlay over the preview (like 003's
  hover bar). It shows on a cell element that is hovered or selected, is
  focusable, and is named "Run cell REF".
- **States**: running, error, stopped and stale are classes the editor puts
  on the cell's element from the run events and `/api/outputs`.
- **Keyboard**: the existing `run` action (element menu "Run cell") gains
  the key ⇧Enter where an element is selected. **Stop cell** is a new
  action, shown while that cell runs.
- **Rendered HTML**: unchanged, so `/present/` and exports never show a ▶
  (FR-008, SC-005).

Rationale: the presented deck and exports carry no editor code, and the
server's run endpoints already require the session token (004).

## R8. Run in the source pop-up

Decision: `openSource(rect, text, on)` takes an optional `on.run`. When it is
given (code cells), the bar shows **Run ⇧Enter** next to the save hint, and
⇧Enter or the button does the following:
1. calls `on.save(text)`, the same path as ⌘Enter, which saves as one edit
   with `--expect`;
2. if the save succeeds, runs the cell by its ref and keeps the pop-up open;
3. if the save fails, shows the error in the pop-up's error line and runs
   nothing (FR-006).

If the text is unchanged, nothing is saved before the run.

Ordering (FR-007): the server runs one request at a time per session, and
events arrive in order. The editor remembers the newest run id it started
for each cell and ignores a `cell_output` for that cell from an older run.

## R9. Observable JS cells (US4): evaluated in the page

Decision: OJS cells run in the preview page, through Quarto's OJS runtime,
which the preview already loads. ▶ on an `{ojs}` cell asks the runtime to
redefine that cell's variables from its current source, and the runtime's
own dependency graph re-runs the cells that depend on it. No server, kernel
or token is involved in evaluating, because the page already runs these
cells on load.

US4's first task finds and probes the runtime entry point that redefines a
cell. Quarto exposes its OJS module on the page; exactly how it does so is
to be confirmed. If there is no stable way to redefine one cell, the
fallback is to re-render the preview, which needs no kernel, and keep the
slide. That is slower but correct.

Probe (T011, the browser pane, Quarto 1.11.9):
- The page has `window._ojs.ojsConnector`, with `mainModule` (an Observable
  runtime module with `redefine`, `define`, `value`) and
  `interpreter.cell(source, module, observer)`. The preview evaluates `{ojs}`
  cells as it loads, even though it renders with `--no-execute`.
- Redefining one cell in place would need the variables each cell defined.
  The runtime doesn't keep that link, and an anonymous cell (an expression
  like `y`) has no name to find it by.

As built, the fallback: ▶ on an OJS cell loads the preview again, which
keeps the slide and runs every OJS cell. The pop-up's Run saves the edit,
and the re-render shows the result. Two things were needed for that:
- A new render now loads while a source pop-up is open. Before, renders
  waited for every edit session, so the pop-up's Run never showed the new
  code. They still wait while text is typed on the slide, where a reload
  would lose it.
- After a saved edit, Run doesn't also reload the frame, which would race
  the render's own load.

Two smaller findings:
- An OJS declaration (`x = 1`) shows no value on a revealjs slide, so its
  cell has no height. The editor's placeholder now covers any cell that
  shows nothing; for OJS it reads "Observable JS cell (a declaration shows
  no value)".
- `quarto inspect` calls an OJS-only deck `markdown`, so the server marks
  OJS cells with `"ojs": true`, and they never go to `/api/run`.

## R10. Quarto's native Julia engine (US5)

Decision, to confirm by probe when US5 starts (no Julia here):
- `engine: julia` decks run in one persistent `julia` process, through a
  third small bridge with the same protocol (R5).
- The bridge evaluates code in `Main` with `include_string` and captures
  `display` output by MIME type (png, svg, html, text), the way Quarto's
  `QuartoNotebookRunner` does.
- It uses the person's Julia, with the deck's `--project` if Quarto's
  front matter names one.

## R11. Tests without R or Julia on this machine

Decision:
- **Named Jupyter kernel**: tests install a second kernelspec under a temp
  `JUPYTER_PATH`, a Python kernel named `qd-test` with the language `python`.
  A deck that says `jupyter: qd-test` must run in it, and the test checks
  `sys.argv` in the kernel.
- **knitr**: the tests skip without `Rscript`, as other tests skip without
  Quarto. CI installs R so they run there.
- **Native Julia**: likewise behind a `julia` check. CI installs Julia when
  US5 is built.
- **Browser tests**: they cover ▶, the states, the pop-up's Run and ordering,
  with Python.

## R12. CLI and errors

Decision:
- `qd run` uses R2 to R5, so an agent's run and the person's agree
  (FR-014). The run result gains `"engine": {"kind", "kernel"?}`.
- `E_ENGINE`'s hint stops saying "only Python cells run for now". It names
  what is missing: a kernel (with the ones found), R or rmarkdown, or Julia.
- No new error codes.
