# Feature Specification: History

**Feature Branch**: `012-history`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "History (roadmap 012): never lose work, and be able to look back, built on git and visible in the editor. (P1) Setting "Save as git commit" (007 Settings): when on and the deck is in a git repository, saves (on-slide edit, source edit, CLI edit) also make commits of the .qmd only, with a message describing the change (`qd: edit results/1`, `qd: title methods`, person or agent noted); off by default; never commits other files, never pushes; edits in a burst (typing, a run of agent commands) become one commit after a pause. (P1) History panel beside the slides: a timeline of commits that touched the deck (qd's and the person's own), with time, author and message; selecting one shows a side preview of the deck as it was (rendered, navigable, next to the current deck), changed slides marked. (P2) Diff any version against the current file or another version: source diff per slide, and the rendered slides side by side. (P2) Restore the whole deck, one slide or one element from a chosen version, as a normal qd edit (undoable, conflict-checked, itself committed when the setting is on); never rewrites git history. CLI for agents: `qd history [--slide REF]`, `qd diff REV [REV]`, `qd restore REV [--slide REF | --element REF]`, JSON output. Open questions: how qd's commits interact with the person's own workflow (staged changes, other files, branches, hooks, signing); what to do outside a git repository (offer git init, or a private history in qd's cache); rendering old versions (render on demand into the cache, following 004's rule that code runs only on an explicit action)."

## Clarifications

### Session 2026-11-12

- Q: How do qd's commits treat signing and commit hooks? → A: exactly as the
  person's own commits do. They follow the repository's git configuration,
  so they are signed and hooks run. If signing or a hook fails (a password
  manager that is locked, a hook that refuses), the edit stays saved, qd says
  why, and it tries again at the next pause.
- Q: What happens when the deck is not in a git repository? → A: History says
  the deck isn't in git and offers to create a repository in the deck's
  folder. This is one click by the person and never happens on its own.
  Without one, History and the commit setting do nothing else; qd keeps no
  private history.

## Context

qd's undo goes back through qd's own edits, but only until the file changes
some other way, and it doesn't survive a restart. Talks change a lot in the
last days before they are given. A slide that worked last week gets rewritten,
an agent's run of edits goes the wrong way, or a figure gets replaced. People
who keep their decks in git can dig the old version out, but only by leaving
the editor, reading `git log`, and copying text back by hand. This feature
puts the deck's history in the editor. qd can save versions as commits on
its own; the person can look through every version that touched the deck,
see what changed slide by slide, and bring back the whole deck, one slide, or
one element. All of it is also available to agents from the command line.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Look back through the deck's versions (Priority: P1)

The person opens **History** beside the slide list. It shows every commit
that touched the deck: their own and qd's, newest first, each with when, who,
and the message. Selecting one opens the deck as it was in a second preview
next to the current one. They can step through its slides, and slides that
differ from today's are marked. An agent gets the same list from
`qd history`, or the commits that touched one slide from
`qd history --slide methods`.

**Why this priority**: Looking back is useful on its own for any deck already
in git, with no new setting and nothing written. Every other story builds on
it.

**Independent Test**: In a git repository with a deck that has five commits,
open History. It lists five entries with the right authors and messages.
Selecting the third shows that version's slides, with the slides changed
since then marked. `qd history` returns the same five as JSON.

**Acceptance Scenarios**:

1. **Given** a deck with commits by the person and by qd, **When** History
   opens, **Then** both kinds are listed, newest first, with time, author
   and message, and qd's are marked as made by qd.
2. **Given** a selected version, **When** its preview opens, **Then** it
   shows that version's slides next to the current deck, the person can step
   through them, and slides that differ from the current file are marked.
