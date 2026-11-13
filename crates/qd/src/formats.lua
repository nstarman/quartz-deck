-- qd export --to (spec 011, research R2): a slide deck written for reveal.js,
-- adapted for other formats. Pauses only mean something in Beamer, notes have
-- their own place only in PowerPoint and Beamer, and fragments and placed
-- boxes are reveal.js layout: their content stays, in the flow.
local beamer = FORMAT == "beamer"
local own_notes = beamer or FORMAT == "pptx"

local function is_pause(b)
  return b.t == "Para" and pandoc.utils.stringify(b) == ". . ."
end

function Blocks(bs)
  if beamer then return nil end
  return bs:filter(function(b) return not is_pause(b) end)
end

function Div(d)
  if d.classes:includes("notes") and not own_notes then
    local c = pandoc.Blocks { pandoc.Para { pandoc.Strong { pandoc.Str("Speaker"), pandoc.Space(), pandoc.Str("notes") } } }
    c:extend(d.content)
    return pandoc.BlockQuote(c)
  end
  if d.classes:includes("fragment") or d.classes:includes("absolute") then
    return d.content
  end
end
