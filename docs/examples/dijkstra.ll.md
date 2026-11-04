# 有向图 · 节点、箭头与权重

普通邻接表就能直接可视化。`{"A": {"B": 4}}` 表示一条 **A → B、权重为 4** 的边。箭头只表示声明的方向，不会自动补出反向边。

```python run id="directed-graph" visualize="graph" watch="graph" pointers="current,neighbor" node_states="node_states" edge_states="edge_states"
graph = {
    "A": {"B": 4, "C": 1},
    "B": {"A": 2, "C": 2},
    "C": {"C": 0},
    "D": {},
}
node_states = {"A": {"label": "起点", "active": True}, "D": {"label": "孤立节点"}}
edge_states = {("A", "B"): {"capacity": 10, "flow": 0, "enabled": True}}
current = "A"
neighbor = "B"
edge_states[("A", "B")]["flow"] = 3
node_states["A"]["active"] = False
current = "B"
neighbor = "A"
```

回放支持环、自环、双向连边和孤立节点。A → B 与 B → A 分别显示自己的箭头和权重，D 没有任何连边。蓝色边框标记 `current`，橙色边框标记 `neighbor`；二者之间存在的有向边加粗为蓝色。节点状态和边状态可以包含任意多个具名标量成员；悬浮提示和“观察详情”展示全部成员，黄色表示变化。无权图也可以写成 `{"A": ["B", "C"], "B": [], "C": []}`。

---

# Dijkstra · 从 A 出发的最短路径

每轮从最小堆中取出尚未确定的最近节点，再尝试通过它缩短邻居的距离。这一步称为**松弛**。Dijkstra 要求所有边权非负；下面的实现会明确拒绝负权、NaN 和无穷边权。

观察声明中，`current,neighbor` 指向正在处理的节点与邻居；`node_states` 保存每个节点的距离、已确定标记和前驱。这些只是作者定义的状态成员。独立的 dynamic table 通过 `tables="table_rows"` 绑定记录数组；数组中的记录与节点状态引用同一个字典，修改后两处同步更新。算法无需导入绘图库。

```python run id="dijkstra-shortest-path" visualize="graph" watch="graph" pointers="current,neighbor" node_states="node_states" tables="table_rows" scope="dijkstra" timeout=5000
from heapq import heappop, heappush
from itertools import count
from math import isfinite

def dijkstra(graph, start):
    for adjacent in graph.values():
        for weight in adjacent.values():
            if type(weight) not in (int, float) or weight < 0:
                raise ValueError("Dijkstra 需要非负的数值边权")
            if type(weight) is float and not isfinite(weight):
                raise ValueError("边权必须是有限数值")

    nodes = dict.fromkeys(graph)
    for adjacent in graph.values():
        for node in adjacent:
            nodes.setdefault(node, None)
    if start not in nodes:
        raise ValueError("起点必须在图中")
    node_states = {
        node: {"node": node, "distance": float("inf"), "visited": False, "predecessor": None}
        for node in nodes
    }
    table_rows = list(node_states.values())
    current = neighbor = None
    node_states[start]["distance"] = 0
    order = count()
    heap = [(0, next(order), start)]

    while heap:
        best, _, current = heappop(heap)
        if node_states[current]["visited"]:
            continue
        node_states[current]["visited"] = True

        for neighbor, weight in graph.get(current, {}).items():
            if node_states[neighbor]["visited"]:
                continue
            candidate = best + weight
            if candidate < node_states[neighbor]["distance"]:
                node_states[neighbor]["distance"] = candidate
                node_states[neighbor]["predecessor"] = current
                heappush(heap, (candidate, next(order), neighbor))

    return ({node: state["distance"] for node, state in node_states.items()},
            {node: state["predecessor"] for node, state in node_states.items()})

graph = {
    "A": {"B": 4, "C": 1},
    "B": {"D": 1, "E": 7},
    "C": {"B": 2, "D": 5},
    "D": {"E": 3},
    "E": {"F": 1},
    "F": {"C": 2},
    "G": {},
}
distances, previous = dijkstra(graph, "A")
for node, distance in distances.items():
    print(f"A → {node}: {distance}")

path = []
node = "F"
while node is not None:
    path.append(node)
    node = previous[node]
print("到 F 的路径:", " → ".join(reversed(path)))
```

| 节点 | A | B | C | D | E | F | G |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 从 A 出发的最终距离 | 0 | 3 | 1 | 4 | 7 | 8 | ∞ |

到 F 的最短路径为 **A → C → B → D → E → F**，总长度 **8**。G 不可达，距离保持 `∞`，不会进入已访问集合。

::: tip
单步观察独立动态表格中 B 的 distance 从 4 改成 3，D 从 6 改成 4，E 从 10 改成 7。黄色节点和单元格标记状态变化。visited、predecessor 与其他自定义成员一样显示在状态详情和表格中；可视化器不会据此推断算法语义。蓝色加粗边连接当前两个指针，它本身不表示松弛成功。
:::

最小堆可以保留旧候选，例如 B 的旧距离 4。节点的 `visited` 状态检查会跳过这些过期条目。回放仍按真实源码事件逐步展示，不把“刚入堆”当成“距离已确定”。

---

# Dynamic table · 独立观察数组

表格可以独立使用，不需要图。记录数组的字段自动成为列；二维数组按下标生成列，一维数组显示为一列。行列的增加、删除及单元格修改都随源码事件回放。

```python run id="dynamic-table" visualize="dynamic-table" watch="rows"
rows = [{"任务": "读取输入", "完成": False}, {"任务": "计算结果", "完成": False}]
rows[0]["完成"] = True
rows[0]["耗时"] = 2
rows.append({"任务": "输出结果", "完成": False, "耗时": 1})
del rows[1]
rows[-1]["完成"] = True
```
