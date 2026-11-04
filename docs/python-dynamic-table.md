# Python Hook Dynamic Table

Dynamic table 是绑定 Python 数组的独立可视化要素，不依赖图、节点状态或特定算法。可单独回放，也可与数组、树或有向图同步显示。

## 单独使用

````markdown
```python run visualize="dynamic-table" watch="rows"
rows = [{"node": "A", "cost": 4}, {"node": "B", "cost": 8}]
rows[0]["cost"] = 3
rows[1]["settled"] = True
rows.append({"node": "C", "cost": 9})
del rows[0]
```
````

`watch` 绑定精确内建 list 或 tuple。`scope` 仍可选择同步函数作用域；dynamic-table 不使用 pointers。

## 与其他要素组合

在已有可视化声明中添加 `tables="rows,queue"`，每个名字绑定一个独立数组，各自显示为 dynamic table。它们与主图共用源码事件、播放、单步和拖动进度，但可独立处于 ready/unbound/invalid 状态。主图不可用时，已绑定的表格继续显示。

```python
node_states = {"A": {"node": "A", "distance": 0, "visited": False}}
rows = list(node_states.values())
node_states["A"]["visited"] = True
```

这里 `rows[0]` 与 `node_states["A"]` 是同一个字典，修改成员会同步反映到图状态和表格。增加/删除节点时，作者需相应维护 `rows` 数组；表格不会自动从图生成数据。

## 数组数据形式

| 输入 | 列与单元格 |
| --- | --- |
| `[{"name": "A", "cost": 3}, {"name": "B"}]` | 记录数组；按字段首次出现的顺序取列名并集，缺少字段填 None |
| `[[1, 2], [3]]` | 二维数组；列名为 `0`、`1`，短行补 None |
| `[1, 2, 3]` | 一维数组；列名为 `value` |
| `[]` | 空表格 |

同一数组中的行须采用一致形式，记录为精确内建 dict，二维行可为精确 list/tuple。记录字段名为非空字符串，最多 256 个 Unicode 码点；单元格使用 VizScalar，支持字符串、布尔值、None、大整数和特殊浮点数。字段、行、列均可在执行期间增删。

每帧最多 256 行，列数不设固定上限，受 256 KiB 单记录和共享采集字节预算约束。无效数据仅影响该表格，修复后可继续回放；预算耗尽时停止采集并继续执行算法。

## 快照与变化比较

```ts
interface DynamicTableState {
  objectId: string;
  columns: string[];
  rows: VizScalar[][];
}
interface DynamicTableSnapshot {
  name: string;
  status: "ready" | "unbound" | "invalid";
  state: DynamicTableState | null;
}
```

单独使用时，帧的 `type` 为 `dynamic-table`，`state` 为 DynamicTableState，`pointers` 为空。组合使用时，配置含 `tables: string[]`，每帧的 `tables: DynamicTableSnapshot[]` 按绑定顺序保存独立状态，不嵌入 GraphState。

相邻事件之间按**行下标和列名**比较，变化的单元格以黄色标记。列调整顺序不会错配；插入、删除行后按新的下标比较。重新绑定数组、更换函数调用作用域或中间状态不可用时重置比较。表格具有独立滚动区域和固定表头。
