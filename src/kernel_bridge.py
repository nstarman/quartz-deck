"""qd's kernel bridge (spec 004 R4): run inside the deck's environment, start
a Jupyter kernel over IPC sockets, and talk to qd in JSON lines.

stdin:  {"op": "execute", "code": "..."} | {"op": "restart"} | {"op": "interrupt"}
stdout: {"ready": true}, then one reply per execute/restart (interrupt has none):
        {"status": "ok|error|stopped", "counter": n, "outputs": [...]} | {"restarted": true}
Outputs: {"kind": "text", "text"} | {"kind": "html", "html"} |
         {"kind": "image", "mime", "data" (base64)} | {"kind": "error", "ename", "evalue", "traceback"}
EOF on stdin shuts the kernel down.
"""

import json
import os
import queue
import re
import sys
import tempfile
import threading

from jupyter_client import KernelManager

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def say(obj):
    sys.stdout.write(json.dumps(obj) + "\n")
    sys.stdout.flush()


def bundle(data):
    """The richest output qd can show from a MIME bundle."""
    for mime in ("image/png", "image/jpeg", "image/svg+xml"):
        if mime in data:
            return {"kind": "image", "mime": mime, "data": data[mime]}
    if "text/html" in data:
        return {"kind": "html", "html": data["text/html"]}
    return {"kind": "text", "text": data.get("text/plain", "")}


def execute(kc, code):
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
        elif kind == "status" and c["execution_state"] == "idle":
            break
    for o in outputs:
        o.pop("stream", None)
    return {"status": status, "counter": counter, "outputs": outputs}


def main():
    km = KernelManager(kernel_name=os.environ.get("QD_KERNEL", "python3"), transport="ipc")
    km.ip = os.path.join(tempfile.mkdtemp(prefix="qd-kernel-"), "k")
    km.start_kernel()
    kc = km.client()
    kc.start_channels()
    kc.wait_for_ready(timeout=120)
    say({"ready": True})

    requests = queue.Queue()

    def read():
        for line in sys.stdin:
            req = json.loads(line)
            if req["op"] == "interrupt":
                km.interrupt_kernel()  # at once, even while a cell runs
            else:
                requests.put(req)
        requests.put(None)

    threading.Thread(target=read, daemon=True).start()
    while (req := requests.get()) is not None:
        if req["op"] == "execute":
            say(execute(kc, req["code"]))
        elif req["op"] == "restart":
            km.restart_kernel(now=True)
            kc.wait_for_ready(timeout=120)
            say({"restarted": True})
    kc.stop_channels()
    km.shutdown_kernel(now=True)


if __name__ == "__main__":
    main()
