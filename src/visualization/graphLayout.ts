import type { GraphState } from "../runtime/graphVisualization";

interface Point { x: number; y: number }
export const GRAPH_NODE_RADIUS = 34;
export const GRAPH_NODE_CLEARANCE = 48;
export const GRAPH_ARROW_LENGTH = 12;
export const GRAPH_ARROW_HALF_WIDTH = 6;

function arrow(tip: Point, control: Point) {
  const length = Math.hypot(tip.x - control.x, tip.y - control.y);
  const ux = (tip.x - control.x) / length, uy = (tip.y - control.y) / length;
  const base = { x: tip.x - ux * GRAPH_ARROW_LENGTH, y: tip.y - uy * GRAPH_ARROW_LENGTH };
  const left = { x: base.x - uy * GRAPH_ARROW_HALF_WIDTH, y: base.y + ux * GRAPH_ARROW_HALF_WIDTH };
  const right = { x: base.x + uy * GRAPH_ARROW_HALF_WIDTH, y: base.y - ux * GRAPH_ARROW_HALF_WIDTH };
  return { base, tip, left, right };
}

function evaluate(points: Point[], t: number): Point {
  let values = points;
  while (values.length > 1) values = values.slice(1).map((p, i) => ({
    x: values[i].x * (1 - t) + p.x * t,
    y: values[i].y * (1 - t) + p.y * t,
  }));
  return values[0];
}

/** One filled outline: two Bezier rails and the head share the same boundary. */
export function graphArrowOutline(edge: { curve: Point[]; left: Point; tip: Point; right: Point }, width: number) {
  const degree = edge.curve.length - 1;
  const derivative = edge.curve.slice(1).map((p, i) => ({ x: degree * (p.x - edge.curve[i].x), y: degree * (p.y - edge.curve[i].y) }));
  const acceleration = derivative.slice(1).map((p, i) => ({ x: (degree - 1) * (p.x - derivative[i].x), y: (degree - 1) * (p.y - derivative[i].y) }));
  // A Bezier offset is generally not itself a Bezier. Short cubic Hermite
  // segments follow the offset while preserving the exact terminal tangent.
  const rail = (offset: number) => Array.from({ length: 9 }, (_, i) => {
    const t = i / 8, p = evaluate(edge.curve, t), d = evaluate(derivative, t), dd = evaluate(acceleration, t);
    const speed = Math.hypot(d.x, d.y), dot = d.x * dd.x + d.y * dd.y;
    return {
      t,
      x: p.x - d.y / speed * offset,
      y: p.y + d.x / speed * offset,
      dx: d.x + (-dd.y / speed + d.y * dot / speed ** 3) * offset,
      dy: d.y + (dd.x / speed - d.x * dot / speed ** 3) * offset,
    };
  });
  const left = rail(width / 2), right = rail(-width / 2).reverse();
  const segments = (points: typeof left) => points.slice(1).map((p, i) => {
    const a = points[i], step = (p.t - a.t) / 3;
    return `C ${a.x + a.dx * step} ${a.y + a.dy * step}, ${p.x - p.dx * step} ${p.y - p.dy * step}, ${p.x} ${p.y}`;
  }).join(" ");
  return `M ${left[0].x} ${left[0].y} ${segments(left)} L ${edge.left.x} ${edge.left.y} L ${edge.tip.x} ${edge.tip.y} L ${edge.right.x} ${edge.right.y} L ${right[0].x} ${right[0].y} ${segments(right)} Z`;
}

/** Stable centerlines; annotations and highlighting do not move nodes or tips. */
export function layoutGraph(state: GraphState) {
  const size = Math.max(560, state.nodes.length * 140 / Math.PI + 260);
  const center = size / 2, radius = (size - 260) / 2;
  const nodes = state.nodes.map((node, i) => {
    const angle = -Math.PI / 2 + i * Math.PI * 2 / state.nodes.length;
    return { id: node.id, x: state.nodes.length === 1 ? center : center + radius * Math.cos(angle), y: state.nodes.length === 1 ? center : center + radius * Math.sin(angle) };
  });
  const positions = new Map(nodes.map(n => [n.id, n]));
  const pairs = new Set(state.edges.map(e => JSON.stringify([e.source, e.target])));
  const edges = state.edges.map(edge => {
    const a = positions.get(edge.source)!, b = positions.get(edge.target)!;
    if (a.id === b.id) {
      const diagonal = GRAPH_NODE_CLEARANCE / Math.SQRT2;
      const start = { x: a.x + diagonal, y: a.y - diagonal };
      const control = { x: a.x - 120, y: a.y - 120 };
      const head = arrow({ x: a.x - diagonal, y: a.y - diagonal }, control);
      const firstControl = { x: a.x + 120, y: a.y - 120 };
      return { id: edge.id, path: `M ${start.x} ${start.y} C ${firstControl.x} ${firstControl.y}, ${control.x} ${control.y}, ${head.base.x} ${head.base.y}`, ...head, start, control, curve: [start, firstControl, control, head.base], x: a.x, y: a.y - 104 };
    }
    const length = Math.hypot(b.x - a.x, b.y - a.y);
    const ux = (b.x - a.x) / length, uy = (b.y - a.y) / length;
    const curve = pairs.has(JSON.stringify([edge.target, edge.source])) ? 48 : 0;
    const control = { x: (a.x + b.x) / 2 - uy * curve, y: (a.y + b.y) / 2 + ux * curve };
    const fromLength = Math.hypot(control.x - a.x, control.y - a.y), toLength = Math.hypot(b.x - control.x, b.y - control.y);
    const start = { x: a.x + (control.x - a.x) / fromLength * GRAPH_NODE_CLEARANCE, y: a.y + (control.y - a.y) / fromLength * GRAPH_NODE_CLEARANCE };
    const tip = { x: b.x - (b.x - control.x) / toLength * GRAPH_NODE_CLEARANCE, y: b.y - (b.y - control.y) / toLength * GRAPH_NODE_CLEARANCE };
    const head = arrow(tip, control);
    return { id: edge.id, path: `M ${start.x} ${start.y} Q ${control.x} ${control.y}, ${head.base.x} ${head.base.y}`, ...head, start, control, curve: [start, control, head.base], x: (start.x + 2 * control.x + tip.x) / 4, y: (start.y + 2 * control.y + tip.y) / 4 - 8 };
  });
  return { size, nodes, edges };
}
