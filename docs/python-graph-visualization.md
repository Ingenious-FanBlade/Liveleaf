# Python Hook 有向图与通用状态

有向图可视化由四部分组成：**原始图数据、pointers、节点状态、边状态**。表格是独立的 [dynamic table](./python-dynamic-table.md)，可以与图使用同一回放时间轴。可运行示例见 [有向图、Dijkstra 与动态表格课程](./examples/dijkstra.ll.md)。

## 作者语法

````markdown
```python run visualize="graph" watch="graph" pointers="current,neighbor" node_states="node_states" edge_states="edge_states" tables="rows" scope="visit"
def visit(graph):
    node_states = {"A": {"distance": 0, "visited": False, "predecessor": None}}
    edge_states = {("A", "B"): {"flow": 0, "capacity": 10, "enabled": True}}
    rows = list(node_states.values())
    current, neighbor = "A", "B"
    node_states["A"]["visited"] = True
    edge_states[("A", "B")]["flow"] = 3

graph = {"A": {"B": 4}, "B": {}}
visit(graph)
```
````

`watch` 必填；`pointers`、`scope`、`node_states`、`edge_states`、`tables` 均可省略。所有绑定都是 ASCII Python 变量名，不执行表达式。`node_states`、`edge_states` 只适用于 graph，`tables` 可与任意可视化类型组合。重复属性或错误配置会在执行代码前拒绝，前后端使用共享语法 fixture 验证。

旧的 `distances`、`visited`、`predecessors` 专用属性已移除，使用时会提示迁移到 `node_states`。距离、已访问、前驱等内容现在全部是普通节点状态成员；可视化器不解释其算法含义，也不自动推断前驱边。

## 原始图与指针

- 图是精确内建 `dict`；节点键为精确内建 `str` 或 `int`，排除 bool、浮点键和自定义对象。
- 带权邻接表为 `{node: {neighbor: weight}}`，权重为有限 int/float。一般图允许负边权，Dijkstra 示例自身拒绝负权。无权邻接表为 `{node: [neighbor, ...]}` 或 tuple，权重编码为 null。
- 只出现在邻居位置的键也会生成节点，视为没有出边；空字典为有效空图。支持环、自环、断开的子图和孤立节点；不支持同一有序节点对的平行边。
- 节点按键类型和值匹配，整数 `1` 和字符串 `"1"` 不同。节点删除后重新加入保留 ID；整个图重新绑定产生新的 objectId 和节点身份。
- 指针按节点键匹配，状态为 matched/null/outside/unbound/invalidType。首个指针使用蓝色边框，其余使用橙色；重合时保留两层边框。前两个指针之间存在的有向边加粗为蓝色。

## 节点状态与边状态

`node_states` 绑定 `{节点键: {成员名: 标量值, ...}, ...}`；`edge_states` 绑定 `{(起点键, 终点键): {成员名: 标量值, ...}, ...}`。内外层均须为精确内建 dict，边键须为精确内建二元 tuple，并对应真实存在的有向边。

成员数量没有固定上限，仅受现有采集字节预算约束。成员名为非空字符串，最多 256 个 Unicode 码点；值沿用 VizScalar 编码：None、bool、str、int、float，含大整数、NaN 和正负无穷。嵌套容器及自定义对象不作为状态值读取，不调用用户转换或属性访问钩子。

状态字典可只覆盖部分节点或边，未覆盖项的 `state` 为 `{}`。成员和状态条目可动态增加、修改、删除。某种状态尚未绑定或无效时，仅清空该类附加状态并保存诊断，原图和另一类状态继续显示；绑定修复后继续采集。图本身无效时，独立表格仍可正常回放。

图内保持节点名和边权；悬浮提示及默认折叠的“观察详情”显示任意状态成员。黄色表示节点/边状态变化，蓝色仍表示当前指针关系。图组件不生成表格，不再根据 `visited` 或 `predecessor` 等成员名添加固定颜色语义。

## 快照协议

当前 JSONL 封套仍使用 schemaVersion 1。graph 配置改为 `graph: {nodeStates?: string, edgeStates?: string}`，旧固定字段不再接受；array/tree 的主体结构保持不变。可选的 `tables: string[]` 是独立数组绑定。

```ts
interface GraphState {
  objectId: string;
  directed: true;
  nodes: Array<{ id: string; value: VizScalar; state: Record<string, VizScalar> }>;
  edges: Array<{
    id: string; source: string; target: string;
    weight: VizScalar;
    state: Record<string, VizScalar>;
  }>;
  annotations: Record<"nodeStates" | "edgeStates", "off" | "ready" | "unbound" | "invalid">;
}
```

状态帧保留 type/state/pointers；graph 指针复用 TreePointer。节点与边 ID 唯一，所有边与指针引用当前帧的节点。普通状态值不承担结构引用约束。Python、Rust 和前端分别校验数据。

单帧最多 128 个节点、512 条边；图对象、注册的节点键与独立表格对象共同计入 512 个对象预算。继续使用 2,000 帧、8 MiB JSONL、256 KiB 单记录及 32 条诊断上限；达到预算后继续执行用户代码，异常和超时可回放有效前缀。

播放器使用确定性环形布局，状态变化不改变坐标；双向边分为曲线，自环单独绘制。每条箭头由线身及头部形成一条闭合填充路径。支持缩放、拖动和局部滚动。

节点/边状态可以表达 BFS 层数、拓扑入度、并查集分量、容量和流量等信息，无需扩展固定算法字段。当前原图仍按有向图解释；对称邻接表会显示两个方向的边。
