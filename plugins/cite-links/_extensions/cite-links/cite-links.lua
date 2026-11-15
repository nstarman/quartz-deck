-- cite-links (quartz-deck, spec 014): each citation links to its paper (DOI, else URL), with an
-- icon and "Label: Title" on hover; [[@key]]{.corner} is the icon alone, top right.
-- Quarto runs citeproc after every filter entry point, so this filter runs it itself, once,
-- and clears the references so Quarto's own pass has nothing left to do (research R1).

local function str(v)
  return v and pandoc.utils.stringify(v) or ""
end

-- The step shared with refs-slide (the same code in both, so each works alone): run citeproc
-- once, and keep what it read in meta["qd-citeproc"] so either filter can look entries up later.
local function citeproc_once(doc, default_csl)
  if doc.meta["qd-citeproc"] or not (doc.meta.bibliography or doc.meta.references) then
    return doc
  end
  doc.meta.csl = doc.meta.csl or default_csl
  doc.meta["link-citations"] = true
  local read = { bibliography = doc.meta.bibliography, references = doc.meta.references, csl = doc.meta.csl }
  doc = pandoc.utils.citeproc(doc)
  doc.meta.bibliography, doc.meta.references, doc.meta.nocite = nil, nil, nil
  doc.meta["qd-citeproc"] = pandoc.MetaMap(read)
  return doc
end

-- The entries citeproc_once read, by key.
local function references(doc)
  local read = doc.meta["qd-citeproc"]
  local by_key = {}
  if read then
    -- `references` returns cited entries only: cite them all
    local all = pandoc.Cite({ pandoc.Str("@*") }, { pandoc.Citation("*", "NormalCitation") })
    local meta = pandoc.Meta({ bibliography = read.bibliography, references = read.references, nocite = { all } })
    for _, r in ipairs(pandoc.utils.references(pandoc.Pandoc({}, meta))) do
      by_key[r.id] = r
    end
  end
  return by_key
end

-- The DOI, except arXiv's own (10.48550) when the entry has its page.
local function target(ref)
  local doi, url = str(ref.doi), str(ref.url)
  if doi ~= "" and not (doi:lower():find("^10%.48550/") and url ~= "") then
    return "https://doi.org/" .. doi
  end
  return url ~= "" and url or nil
end

local function icon_span()
  return pandoc.Span({}, { class = "cite-icon" })
end

-- "Dunn et al. (2021)" -> "Dunn et al. 2021"
local function label(inlines)
  return (str(inlines):gsub("[()]", ""))
end

function Pandoc(doc)
  doc = citeproc_once(doc, pandoc.MetaString(quarto.utils.resolve_path("talk.csl")))
  local opts = doc.meta["cite-links"] or {}
  local icon = opts.icon ~= false
  local to_refs = str(opts.link) == "references"
  local refs = references(doc)
  local warned = {}

  -- a link for one key around `content`, or nil (with a warning) when it has nowhere to go
  local function link(key, content)
    local ref = refs[key]
    local href = to_refs and ("#ref-" .. key) or (ref and target(ref))
    if not href then
      if not warned[key] then
        warned[key] = true
        quarto.log.warning("cite-links: no DOI or URL for '" .. key .. "'")
      end
      return nil
    end
    local title = label(content)
    if ref and str(ref.title) ~= "" then
      title = title .. ": " .. str(ref.title)
    end
    local inlines = pandoc.Inlines({})
    if icon then
      inlines:extend({ icon_span(), pandoc.Space() })
    end
    inlines:extend(content)
    return pandoc.Link(inlines, href, title, pandoc.Attr("", { "cite-link" }, { ["aria-label"] = title }))
  end

  -- citeproc's own link for a key (`#ref-KEY`): its key
  local function key_of(el)
    return el.t == "Link" and el.target:match("^#ref%-(.+)$")
  end

  local function plain(inlines)
    return inlines:walk({ Link = function(l) if key_of(l) then return l.content end end })
  end

  doc = doc:walk({
    Cite = function(c)
      local first = c.citations[1]
      if #c.citations == 1 and first.mode == "AuthorInText" then
        -- narrative: the whole "Name et al. (2021)" is the link, not only the year
        local content = plain(c.content)
        return link(first.id, content) or content
      end
      -- a span, not the Cite: Quarto's own citeproc pass would render a Cite again
      local ids = c.citations:map(function(ci) return ci.id end)
      local content = c.content:walk({
        Link = function(l)
          local key = key_of(l)
          if key then
            return link(key, l.content) or l.content
          end
        end,
      })
      return pandoc.Span(content, pandoc.Attr("", { "citation" }, { ["data-cites"] = table.concat(ids, " ") }))
    end,
    Span = function(s)
      if not s.classes:includes("corner") then
        return nil
      end
      local found
      s:walk({ Link = function(l) if not found and l.classes:includes("cite-link") then found = l end end })
      if found then
        found.content = { icon_span() }
        found.classes:insert("cite-corner")
        return found
      end
    end,
  })
  quarto.doc.add_html_dependency({ name = "cite-links", version = "1.0.0", stylesheets = { "cite-links.css" } })
  return doc
end
