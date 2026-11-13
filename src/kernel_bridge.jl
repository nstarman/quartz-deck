# qd's Julia bridge (spec 013 R10): run an `engine: julia` deck's cells in one
# Julia session, talking to qd in JSON lines, like kernel_bridge.py and .R.
# Base Julia only (no JSON package): requests are read with a pattern, replies
# written by hand.
#
# stdin:  {"op": "execute", "code": "..."} | {"op": "restart"}
#         (an interrupt is SIGINT, sent only while a cell runs)
# stdout: {"ready": true}, then one reply per request:
#         {"status": "ok|error|stopped", "counter": n, "outputs": [...]} | {"restarted": true}
# Outputs: {"kind": "text", "text"} | {"kind": "html", "html"} |
#          {"kind": "image", "mime", "path"} | {"kind": "error", "ename", "evalue", "traceback"}

Base.exit_on_sigint(false)  # SIGINT stops the cell (InterruptException), not Julia

function js(s::AbstractString)
    io = IOBuffer()
    print(io, '"')
    for c in s
        if c == '"' || c == '\\'
            print(io, '\\', c)
        elseif c == '\n'
            print(io, "\\n")
        elseif c < ' '
            print(io, "\\u", string(UInt16(c), base = 16, pad = 4))
        else
            print(io, c)
        end
    end
    print(io, '"')
    String(take!(io))
end
obj(pairs) = "{" * join(["$(js(k)): $v" for (k, v) in pairs], ", ") * "}"
say(s) = (println(stdout, s); flush(stdout))

const FIGURES = mktempdir(; prefix = "qd-figures-", cleanup = false)

# what `display` shows during a cell, richest MIME first (as the Python bridge)
struct Capture <: AbstractDisplay
    out::Vector{String}
end
function Base.display(d::Capture, x)
    for (mime, ext) in (("image/png", "png"), ("image/svg+xml", "svg"))
        if showable(mime, x)
            path = joinpath(FIGURES, "$(time_ns()).$ext")
            open(io -> show(io, MIME(mime), x), path, "w")
            return push!(d.out, obj(["kind" => js("image"), "mime" => js(mime), "path" => js(path)]))
        end
    end
    if showable("text/html", x)
        return push!(d.out, obj(["kind" => js("html"), "html" => js(repr("text/html", x))]))
    end
    push!(d.out, obj(["kind" => js("text"), "text" => js(repr("text/plain", x) * "\n")]))
end

deck = Module(:Deck)
counter = 0

function run(code)
    out = String[]
    status = "ok"
    cap = Capture(out)
    pushdisplay(cap)
    printed = mktemp() do path, io
        try
            redirect_stdout(io) do
                v = include_string(deck, code, "cell")
                # the last value, as a notebook shows it (not after a `;`)
                v === nothing || endswith(rstrip(code), ';') || display(cap, v)
            end
        catch e
            m = e isa LoadError ? e.error : e  # include_string wraps what the cell threw
            status = m isa InterruptException ? "stopped" : "error"
            msg = sprint(showerror, m)
            push!(out, obj(["kind" => js("error"), "ename" => js(string(nameof(typeof(m)))),
                            "evalue" => js(msg), "traceback" => "[" * js(msg) * "]"]))
        end
        close(io)
        read(path, String)
    end
    popdisplay(cap)
    isempty(printed) || pushfirst!(out, obj(["kind" => js("text"), "text" => js(printed)]))
    status, out
end

say(obj(["ready" => "true"]))
for line in eachline(stdin)
    global deck, counter
    op = match(r"\"op\"\s*:\s*\"(\w+)\"", line)
    op === nothing && continue
    if op[1] == "execute"
        m = match(r"\"code\"\s*:\s*\"((?:[^\"\\]|\\.)*)\"", line)
        counter += 1
        status, out = run(m === nothing ? "" : unescape_string(m[1]))
        say(obj(["status" => js(status), "counter" => string(counter), "outputs" => "[" * join(out, ", ") * "]"]))
    elseif op[1] == "restart"
        deck = Module(:Deck)
        counter = 0
        say(obj(["restarted" => "true"]))
    end
end
