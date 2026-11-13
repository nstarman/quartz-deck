# qd's R bridge (spec 013 R4): run a knitr deck's cells in one R session,
# talking to qd in JSON lines, like kernel_bridge.py.
#
# stdin:  {"op": "execute", "code": "..."} | {"op": "restart"}
#         (an interrupt is SIGINT, sent only while a cell runs)
# stdout: {"ready": true} (or {"error": "..."}), then one reply per request:
#         {"status": "ok|error|stopped", "counter": n, "outputs": [...]} | {"restarted": true}
# Outputs: {"kind": "text", "text"} | {"kind": "image", "mime", "path"} |
#          {"kind": "error", "ename", "evalue", "traceback"}
# QD_FIG is "width height dpi" (inches), the deck's figure size. EOF ends it.

missing <- Filter(function(p) !requireNamespace(p, quietly = TRUE), c("evaluate", "jsonlite"))
if (length(missing)) {
  cat(sprintf(paste0('{"error": "r cells: the R package %s is missing; ',
                     'install.packages(c(\\"knitr\\", \\"rmarkdown\\"))"}\n'),
              paste(missing, collapse = ", ")))
  quit(save = "no")
}

say <- function(x) {
  cat(jsonlite::toJSON(x, auto_unbox = TRUE, null = "null"), "\n", sep = "")
  flush(stdout())
}

fig <- as.numeric(strsplit(Sys.getenv("QD_FIG", "10 5 96"), " ")[[1]])
figures <- tempfile("qd-figures-")
dir.create(figures)
counter <- 0L

run <- function(code) {
  out <- list()
  status <- "ok"
  add <- function(o) out[[length(out) + 1]] <<- o
  text <- function(t) {
    n <- length(out)
    if (n && out[[n]]$kind == "text") out[[n]]$text <<- paste0(out[[n]]$text, t)
    else add(list(kind = "text", text = t))
  }
  res <- tryCatch(
    evaluate::evaluate(code, envir = globalenv(), stop_on_error = 1L, new_device = TRUE),
    interrupt = function(e) { status <<- "stopped"; list() }
  )
  for (r in res) {
    if (inherits(r, "source")) next
    if (is.character(r)) {
      text(r)
    } else if (inherits(r, "recordedplot")) {
      f <- tempfile(fileext = ".png", tmpdir = figures)
      grDevices::png(f, width = fig[1], height = fig[2], units = "in", res = fig[3])
      grDevices::replayPlot(r)
      grDevices::dev.off()
      add(list(kind = "image", mime = "image/png", path = f))
    } else if (inherits(r, "error")) {
      status <- "error"
      m <- conditionMessage(r)
      add(list(kind = "error", ename = "Error", evalue = m, traceback = list(paste("Error:", m))))
    } else if (inherits(r, "condition")) {
      label <- if (inherits(r, "warning")) "Warning: " else ""
      text(paste0(label, conditionMessage(r), if (inherits(r, "message")) "" else "\n"))
    }
  }
  if (status == "stopped") {
    add(list(kind = "error", ename = "Interrupted", evalue = "", traceback = list("Interrupted")))
  }
  list(status = status, outputs = out)
}

con <- file("stdin", "r")
say(list(ready = TRUE))
repeat {
  line <- readLines(con, n = 1)
  if (!length(line)) break
  req <- jsonlite::fromJSON(line)
  if (req$op == "execute") {
    counter <- counter + 1L
    r <- run(req$code)
    say(list(status = r$status, counter = counter, outputs = r$outputs))
  } else if (req$op == "restart") {
    rm(list = ls(globalenv(), all.names = TRUE), envir = globalenv())
    counter <- 0L
    say(list(restarted = TRUE))
  }
}
