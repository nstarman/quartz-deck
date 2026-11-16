-- dot-image (quartz-deck spec 015 US3): pictures in the boxes of a {dot}
-- diagram. Quarto's Graphviz ignores a node's image="…", so this filter
-- reads the deck's own source, finds each {dot} cell's boxes with an image,
-- and draws the picture into the rendered SVG at the box's left, over the
-- empty spacer cell that qd writes into the label:
--   label=<<TABLE …><TR><TD WIDTH="28" HEIGHT="28" FIXEDSIZE="TRUE"></TD><TD>Text</TD></TR></TABLE>>
-- The picture is an <image class="dot-image">, so the deck's CSS can style it.

local warned = {}
local function warn(msg)
  if not warned[msg] then
    warned[msg] = true
    quarto.log.warning('dot-image: ' .. msg)
  end
end

--- The {dot} cells with pictures, in order: { {name = {path, size}} }.
local function scan(src)
  local cells = {}
  for body in ('\n' .. src):gmatch('\n```+%s*{dot}[^\n]*\n(.-)\n```') do
    local boxes = {}
    local function box(at, name, list)
      -- `a -> b [ … ]` is an arrow: its attributes carry no picture
      if body:sub(1, at - 1):match('%-[%->]%s*$') then return end
      local path = list:match('[%[,;%s]image%s*=%s*"([^"]*)"') or list:match('[%[,;%s]image%s*=%s*([%w_%./%-]+)')
      if path and path ~= '' then
        boxes[name] = { path = path, size = tonumber(list:match('<TD WIDTH="(%d+)"')) }
      end
    end
    for at, name, list in body:gmatch('()"([^"]+)"%s*(%b[])') do box(at, name, list) end
    for at, name, list in body:gmatch('()([%w_%.]+)%s*(%b[])') do box(at, name, list) end
    if next(boxes) then cells[#cells + 1] = boxes end
  end
  return cells
end

--- The numbers of a shape's `points`, `d` or ellipse: its box {x0, y0, x1, y1}.
local function bounds(shape)
  local xs, ys = {}, {}
  local pts = shape:match(' points="([^"]+)"') or shape:match(' d="([^"]+)"')
  if pts then
    for x, y in pts:gmatch('(%-?[%d%.]+),(%-?[%d%.]+)') do
      xs[#xs + 1], ys[#ys + 1] = tonumber(x), tonumber(y)
    end
  else
    local cx, cy = tonumber(shape:match(' cx="([^"]+)"')), tonumber(shape:match(' cy="([^"]+)"'))
    local rx, ry = tonumber(shape:match(' rx="([^"]+)"')), tonumber(shape:match(' ry="([^"]+)"'))
    if not (cx and cy and rx and ry) then return nil end
    xs, ys = { cx - rx, cx + rx }, { cy - ry, cy + ry }
  end
  if #xs == 0 then return nil end
  return { math.min(table.unpack(xs)), math.min(table.unpack(ys)), math.max(table.unpack(xs)), math.max(table.unpack(ys)) }
end

local function attr(s) return (s:gsub('&', '&amp;'):gsub('"', '&quot;'):gsub('<', '&lt;'):gsub('>', '&gt;')) end

--- Where the node `name`'s `<title>` ends in `svg` (Graphviz writes `-` as `&#45;`).
local function title(svg, name)
  local esc = attr(name)
  local _, at = svg:find('<title>' .. esc .. '</title>', 1, true)
  if not at then _, at = svg:find('<title>' .. esc:gsub('%-', '&#45;') .. '</title>', 1, true) end
  return at
end

--- `svg` with the picture of each box in `boxes` drawn into its node.
local function draw(svg, boxes, dir)
  for name, b in pairs(boxes) do
    local file = pandoc.path.join({ dir, b.path }) -- an absolute path stays as it is
    local f = io.open(file, 'r')
    if not f then
      warn('no file ' .. b.path .. ' for the box ' .. name)
      goto next
    end
    f:close()
    do
      local at = title(svg, name)
      if not at then goto next end
      local stop = svg:find('</g>', at, true)
      local node = svg:sub(at + 1, stop - 1)
      local shape = node:match('<polygon[^>]*>') or node:match('<path[^>]*>') or node:match('<ellipse[^>]*>')
      local box = shape and bounds(shape)
      if not box then goto next end
      local h = box[4] - box[2]
      local s = b.size or (h - 16)
      -- at the spacer: just left of a label's text that starts after it, else at the box's left
      local tx = node:match('<text text%-anchor="start" x="(%-?[%d%.]+)"')
      local x = tx and (tonumber(tx) - 4 - s) or (box[1] + 4)
      local y = (box[2] + box[4]) / 2 - s / 2
      local img = string.format(
        '<image class="dot-image" href="%s" xlink:href="%s" x="%.2f" y="%.2f" width="%.2f" height="%.2f" preserveAspectRatio="xMidYMid slice"/>\n',
        attr(b.path), attr(b.path), x, y, s, s)
      -- after the shape, before the label, so the text stays on top
      local t = node:find('<text', 1, true)
      local insert = at + (t or #node + 1)
      svg = svg:sub(1, insert - 1) .. img .. svg:sub(insert)
    end
    ::next::
  end
  return svg
end

function Pandoc(doc)
  local input = quarto.doc.input_file
  local f = input and io.open(input, 'r')
  if not f then return nil end
  local src = f:read('a')
  f:close()
  if src:find('{{<%s*include') then
    warn('cells pulled in with {{< include >}} are not scanned for pictures')
  end
  local cells = scan(src)
  local dir = pandoc.path.directory(input)
  -- a diagram takes the first cell, in order, whose picture boxes it has: a
  -- diagram the scan can't see (an include, pasted SVG) is passed over
  local used = {}
  return doc:walk({
    RawBlock = function(r)
      if r.format ~= 'html' or not r.text:find('<svg') or not r.text:find('class="graph"', 1, true) then return nil end
      for i, boxes in ipairs(cells) do
        local fits = not used[i]
        for name in pairs(boxes) do fits = fits and title(r.text, name) ~= nil end
        if fits then
          used[i] = true
          r.text = draw(r.text, boxes, dir)
          return r
        end
      end
    end,
  })
end
