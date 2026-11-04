import type { RichOutput } from "./runResult";
import { GRAPH_PROTOCOL_MISMATCH, LEGACY_GRAPH_ANNOTATIONS, validateGraphState } from "./graphVisualization";
import type { GraphConfig, GraphState } from "./graphVisualization";
import { validateDynamicTable } from "./dynamicTable";
import type { DynamicTableState, DynamicTableSnapshot } from "./dynamicTable";

export const VISUALIZATION_MIME = "application/vnd.liveleaf.visualization+json";
export type VizScalar = null | boolean | string | number | { kind: "integer"; decimal: string } | { kind: "float"; text: "nan" | "inf" | "-inf" };
export interface VisualizationConfig {
  type: "array" | "tree" | "graph" | "dynamic-table";
  watch: string;
  pointers: string[];
  scope: string;
  tree?: { valueField: string; childFields: string[] };
  graph?: GraphConfig;
  tables?: string[];
}
export interface VisualizationDiagnostic { code: string; message: string; attribute?: string; seq?: number }
export interface ArrayState { objectId: string; items: VizScalar[] }
export interface ArrayPointer { name: string; raw: VizScalar; index: number | null; status: "inBounds" | "outOfBounds" | "unbound" | "invalidType" }
export interface TreeNode { id: string; value: VizScalar; children: { field: string; index: number | null; nodeId: string | null }[] }
export interface TreeState { rootId: string | null; nodes: TreeNode[] }
export interface TreePointer { name: string; nodeId: string | null; status: "matched" | "null" | "outside" | "unbound" | "invalidType" }
export interface TracePosition { cellLine: number; functionName: string; frameId: string; scopeFrameId: string; documentLine?: number }
interface FrameBase { seq: number; event: "call" | "line" | "return" | "exception"; position: TracePosition; status: "ready" | "unbound" | "invalid"; diagnosticCodes: string[]; tables?: DynamicTableSnapshot[] }
export type VisualizationFrame = FrameBase & (
  { type: "array"; state: ArrayState | null; pointers: ArrayPointer[] } |
  { type: "tree"; state: TreeState | null; pointers: TreePointer[] } |
  { type: "graph"; state: GraphState | null; pointers: TreePointer[] } |
  { type: "dynamic-table"; state: DynamicTableState | null; pointers: [] }
);
export interface VisualizationTrace {
  schemaVersion: 1; runId: string; cellId: string; config: VisualizationConfig; frames: VisualizationFrame[];
  capture: { status: "complete" | "truncated" | "error"; reason?: string };
  diagnostics: VisualizationDiagnostic[];
}

const attributes = ["visualize", "watch", "pointers", "scope", "value", "children", "node_states", "edge_states", "tables", "distances", "visited", "predecessors"];
const keywords = new Set("False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield".split(" "));
export const isIdentifier = (s: string) => /^[A-Za-z_][A-Za-z0-9_]*$/.test(s) && !keywords.has(s);

export function parseVisualization(language: string, attrs: Record<string, string>, duplicates: string[], flags: Set<string> = new Set()): { visualization?: VisualizationConfig; visualizationDiagnostics?: VisualizationDiagnostic[] } {
  if (!attributes.some(a => a in attrs || flags.has(a))) return {};
  const diagnostics: VisualizationDiagnostic[] = [];
  const fail = (attribute: string, message: string) => diagnostics.push({ code: "VisualizationConfigError", attribute, message });
  for (const a of attributes) {
    if (duplicates.includes(a)) fail(a, `Duplicate attribute: ${a}`);
    if (flags.has(a)) fail(a, `Attribute requires a value: ${a}`);
  }
  if (language.toLowerCase() !== "python") fail("visualize", "Visualization requires Python.");
  if (!["array", "tree", "graph", "dynamic-table"].includes(attrs.visualize)) fail("visualize", "visualize must be array, tree, graph or dynamic-table.");
  const name = (a: string, fallback?: string) => {
    const s = attrs[a] ?? fallback ?? "";
    if (!isIdentifier(s)) fail(a, `${a} must be an ASCII Python identifier.`);
    return s;
  };
  const list = (a: string, fallback: string) => {
    const raw = attrs[a] ?? fallback;
    const parts = ["pointers", "tables"].includes(a) && raw === "" ? [] : raw.split(",").map(s => s.trim());
    if (parts.some(s => !isIdentifier(s)) || new Set(parts).size !== parts.length) fail(a, `${a} contains an empty, duplicate or invalid name.`);
    return parts;
  };
  const config: VisualizationConfig = { type: attrs.visualize as VisualizationConfig["type"], watch: name("watch"), pointers: list("pointers", ""), scope: name("scope", "module") };
  if (config.pointers.length > 8) fail("pointers", "At most 8 pointers are supported.");
  if (config.type === "dynamic-table" && config.pointers.length) fail("pointers", "dynamic-table does not use pointers.");
  if ("tables" in attrs) config.tables = list("tables", "");
  if (config.type === "tree") config.tree = { valueField: name("value", "value"), childFields: list("children", "left,right") };
  else for (const a of ["value", "children"]) if (a in attrs) fail(a, `${a} is only supported for tree.`);
  if (config.type === "graph") config.graph = {};
  for (const [a, field] of [["node_states", "nodeStates"], ["edge_states", "edgeStates"]] as const) if (a in attrs) {
    if (config.graph) config.graph[field] = name(a);
    else fail(a, `${a} is only supported for graph.`);
  }
  for (const a of ["distances", "visited", "predecessors"]) if (a in attrs) fail(a, `${a} has been replaced by node_states. Put algorithm fields in the node state dictionary.`);
  return diagnostics.length ? { visualizationDiagnostics: diagnostics } : { visualization: config };
}

