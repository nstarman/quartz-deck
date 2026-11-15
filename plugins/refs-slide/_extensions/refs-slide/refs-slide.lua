-- refs-slide (quartz-deck, spec 014): the References slide in the style, columns and size chosen
-- on its #refs div: ::: {#refs refs-style="compact" refs-columns="3" refs-size="smaller"}.
-- Like cite-links, it runs citeproc itself, once (research R1); list it after cite-links.

local function str(v)
  return v and pandoc.utils.stringify(v) or ""
end

-- The step shared with cite-links (the same code in both, so each works alone): run citeproc
-- once, and keep what it read in meta["qd-citeproc"] so either filter can look entries up later.
local function citeproc_once(doc, default_csl)
  if doc.meta["qd-citeproc"] or not (doc.meta.bibliography or doc.meta.references) then
    return doc
  end
  doc.meta.csl = doc.meta.csl or default_csl
  doc.meta["link-citations"] = true
  local read = { bibliography = doc.meta.bibliography, references = doc.meta.references, csl = doc.meta.csl }
  -- with the list suppressed citeproc links nothing; keep its links, drop the list after
  local suppress = doc.meta["suppress-bibliography"]
  doc.meta["suppress-bibliography"] = nil
  doc = pandoc.utils.citeproc(doc)
  if suppress then
    doc = doc:walk({ Div = function(d) if d.identifier == "refs" then return {} end end })
  end
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

local STYLES = { compact = true, short = true, full = true }
local SIZES = { normal = true, smaller = true, smallest = true }

-- "A", "A & B", "A, B & C", "A et al."
local function authors(ref)
  local names = {}
  for _, a in ipairs(ref.author or {}) do
    names[#names + 1] = str(a.family) ~= "" and str(a.family) or str(a.literal)
  end
  if #names == 0 then
    return str(ref.title)
  elseif #names == 1 then
    return names[1]
  elseif #names > 3 then
    return names[1] .. " et al."
  end
  return table.concat(names, ", ", 1, #names - 1) .. " & " .. names[#names]
end

local function year(ref)
  local ok, y = pcall(function()
    return ref.issued["date-parts"][1][1]
  end)
  return ok and y and tostring(y) or "n.d."
end

-- Each key's narrative citation, as citeproc renders it for this deck: the compact labels,
-- suffixes included (research R3).
local function labels(doc, keys)
  local read = doc.meta["qd-citeproc"]
  local cites = {}
  for _, k in ipairs(keys) do
    cites[#cites + 1] = pandoc.Cite({}, { pandoc.Citation(k, "AuthorInText") })
    cites[#cites + 1] = pandoc.Space()
  end
  local meta = pandoc.Meta({
    bibliography = read.bibliography,
    references = read.references,
    csl = read.csl,
    ["suppress-bibliography"] = true,
  })
  local out = {}
  pandoc.utils.citeproc(pandoc.Pandoc({ pandoc.Para(cites) }, meta)):walk({
    Cite = function(c)
      out[c.citations[1].id] = c.content:walk({ Link = function(l) return l.content end })
    end,
  })
  return out
end

-- citeproc's entry div `ref-KEY`: its key
local function key_of(e)
  return e.t == "Div" and e.identifier:match("^ref%-(.+)$") or nil
end

function Pandoc(doc)
  doc = citeproc_once(doc)
  local refs = references(doc)
  doc = doc:walk({
    Div = function(d)
      if d.identifier ~= "refs" then
        return nil
      end
      local a = d.attributes
      local style = STYLES[a["refs-style"]] and a["refs-style"] or "compact"
      local cols = math.max(1, math.min(3, tonumber(a["refs-columns"]) or 3))
      local size = SIZES[a["refs-size"]] and a["refs-size"] or "smaller"
      a["refs-style"], a["refs-columns"], a["refs-size"] = nil, nil, nil
      d.classes = pandoc.List({ "refs-slide", "refs-" .. style, "refs-cols-" .. cols, "refs-size-" .. size })
        .. d.classes
      if style == "full" then
        return d
      end
      local keys = d.content:filter(key_of):map(key_of)
      local label = style == "compact" and labels(doc, keys) or {}
      d.content = d.content:map(function(e)
        local key = key_of(e)
        local ref = key and refs[key]
        if not ref then
          return e
        end
        local href = target(ref)
        local inlines
        if style == "compact" then
          inlines = pandoc.Inlines(label[key] or { pandoc.Str(key) })
          if href then
            local tip = str(inlines):gsub("[()]", "") .. (str(ref.title) ~= "" and (": " .. str(ref.title)) or "")
            local attr = pandoc.Attr("", { "cite-link" }, { ["aria-label"] = tip })
            inlines = { pandoc.Link(pandoc.Inlines({ icon_span(), pandoc.Space() }) .. inlines, href, tip, attr) }
          end
        else
          inlines = pandoc.Inlines({ pandoc.Str(authors(ref) .. " (" .. year(ref) .. ")") })
          if href then
            inlines = { pandoc.Link(inlines, href) }
          end
          local venue = str(ref["container-title"])
          if venue ~= "" then
            inlines = pandoc.Inlines(inlines) .. { pandoc.Str(","), pandoc.Space(), pandoc.Emph(venue) }
          end
        end
        e.content = { pandoc.Plain(inlines) }
        return e
      end)
      return d
    end,
  })
  -- a Cite left for Quarto's own citeproc pass would be rendered again: leave spans
  doc = doc:walk({
    Cite = function(c)
      return pandoc.Span(c.content, pandoc.Attr("", { "citation" }, {
        ["data-cites"] = table.concat(c.citations:map(function(ci) return ci.id end), " "),
      }))
    end,
  })
  quarto.doc.add_html_dependency({ name = "refs-slide", version = "1.0.0", stylesheets = { "refs-slide.css" } })
  return doc
end