3. **Given** a version with code cells, **When** its preview opens, **Then**
   no code runs. The cells show their code, and the preview offers to run
   them on request (004's rule).
4. **Given** `qd history --slide methods`, **When** it runs, **Then** it
   lists only the versions in which that slide's text changed.
5. **Given** the deck has changes not yet committed, **When** History opens,
   **Then** the top entry is "Not committed" and stands for the current file.

---

### User Story 2 — Save as git commit (Priority: P1)

The person turns on **Save as git commit** in Settings. From then on, their
edits on the slide, in the source pane and by agents through `qd` are
committed after a short pause, a burst of edits as one commit. Each commit
holds only the deck file, and its message says what changed and who changed
it (`qd: edit results/1 (person)`, `qd: title methods, add slide (agent)`).
qd never pushes, never commits other files, and never touches what the person
has staged.

**Why this priority**: History is only as good as the versions saved.
Without this, a deck edited for an afternoon has one version: the last commit
the person remembered to make.

**Independent Test**: With the setting on, make three edits within a few
seconds, wait for the pause, then make one more. Exactly two new commits
appear. Each changes only the deck file and names its edits. A file the
person had staged before is still staged and not committed.

**Acceptance Scenarios**:

1. **Given** the setting is off (the default), **When** the deck is edited,
   **Then** no commit is made.
2. **Given** the setting is on and edits come in a burst, **When** the pause
   ends, **Then** one commit holds them all, with a message listing the
   edits and whether a person or an agent made them.
3. **Given** other files are staged or changed, **When** qd commits, **Then**
   only the deck file is in the commit, and the other files' staged and
   unstaged state is unchanged.
4. **Given** the repository is mid-merge, mid-rebase, or on a detached head,
   **When** a commit is due, **Then** qd does not commit and says why once,
   and commits again when the repository is back to normal.
5. **Given** the repository signs commits or has commit hooks, **When** qd
   commits, **Then** the commit is signed and the hooks run, exactly as for
   the person's own commits.
6. **Given** a commit fails (signing locked, a hook refuses), **When** it
   fails, **Then** the edit is still saved in the file, nothing is lost, the
   editor says the commit failed and why, and qd tries again at the next
   pause.

---

### User Story 3 — See what changed (Priority: P2)

The person picks two versions, or one version and the current file, and sees
what changed: the source, slide by slide, with only the changed slides shown,
and the two renders of each changed slide side by side. `qd diff REV [REV]`
gives an agent the same thing as JSON: per slide, whether it was added,
removed, moved or changed, with the text before and after.

**Why this priority**: The preview (US1) shows a version; the diff shows what
an edit did. That matters most when someone else, or an agent, made the
change.

**Independent Test**: Between two versions where one slide's title changed,
one slide was added and one moved, the diff lists exactly those three slides
with the right kinds. The changed slide shows before and after, both as
source and rendered.

**Acceptance Scenarios**:

1. **Given** two versions, **When** they are compared, **Then** only the
   slides that differ are listed, each marked added, removed, moved or
   changed, matched by slide id where slides have one.
2. **Given** a changed slide, **When** it is opened in the diff, **Then** its
   source shows changed lines marked, and its two renders sit side by side.
3. **Given** `qd diff HEAD~3`, **When** it runs, **Then** it compares that
   version with the current file and prints the per-slide changes as JSON.

---

### User Story 4 — Restore (Priority: P2)

From a version in History, the person restores the whole deck, one slide, or
one element. A restore is an ordinary qd edit: it can be undone, it is
refused if the file changed underneath, and it is committed like any other
edit when the setting is on. It never rewrites git history. An agent does the
same with `qd restore REV [--slide REF | --element REF]`.

**Why this priority**: Looking back pays off when the old version can come
back without copying text by hand.

**Independent Test**: Change a slide's text, commit, then restore that slide
from the earlier version. The slide's text is back, the rest of the deck is
untouched, one undo removes the restore, and git's log has gained one commit
(with the setting on) and lost none.

**Acceptance Scenarios**:

1. **Given** a slide that exists in both versions, **When** it is restored,
   **Then** only that slide's text changes, back to the old version's.
2. **Given** a slide deleted since that version, **When** it is restored,
   **Then** it comes back at the position nearest to where it was.
3. **Given** an element (`#id` or `slide/i`), **When** it is restored,
   **Then** only that element changes. If its slide no longer exists, the
   restore is refused with a message that says to restore the slide
   instead.
4. **Given** the whole deck, **When** it is restored, **Then** the file
   becomes that version's, as one edit that one undo takes back.
5. **Given** the file changed since the person looked, **When** the restore
   runs, **Then** it is refused as a conflict and nothing is written.

---

### Edge Cases

- **The deck is not in a git repository**: History says so and offers
  **Start a history here** (a git repository in the deck's folder, created
  only when the person clicks it; `qd history --init` for agents). Until
  then, the commit setting has no effect, and qd keeps no history of its
  own.
- **The deck file was renamed or moved in git's history**: History follows
  the file through renames.
- **A version where the deck file can't be parsed**: it appears in History,
  but its preview says it couldn't be read and shows the raw text.
- **A very long history (thousands of commits)**: History loads the newest
  first and more on scroll; `qd history` takes `--limit`.
- **Old versions that need files no longer there** (images, data): the
  preview uses the files as they were in that version when git has them, and
  marks any that are missing.
- **Two people or an agent editing during a pending commit**: the commit made
  after the pause holds whatever the file is at that moment, and its message
  lists all the edits since the last commit.
- **The person commits by hand while a qd commit is pending**: qd sees that
  the file is already committed and makes no empty commit.
- **The repository has another branch checked out than when qd started**: qd
  commits to whatever branch is checked out at commit time.

## Requirements *(mandatory)*

### Functional Requirements

**Reading history**

- **FR-001**: Users MUST be able to see every version of the deck from git:
  commits that changed the deck file, newest first, with time, author,
  message and whether qd made it, following renames.
- **FR-002**: Users MUST be able to list only the versions in which one slide
  (by id or position) or one element changed.
- **FR-003**: Uncommitted changes to the deck MUST appear as the top entry,
  "Not committed".
- **FR-004**: Selecting a version MUST show its deck rendered in a second
  preview beside the current deck, navigable slide by slide, with slides that
  differ from the current file marked.
- **FR-005**: Rendering an old version MUST NOT run code. Cells show their
  code, and running them is a separate explicit action (constitution, 004).
  Renders of old versions are kept in qd's cache, never beside the deck.

**Saving versions**

- **FR-006**: A setting **Save as git commit** (off by default) MUST make qd
  commit the deck file after edits: on-slide, source pane and CLI.
- **FR-007**: Edits less than a pause apart MUST go into one commit, made
  once the pause has passed. The pause is a setting (default 30 seconds).
- **FR-008**: A qd commit MUST contain only the deck file. It MUST leave every
  other file's staged and unstaged state as it was, and it MUST NOT push.
- **FR-009**: A qd commit's message MUST start with `qd:`, list the edits it
  holds (from the edits' own names), and say whether a person, an agent, or
  both made them.
- **FR-010**: qd MUST NOT commit during a merge, a rebase, a cherry-pick, or on
  a detached head. It says so once and commits normally afterwards.
- **FR-011**: qd's commits MUST follow the repository's git configuration
  exactly as the person's own commits do: signing, hooks and author. qd adds
  no flags that skip or change them.
- **FR-011a**: A failed commit (signing, a hook, anything else) MUST NOT lose
  or undo the edit. The editor and the CLI result report the failure and its
  reason, and qd tries again at the next pause.
- **FR-011b**: Outside a git repository, qd MUST offer to create one in the
  deck's folder (`qd history --init`, or one click in History) and MUST
  never create one on its own. It keeps no private history.
- **FR-012**: qd MUST NOT rewrite git history: no amend, no squash after the
  fact, no reset, no force.

**Comparing**

- **FR-013**: Users MUST be able to compare any two versions, or a version and
  the current file. The comparison lists the slides that were added,
  removed, moved or changed, matching slides by id where they have one.
- **FR-014**: For a changed slide, the comparison MUST show the source with
  changed lines marked and both renders side by side.

**Restoring**

- **FR-015**: Users MUST be able to restore the whole deck, one slide, or one
  element from any version, as one qd edit. It is undoable,
  conflict-checked against the version the person was looking at, and
  committed like other edits when the setting is on.
- **FR-016**: Restoring a slide that no longer exists MUST put it back at the
  position nearest its old one. Restoring an element whose slide no longer
  exists MUST be refused with a hint to restore the slide.

**CLI and editor**

- **FR-017**: Each capability MUST be a `qd` command with JSON output:
  `qd history [--slide REF | --element REF] [--limit N]`,
  `qd diff REV [REV]`, and `qd restore REV [--slide REF | --element REF]`.
  REV accepts what git accepts (a hash, `HEAD~2`, a tag).
- **FR-018**: History, compare and restore MUST be actions in the 007 action
  list, with a History panel in the View menu and restore on the slide and
  element right-click menus when a version is open.
- **FR-019**: Reading history and restoring MUST be same-origin, like every
  other read and edit. Creating a repository and rendering an old version
  write outside the deck file or run Quarto, so they MUST also require the
  session token.

### Key Entities

- **Version**: a commit that changed the deck file, or the uncommitted state:
  its revision, time, author, message, whether qd made it, and the deck's
  text at that revision.
- **Slide change**: one slide that differs between two versions: added,
  removed, moved (old and new position) or changed, with its text in each
  version.
- **Pending commit**: the edits since the last qd commit, with who made them
  and when the pause ends.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person can find and preview a version from a week ago in under
  30 seconds, starting from the editor, without using a terminal.
- **SC-002**: With the setting on, an afternoon of editing (a few hundred
  edits in bursts) produces commits in the low tens, not hundreds. Each
  commit's message names its edits.
- **SC-003**: Restoring one slide changes no byte outside that slide, and one
  undo returns the file byte-for-byte to before the restore.
- **SC-004**: In every test of qd's commits, files other than the deck keep
  their staged and unstaged state exactly, and git history never loses a
  commit.
- **SC-005**: History opens in under 2 seconds for a deck with 1,000 commits.
- **SC-006**: An agent can answer "what changed on slide X since yesterday,
  and put it back" with two commands, `qd history`/`qd diff` and then
  `qd restore`, using only their JSON.

## Assumptions

- git is installed wherever history is used. Without git, History says so;
  nothing else changes.
- The "pause" is measured from the last edit. A commit due when `qd serve`
  stops is made on stop, and a CLI edit with no server running commits at
  once.
- Edits record who made them: the editor's are the person's, and the CLI's
  are the agent's unless run from the editor. 005 already tells the two
  apart.
- The deck's own git history is the source of truth. qd keeps no second
  history.
- A repository that signs commits through a password manager prompts once
  per qd commit. Bursts keep that to one prompt per pause, and a locked
  manager makes the commit wait, not fail the edit.
- Slide matching across versions uses slide ids, then titles, then
  positions, in that order.
- Old versions are rendered on demand and cached by revision. Quarto renders
  them as it renders the current deck, with code not run (FR-005).