export function formatScalar(v: VizScalar): string {
  if (v === null) return "None";
  if (typeof v === "object") return v.kind === "integer" ? v.decimal : v.text;
  if (typeof v === "boolean") return v ? "True" : "False";
  return typeof v === "string" ? JSON.stringify(v) : String(v);
}

const obj = (v: unknown): v is Record<string, any> => v !== null && typeof v === "object" && !Array.isArray(v);
const id = (v: unknown): v is string => typeof v === "string" && v.length > 0 && v.length <= 256;
const natural = (v: unknown): v is number => Number.isSafeInteger(v) && Number(v) >= 0;
function scalar(v: unknown): boolean {
  if (v === null || typeof v === "boolean") return true;
  if (typeof v === "string") return Array.from(v).length <= 256;
  if (typeof v === "number") return Number.isFinite(v);
  return obj(v) && Object.keys(v).length === 2 && ((v.kind === "integer" && typeof v.decimal === "string" && /^-?(0|[1-9][0-9]*)$/.test(v.decimal) && v.decimal.length <= 256 * 1024) || (v.kind === "float" && ["nan", "inf", "-inf"].includes(v.text)));
}

// Treat JSON outputs as data only; reject incompatible versions and malformed graphs.
export function readVisualization(outputs: RichOutput[], cellId: string): { trace?: VisualizationTrace; error?: string } {
  const output = outputs.find(o => o.kind === "json" && o.mime === VISUALIZATION_MIME);
  if (!output) return {};
  try {
    if (new TextEncoder().encode(output.value).length > 10 * 1024 * 1024) throw new Error("Visualization output exceeds the size limit.");
    const t = JSON.parse(output.value);
    if (!obj(t) || t.schemaVersion !== 1) throw new Error("Unsupported visualization schemaVersion.");
    if (!id(t.runId) || t.cellId !== cellId || !obj(t.config)) throw new Error("Invalid visualization header.");
    const c = t.config;
    if (c.type === "graph" && obj(c.graph) && LEGACY_GRAPH_ANNOTATIONS.some(name => Object.prototype.hasOwnProperty.call(c.graph, name))) throw new Error(GRAPH_PROTOCOL_MISMATCH);
    const names = (v: unknown): v is string[] => Array.isArray(v) && v.every(n => typeof n === "string" && isIdentifier(n)) && new Set(v).size === v.length;
    if (!["array", "tree", "graph", "dynamic-table"].includes(c.type) || typeof c.watch !== "string" || !isIdentifier(c.watch) || typeof c.scope !== "string" || !isIdentifier(c.scope) || !names(c.pointers) || c.pointers.length > 8 || (c.type === "dynamic-table" && c.pointers.length) || (c.tables !== undefined && !names(c.tables)) || Object.keys(c).some(k => !["type", "watch", "pointers", "scope", "tree", "graph", "tables"].includes(k))) throw new Error("Invalid visualization configuration.");
    if (c.type === "tree" ? !obj(c.tree) || typeof c.tree.valueField !== "string" || !isIdentifier(c.tree.valueField) || !names(c.tree.childFields) || !c.tree.childFields.length || Object.keys(c.tree).some(k => !["valueField", "childFields"].includes(k)) : c.tree !== undefined) throw new Error("Invalid tree configuration.");
    if (c.type === "graph" ? !obj(c.graph) || Object.entries(c.graph).some(([k, v]) => !["nodeStates", "edgeStates"].includes(k) || typeof v !== "string" || !isIdentifier(v)) : c.graph !== undefined) throw new Error("Invalid graph configuration.");
    if (!Array.isArray(t.frames) || t.frames.length > 2000 || !Array.isArray(t.diagnostics) || t.diagnostics.length > 32 || !obj(t.capture) || !["complete", "truncated", "error"].includes(t.capture.status)) throw new Error("Invalid capture metadata.");
    for (const d of t.diagnostics) if (!obj(d) || !id(d.code) || typeof d.message !== "string" || d.message.length > 4096 || (d.seq !== undefined && !natural(d.seq))) throw new Error("Invalid diagnostic.");
    for (const [seq, f] of t.frames.entries()) {
      if (!obj(f) || f.seq !== seq || f.type !== c.type || !["call", "line", "return", "exception"].includes(f.event) || !["ready", "unbound", "invalid"].includes(f.status) || !Array.isArray(f.diagnosticCodes) || f.diagnosticCodes.length > 32 || !f.diagnosticCodes.every(id)) throw new Error("Invalid visualization frame.");
      const p = f.position;
      if (!obj(p) || !natural(p.cellLine) || p.cellLine < 1 || !id(p.frameId) || !id(p.scopeFrameId) || !id(p.functionName) || (p.documentLine !== undefined && (!natural(p.documentLine) || p.documentLine < 1))) throw new Error("Invalid source position.");
      if (!Array.isArray(f.pointers) || f.pointers.length !== c.pointers.length || f.pointers.some((p: any, i: number) => !obj(p) || p.name !== c.pointers[i])) throw new Error("Invalid pointers.");
      if (f.status !== "ready") { if (f.state !== null) throw new Error("Unavailable state must be null."); }
      else if (!obj(f.state)) throw new Error("Missing snapshot.");
      if ((f.tables !== undefined && !Array.isArray(f.tables)) || (f.tables ?? []).length !== (c.tables ?? []).length) throw new Error("Invalid dynamic table bindings.");
      for (const [i, table] of (f.tables ?? []).entries()) {
        if (!obj(table) || table.name !== c.tables[i] || !["ready", "unbound", "invalid"].includes(table.status) || (table.status === "ready" ? !obj(table.state) : table.state !== null)) throw new Error("Invalid dynamic table binding.");
        if (table.state) validateDynamicTable(table.state, scalar);
      }
      if (f.type === "array") {
        const s = f.state;
        if (s && (!id(s.objectId) || !Array.isArray(s.items) || s.items.length > 256 || !s.items.every(scalar))) throw new Error("Invalid array.");
        for (const p of f.pointers) {
          if (!["inBounds", "outOfBounds", "unbound", "invalidType"].includes(p.status) || !scalar(p.raw) || (p.status === "inBounds" ? !s || !natural(p.index) || p.index >= s.items.length || !Number.isSafeInteger(p.raw) || p.raw < -s.items.length || p.raw >= s.items.length || (p.raw + s.items.length) % s.items.length !== p.index : p.index !== null)) throw new Error("Invalid array pointer.");
          if ((p.status === "unbound" && p.raw !== null) || (p.status === "outOfBounds" && !Number.isSafeInteger(p.raw) && !(obj(p.raw) && p.raw.kind === "integer"))) throw new Error("Invalid array pointer value.");
        }
      } else if (f.type === "graph") {
        if (f.state) validateGraphState(f.state, c.graph, scalar);
        const ids = new Set(f.state?.nodes.map((n: any) => n.id));
        for (const p of f.pointers) if (!["matched", "null", "outside", "unbound", "invalidType"].includes(p.status) || (p.status === "matched" ? !ids.has(p.nodeId) : p.nodeId !== null)) throw new Error("Invalid graph pointer.");
      } else if (f.type === "dynamic-table") {
        if (f.state) validateDynamicTable(f.state, scalar);
      } else {
        const s = f.state;
        const nodes = new Map<string, TreeNode>();
        if (s) {
          if (!Array.isArray(s.nodes) || s.nodes.length > 128) throw new Error("Invalid tree nodes.");
          for (const n of s.nodes) {
            if (!obj(n) || !id(n.id) || nodes.has(n.id) || !scalar(n.value) || !Array.isArray(n.children)) throw new Error("Invalid tree node.");
            nodes.set(n.id, n as unknown as TreeNode);
          }
          let edges = 0;
          for (const n of nodes.values()) {
            let fieldIndex = -1, lastIndex: number | null = null;
            for (const e of n.children) {
              if (++edges > 512 || !obj(e) || !c.tree.childFields.includes(e.field) || (e.index !== null && (!natural(e.index) || e.index >= 512)) || (e.nodeId !== null && !nodes.has(e.nodeId))) throw new Error("Invalid tree edge.");
              const next = c.tree.childFields.indexOf(e.field);
              if (next < fieldIndex || (next === fieldIndex && (lastIndex === null || e.index !== lastIndex + 1)) || (next !== fieldIndex && e.index !== null && e.index !== 0)) throw new Error("Invalid tree child order.");
              fieldIndex = next; lastIndex = e.index;
            }
          }
          const seen = new Set<string>();
          const queue: [string, number][] = s.rootId === null ? [] : [[s.rootId, 0]];
          for (let i = 0; i < queue.length; i++) {
            const [key, depth] = queue[i]; const n = nodes.get(key);
            if (!n || seen.has(key) || depth > 32) throw new Error("Invalid tree topology.");
            seen.add(key);
            for (const e of n.children) if (e.nodeId !== null) queue.push([e.nodeId, depth + 1]);
          }
          if (seen.size !== nodes.size) throw new Error("Unreachable tree nodes.");
        }
        for (const p of f.pointers) if (!["matched", "null", "outside", "unbound", "invalidType"].includes(p.status) || (p.status === "matched" ? !nodes.has(p.nodeId) : p.nodeId !== null)) throw new Error("Invalid tree pointer.");
      }
    }
    return { trace: t as unknown as VisualizationTrace };
  } catch (e) { return { error: e instanceof Error ? e.message : "Invalid visualization output." }; }
}
