import type { VizScalar } from "./visualization";

export interface DynamicTableState { objectId: string; columns: string[]; rows: VizScalar[][] }
export interface DynamicTableSnapshot { name: string; status: "ready" | "unbound" | "invalid"; state: DynamicTableState | null }

export function validateDynamicTable(s: unknown, scalar: (v: unknown) => boolean): asserts s is DynamicTableState {
  const t = s as DynamicTableState | null;
  if (!t || typeof t !== "object" || typeof t.objectId !== "string" || !t.objectId.length || t.objectId.length > 256 || !Array.isArray(t.columns) || t.columns.some(c => typeof c !== "string" || !c.length || Array.from(c).length > 256) || new Set(t.columns).size !== t.columns.length || !Array.isArray(t.rows) || t.rows.length > 256 || t.rows.some(row => !Array.isArray(row) || row.length !== t.columns.length || !row.every(scalar))) throw new Error("Invalid dynamic table.");
}
