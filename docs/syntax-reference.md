# 活页 Liveleaf 特制语法清单

## 幻灯片分页

代码围栏外独占一行：

```markdown
---
```

## 可执行代码块

````markdown
```python run id="hello" timeout=5000 session="lesson"
print("Hello")
```

```python exercise id="practice"
def double(value):
    return value * 2
```
````

| 标记 / 属性 | 含义 |
| --- | --- |
| `run` | 可运行代码；与 `exercise` 互斥 |
| `exercise` | 练习代码；与 `run` 互斥 |
| `id="hello"` | 非空、全文唯一的代码块 ID |
| `timeout=5000` | 超时毫秒数，1–3600000，默认 5000 |
| `session="lesson"` | 会话名称；不保留跨次运行的 Python 状态 |

## 可视化代码块

````markdown
```python run id="array" visualize="array" watch="values" pointers="i"
values = [3, 1, 2]
for i in range(len(values)):
    values[i] *= 2
```

```python run id="tree" visualize="tree" watch="root" value="val" children="left,right" pointers="node" scope="visit"
root = {"val": "A", "left": None, "right": None}

def visit(node):
    print(node["val"])

visit(root)
```

```python run id="graph" visualize="graph" watch="graph" pointers="current,neighbor" node_states="states" edge_states="edges" tables="rows"
graph = {"A": {"B": 4}, "B": {}}
states = {"A": {"distance": 0}, "B": {"distance": None}}
edges = {("A", "B"): {"active": False}}
rows = list(states.values())
current, neighbor = "A", "B"
states[neighbor]["distance"] = 4
edges[(current, neighbor)]["active"] = True
```

```python run id="table" visualize="dynamic-table" watch="rows"
rows = [{"name": "A", "cost": 3}]
rows[0]["cost"] = 2
```
````

| 属性 | 取值 / 含义 |
| --- | --- |
| `visualize="array"` | 标量 list；指针为数组下标 |
| `visualize="tree"` | 节点 dict / 普通实例对象，或空根 `None`；指针为节点引用 |
| `visualize="graph"` | 邻接 dict：`{节点: {邻居: 权重}}` 或 `{节点: [邻居]}`；指针为节点键 |
| `visualize="dynamic-table"` | list / tuple；行统一为标量、记录 dict 或二维 list / tuple |
| `watch="values"` | 必填，观察的变量名 |
| `pointers="i,j"` | 最多 8 个不重复的变量名；dynamic-table 不使用此属性 |
| `scope="module"` | 默认模块作用域；也可指定唯一、无装饰器的顶层同步函数名 |
| `value="val"` | 仅 tree：节点值字段名，默认 `value` |
| `children="left,right"` | 仅 tree：子节点字段名列表，默认 `left,right`；空孩子为 `None` |
| `node_states="states"` | 仅 graph：`{节点键: {成员名: 标量}}` |
| `edge_states="edges"` | 仅 graph：`{(源节点键, 目标节点键): {成员名: 标量}}` |
| `tables="rows,queue"` | 为任意可视化附加独立 dynamic-table，绑定数组变量 |

绑定的变量名、函数名和树字段名使用非关键字的 ASCII Python 标识符；列表以逗号分隔，不使用表达式。

## 教学容器

```markdown
::: note
注释内容
:::

::: tip
提示内容
:::

::: warning
警告内容
:::

::: danger
危险提示内容
:::

::: question
问题内容
:::

::: hint
可折叠提示内容
:::

::: solution
默认展开的答案
:::

::: solution collapsed
默认折叠的答案
:::

::: solution open
显式展开的答案
:::
```

每个容器使用独立的 `:::` 闭合，并在分页符前闭合。

## 分栏

```markdown
::: columns
::: column width=45%
左栏内容
:::

::: column width=55%
右栏内容
:::
:::
```

`column` 必须是 `columns` 的直接子项。`width` 可省略，支持 `%`、`px`、`rem`、`em`。

## 视频与音频

```markdown
::video[标题](./assets/demo.mp4){controls start=12 width=80% poster=./assets/poster.png}
::audio[标题](./assets/intro.mp3){controls start=2 width=100%}
```

| 属性 | 含义 |
| --- | --- |
| `controls` | 显示播放控件 |
| `start=12` | 起始秒数，支持非负整数或小数 |
| `width=80%` | 宽度，支持 `%`、`px`、`rem`、`em` |
| `poster="./assets/poster.png"` | 视频封面，仅 video |
