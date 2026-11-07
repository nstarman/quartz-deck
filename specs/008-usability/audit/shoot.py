# /// script
# requires-python = ">=3.11"
# dependencies = ["playwright==1.55.0"]
# ///
"""Screenshots of every editor surface for the 008 usability audit.

Serves audit/deck/talk.qmd with fresh settings and cache folders, drives the
editor with headless Playwright (the installed Chrome, no download), and saves
numbered PNGs in audit/shots/. A shot that fails is logged and skipped.

    uv run specs/008-usability/audit/shoot.py [path/to/qd]
    QD_SHOTS=shots-after uv run …   # the rerun after the fixes (T039)
"""

import contextlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

from playwright.sync_api import sync_playwright

HERE = Path(__file__).parent
SHOTS = HERE / os.environ.get("QD_SHOTS", "shots")  # the rerun after the fixes: QD_SHOTS=shots-after
QD = sys.argv[1] if len(sys.argv) > 1 else str(HERE.parents[2] / "target/release/qd")
READY = "() => { const f = document.querySelector('#preview'); return !!(f && f.contentWindow?.Reveal?.isReady?.() && document.querySelector('#slides li') && window.qdSession); }"

tmp = Path(tempfile.mkdtemp(prefix="qd-audit-"))
deck = tmp / "deck"
shutil.copytree(HERE / "deck", deck)
env = {**os.environ, "QD_CONFIG_DIR": str(tmp / "config"), "QD_CACHE_DIR": str(tmp / "cache")}
url = json.loads(subprocess.run([QD, "serve", "--detach"], cwd=deck, env=env, capture_output=True, text=True, check=True).stdout)["url"]
SHOTS.mkdir(exist_ok=True)
failed = []


def framed(params):
    return url.replace("/#t=", f"/?{params}#t=")


def open_editor(page, link):
    page.goto(link)
    page.wait_for_function(READY, timeout=60000)
    page.wait_for_timeout(800)


def go(page, anchor):
    page.evaluate(f"document.querySelector('#preview').contentWindow.location.hash = '#/{anchor}'")
    page.wait_for_function(f"() => document.querySelector('#preview').contentWindow.Reveal.getCurrentSlide()?.id === '{anchor}'")
    page.wait_for_timeout(700)


def in_slide(page, sel, dx=None):
    """Page point of an element inside the slide preview (its middle, or dx from its left)."""
    return page.evaluate(
        "([sel, dx]) => { const f = document.querySelector('#preview'); const a = f.getBoundingClientRect(); const r = f.contentDocument.querySelector(sel).getBoundingClientRect(); return [a.left + r.left + (dx ?? r.width / 2), a.top + r.top + r.height / 2]; }",
        [sel, dx],
    )


def reset(page):
    page.keyboard.press("Escape")
    page.keyboard.press("Escape")
    page.mouse.move(5, 790)
    page.wait_for_timeout(300)


def shot(page, name, steps):
    try:
        steps(page)
        page.wait_for_timeout(400)
        page.screenshot(path=SHOTS / f"{name}.png")
    except Exception as e:  # one broken shot shouldn't stop the rest
        failed.append(f"{name}: {e}".splitlines()[0])
    finally:
        with contextlib.suppress(Exception):
            reset(page)


