"""qd's kernel bridge (spec 004 R4): run inside the deck's environment, start
a Jupyter kernel over IPC sockets, and talk to qd in JSON lines.

stdin:  {"op": "execute", "code": "..."} | {"op": "restart"} | {"op": "interrupt"}
stdout: {"ready": true} (or {"error": "..."} when the kernel can't start), then
        one reply per execute/restart (interrupt has none):
        {"status": "ok|error|stopped", "counter": n, "outputs": [...]} | {"restarted": true}
Outputs: {"kind": "text", "text"} | {"kind": "html", "html"} |
         {"kind": "image", "mime", "path" (a file in a temporary folder)} | {"kind": "error", "ename", "evalue", "traceback"}
EOF on stdin shuts the kernel down.
"""

import base64
import json
import os
import queue
import re
import sys
import tempfile
import threading

from jupyter_client import KernelManager
from jupyter_client.kernelspec import KernelSpecManager

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def say(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


FIGURES = tempfile.mkdtemp(prefix="qd-figures-")
EXT = {"image/png": "png", "image/jpeg": "jpg", "image/svg+xml": "svg"}


def bundle(data):
    """The richest output qd can show from a MIME bundle; figures go to files."""
    for mime, ext in EXT.items():
        if mime in data:
            fd, path = tempfile.mkstemp(suffix="." + ext, dir=FIGURES)
            raw = data[mime]
            with os.fdopen(fd, "wb") as f:
                f.write(raw.encode() if ext == "svg" else base64.b64decode(raw))
            return {"kind": "image", "mime": mime, "path": path}
    if "text/html" in data:
        return {"kind": "html", "html": data["text/html"]}
    return {"kind": "text", "text": data.get("text/plain", "")}


def execute(kc, km, code, stop):
    msg_id = kc.execute(code, store_history=True)
    outputs, status, counter = [], "ok", None
    while True:
        msg = kc.get_iopub_msg()
        if msg["parent_header"].get("msg_id") != msg_id:
            continue
        kind, c = msg["msg_type"], msg["content"]
        if kind == "stream":
            if outputs and outputs[-1]["kind"] == "text" and outputs[-1].get("stream"):
                outputs[-1]["text"] += c["text"]
            else:
                outputs.append({"kind": "text", "text": c["text"], "stream": True})
        elif kind in ("execute_result", "display_data"):
            outputs.append(bundle(c["data"]))
        elif kind == "error":
            status = "stopped" if c["ename"] == "KeyboardInterrupt" else "error"
            tb = [ANSI.sub("", line) for line in c["traceback"]]
            outputs.append(
                {"kind": "error", "ename": c["ename"], "evalue": c["evalue"], "traceback": tb}
            )
        elif kind == "execute_input":
            counter = c.get("execution_count")
            # an interrupt sent before the cell started fell on an idle kernel, which ignores it
            if stop.is_set():
                km.interrupt_kernel()
        elif kind == "status" and c["execution_state"] == "idle":
            break
    for o in outputs:
        o.pop("stream", None)
    return {"status": status, "counter": counter, "outputs": outputs}


def kernel_name():
    """QD_KERNEL, else the first installed kernel for QD_LANGUAGE (Quarto's rule)."""
    specs = KernelSpecManager().get_all_specs()
    name, lang = os.environ.get("QD_KERNEL"), os.environ.get("QD_LANGUAGE", "python")
    if name:
        return name if name in specs else None
    return next((k for k, s in sorted(specs.items()) if s["spec"]["language"].lower() == lang), None)


def main():
    name = kernel_name()
    if name is None:
        found = ", ".join(sorted(KernelSpecManager().get_all_specs())) or "none"
        wanted = os.environ.get("QD_KERNEL") or "for " + os.environ.get("QD_LANGUAGE", "python")
        say({"error": f"no Jupyter kernel {wanted} is installed (found: {found})"})
        return
    km = KernelManager(kernel_name=name, transport="ipc")
    km.ip = os.path.join(tempfile.mkdtemp(prefix="qd-kernel-"), "k")
    km.start_kernel()
    kc = km.client()
    kc.start_channels()
    kc.wait_for_ready(timeout=120)
    say({"ready": True})

    requests = queue.Queue()
    running, stop = threading.Event(), threading.Event()  # a cell is asked for / to stop

    def read():
        for line in sys.stdin:
            req = json.loads(line)
            if req["op"] == "interrupt":
                if running.is_set():
                    stop.set()
                km.interrupt_kernel()  # at once, even while a cell runs
            else:
                if req["op"] == "execute":
                    running.set()  # from here: it may wait in the queue
                requests.put(req)
        requests.put(None)

    threading.Thread(target=read, daemon=True).start()
    while (req := requests.get()) is not None:
        if req["op"] == "execute":
            out = execute(kc, km, req["code"], stop)
            running.clear()  # before the reply: the next cell may follow at once
            stop.clear()
            say(out)
        elif req["op"] == "restart":
            km.restart_kernel(now=True)
            kc.wait_for_ready(timeout=120)
            say({"restarted": True})
    kc.stop_channels()
    km.shutdown_kernel(now=True)


if __name__ == "__main__":
    main()
