// Snap guides (spec 009 US4), shared by placed boxes on a slide and boxes in
// a diagram (spec 015): a dragged box's edges and centre catch on other
// boxes' lines within a few screen pixels.

/** How near, in screen px, a line catches. */
export const SNAP_PX = 6;

/** The nearest line to any of `edges` within `reach`: [shift, line], or null. */
export function nearest(edges, lines, reach) {
  let best = null;
  for (const e of edges) for (const x of lines) if (Math.abs(x - e) <= reach && (!best || Math.abs(x - e) < Math.abs(best[0]))) best = [x - e, x];
  return best;
}