def editor_shots(page, prefix):
    shot(page, f"{prefix}01-start", lambda p: None)
    shot(page, f"{prefix}02-content", lambda p: go(p, "highlights"))

    def hover(p):
        go(p, "highlights")
        p.mouse.move(*in_slide(p, "[data-qd='highlights/0'] li"))

    shot(page, f"{prefix}03-hover-element", hover)

    def context_menu(p):
        go(p, "highlights")
        p.mouse.click(*in_slide(p, "[data-qd='highlights/0'] li"), button="right")

    shot(page, f"{prefix}04-context-menu", context_menu)

    def selected(p):
        context_menu(p)
        p.keyboard.press("Escape")

    shot(page, f"{prefix}05-selected", selected)

    def editing(p):
        go(p, "highlights")
        p.mouse.click(*in_slide(p, "[data-qd='highlights/0'] li", 6))
        p.wait_for_function("() => window.qdSession() !== null")

    shot(page, f"{prefix}06-editing-text", editing)

    for menu in ["File", "Edit", "Insert", "Slide", "View"]:
        def open_menu(p, m=menu):
            go(p, "highlights")
            p.click(f"#menubar [data-menu={m}]")
            p.wait_for_function("() => !document.querySelector('#ctxmenu').hidden")

        shot(page, f"{prefix}08-menu-{menu.lower()}", open_menu)

    def list_hover(p):
        p.hover("#slides li:nth-child(3) > button:first-child")
        p.wait_for_timeout(600)

    shot(page, f"{prefix}09-slide-list-hover", list_hover)

    shot(page, f"{prefix}10-slide-list-menu", lambda p: p.click("#slides li:nth-child(3) > button:first-child", button="right"))

    def dragging(p):
        src = p.locator("#slides li:nth-child(5) > button:first-child").bounding_box()
        dst = p.locator("#slides li:nth-child(2) > button:first-child").bounding_box()
        p.mouse.move(src["x"] + 20, src["y"] + src["height"] / 2)
        p.mouse.down()
        p.mouse.move(dst["x"] + 20, dst["y"] + 4, steps=10)
        p.wait_for_timeout(300)

    try:
        shot(page, f"{prefix}11-slide-list-drag", dragging)
    finally:
        page.mouse.up()

    def source_pane(p):
        go(p, "model")
        p.evaluate("import('/ui/keys.js').then((k) => k.run('toggle-source'))")  # View ▸ Source pane
        p.wait_for_function("() => !document.querySelector('#srcpane').hidden")
        p.wait_for_timeout(800)

    shot(page, f"{prefix}12-source-pane", source_pane)
    page.evaluate("import('/ui/keys.js').then((k) => k.run('toggle-source'))")  # back to closed

    def settings(p):
        p.keyboard.press("Control+,")
        p.wait_for_function("() => document.querySelector('#settings').open")

    shot(page, f"{prefix}13-settings", settings)

    def shortcuts(p):
        p.click("#slides li:nth-child(2) > button:first-child")
        p.keyboard.press("?")
        p.wait_for_function("() => document.querySelector('#shortcuts').open")

    shot(page, f"{prefix}14-shortcuts", shortcuts)

    for anchor, n in [("title-slide", "15-title-slide"), ("growth", "16-image"), ("columns", "17-columns"), ("model", "18-math-callout"), ("next", "20-empty-slide")]:
        shot(page, f"{prefix}{n}", lambda p, a=anchor: go(p, a))

    def box(p):
        go(p, "annotated")
        p.mouse.click(*in_slide(p, ".absolute"), button="right")
        p.keyboard.press("Escape")

    shot(page, f"{prefix}19-positioned-box", box)


with sync_playwright() as pw:
    browser = pw.chromium.launch(channel="chrome", headless=True)
    try:
        for prefix, link, size in [
            ("light-", framed("theme=light"), (1280, 800)),
            ("dark-", framed("theme=dark"), (1280, 800)),
            ("hc-", framed("theme=hc"), (1280, 800)),
        ]:
            page = browser.new_page(viewport={"width": size[0], "height": size[1]})
            open_editor(page, link)
            editor_shots(page, prefix)
            page.close()
        # a narrow window (the menus still fit at 420 px; the list becomes a strip)
        page = browser.new_page(viewport={"width": 420, "height": 760})
        open_editor(page, framed("theme=light"))
        shot(page, "narrow-01-start", lambda p: go(p, "highlights"))

        page.close()
        # VS Code's panel (framed)
        page = browser.new_page(viewport={"width": 900, "height": 760})
        open_editor(page, framed("host=vscode&theme=dark"))
        shot(page, "vscode-01-panel", lambda p: go(p, "highlights"))
        page.close()
    finally:
        browser.close()
        subprocess.run([QD, "stop"], cwd=deck, env=env, capture_output=True)

print(f"{len(list(SHOTS.glob('*.png')))} screenshots in {SHOTS}")
for f in failed:
    print("failed:", f)
