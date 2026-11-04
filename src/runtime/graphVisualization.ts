import type { VizScalar } from "./visualization";

export const GRAPH_PROTOCOL_MISMATCH = "有向图可视化协议版本不一致。请重新编译并重启桌面应用，再重新运行代码。";
export const LEGACY_GRAPH_ANNOTATIONS = ["distances", "visited", "predecessors"] as const;

export interface GraphConfig { nodeStates?: string; edgeStates?: string }
export type GraphAnnotationStatus = "off" | "ready" | "unbound" | "invalid";
export type GraphMembers = Record<string, VizScalar>;
export interface GraphNode { id: string; value: VizScalar; state: GraphMembers }
export interface GraphEdge { id: string; source: string; target: string; weight: VizScalar; state: GraphMembers }
export interface GraphState {
  objectId: string;
  directed: true;
  nodes: GraphNode[];
  edges: GraphEdge[];
  annotations: Record<"nodeStates" | "edgeStates", GraphAnnotationStatus>;
}

const object = (v: unknown): v is Record<string, any> => v !== null && typeof v === "object" && !Array.isArray(v);
const id = (v: unknown): v is string => typeof v === "string" && v.length > 0 && v.length <= 256;
const integer = (v: unknown) => typeof v === "number" && Number.isSafeInteger(v) || object(v) && v.kind === "integer";
const numeric = (v: unknown) => typeof v === "number" && Number.isFinite(v) || object(v) && v.kind === "integer";
const key = (v: any) => typeof v === "string" ? `s:${v}` : `i:${typeof v === "number" ? String(v) : v.decimal === "-0" ? "0" : v.decimal}`;

/** Validate graph references without imposing tree constraints on cycles or self-loops. */
export function validateGraphState(s: unknown, config: GraphConfig, scalar: (v: unknown) => boolean): asserts s is GraphState {
  if (object(s) && object(s.annotations) && LEGACY_GRAPH_ANNOTATIONS.some(name => Object.prototype.hasOwnProperty.call(s.annotations, name))) throw new Error(GRAPH_PROTOCOL_MISMATCH);
  if (!object(s) || !id(s.objectId) || s.directed !== true || !Array.isArray(s.nodes) || s.nodes.length > 128 || !Array.isArray(s.edges) || s.edges.length > 512 || !object(s.annotations) || Object.keys(s.annotations).length !== 2) throw new Error("Invalid directed graph snapshot.");
  for (const name of ["nodeStates", "edgeStates"] as const) {
    const status = s.annotations[name];
    if (!["off", "ready", "unbound", "invalid"].includes(status) || (status === "off") !== (config[name] === undefined)) throw new Error("Invalid graph annotation status.");
  }
  const members = (v: unknown, status: GraphAnnotationStatus) => object(v) && Object.entries(v).every(([k, value]) => k.length > 0 && Array.from(k).length <= 256 && scalar(value)) && (status === "ready" || Object.keys(v).length === 0);
  const ids = new Set<string>(), keys = new Set<string>();
  for (const n of s.nodes) {
    if (!object(n) || !id(n.id) || ids.has(n.id) || !scalar(n.value) || !(typeof n.value === "string" || integer(n.value)) || keys.has(key(n.value)) || !members(n.state, s.annotations.nodeStates)) throw new Error("Invalid graph node.");
    ids.add(n.id); keys.add(key(n.value));
  }
  const edgeIds = new Set<string>(), pairs = new Set<string>();
  for (const e of s.edges) {
    if (!object(e) || !id(e.id) || edgeIds.has(e.id) || !ids.has(e.source) || !ids.has(e.target) || !scalar(e.weight) || !(e.weight === null || numeric(e.weight)) || !members(e.state, s.annotations.edgeStates)) throw new Error("Invalid directed graph edge.");
    const pair = JSON.stringify([e.source, e.target]);
    if (pairs.has(pair)) throw new Error("Duplicate directed graph edge.");
    edgeIds.add(e.id); pairs.add(pair);
  }
}
