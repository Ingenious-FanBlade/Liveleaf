# Python Hook 可视化功能设计

状态：第一版已实现，支持 `array` 与 `tree`。采用代码块属性声明观察规则，通过 Python 执行追踪采集状态，在执行结束后回放。实现位于本文第 11 节列出的 Python、Rust、Vue 模块；可运行课程见 [数组与二叉树先序遍历示例](./examples/python-visualization.ll.md)，也已加入应用内置示例。当前实际验收解释器为 CPython 3.13。

## 1. 目标与范围

课程作者应当能够直接展示正常的 Python 算法，通过 Markdown 代码围栏的属性选择观察对象。算法内部不需要导入可视化库、调用绘图函数或使用特殊变量名前缀。

第一版提供：

- `array`：一维数组、元素变化、下标指针。
- `tree`：有序树结构、节点值与连边变化、节点引用指针。默认使用二叉树字段，也支持显式声明子节点列表。
- 当前源码行、函数调用上下文及状态快照。
- 执行结束后的播放、暂停、前后单步、进度定位与倍速回放。
- Python 异常、超时和采集截断时的已有记录回放。

第一版不包括 AST 插桩、自动识别比较或交换、动态规划依赖表达式、C 扩展内部步骤、多线程或异步执行追踪、跨代码块共享状态、实时调试和执行中断点。回放暂停只暂停播放器，代码在回放开始前已经执行完毕。

`array` 与 `tree` 是渲染类型，不承诺自动理解算法含义。例如 `dp[i] = dp[i - 1] + dp[i - 2]` 可以展示状态变化，但不会自动生成递推依赖箭头。

## 2. 当前实现与接入位置

当前执行链路为：

```text
Markdown 源码与 cellId
  → run_code_cell
  → Rust 重新解析源码并定位 CodeCell
  → runtime::python 写入 cell.py
  → 启动本地 Python 子进程
  → 收集 stdout、stderr、退出状态
  → RunResult
```

相关实现：

| 位置 | 当前职责 | 本功能需要的调整 |
| --- | --- | --- |
| `src/markdown/documentModel.ts` | 前端 CodeCell 类型 | 增加可视化配置和配置诊断 |
| `src/markdown/parseCourse.ts` | 代码围栏属性解析、预览 | 识别并验证新增属性 |
| `src-tauri/src/markdown.rs` | Rust 文档模型与解析 | 生成相同配置，作为执行时的权威来源 |
| `src-tauri/src/lib.rs` | 按源码与 cellId 执行 | 将重新解析得到的配置传入 Python runtime |
| `src-tauri/src/runtime/python.rs` | 子进程执行、超时、输出 | 可选启动 runner，回收追踪文件 |
| `src/runtime/runResult.ts` | 输出类型 | 增加可视化协议类型，继续复用 JSON RichOutput |
| `src/App.vue` | 文档与演示模式中的输出展示 | 接入共用的可视化播放器 |

当前 `RichOutput` 已允许 `kind: "json"`，但 Python runtime 只生成文本输出，界面也主要展示 stdout、stderr 和错误。JSON 类型的存在不代表已有追踪或可视化能力。

未声明 `visualize` 的代码块继续沿用直接执行 `cell.py` 的路径。

## 3. 作者语法

### 3.1 Array

````markdown
```python run id="bubble-sort" visualize="array" watch="arr" pointers="i,j"
arr = [5, 2, 4, 1]

for i in range(len(arr)):
    for j in range(len(arr) - 1 - i):
        if arr[j] > arr[j + 1]:
            arr[j], arr[j + 1] = arr[j + 1], arr[j]
```
````

`watch` 指定一个数组变量。`pointers` 中的变量按整数下标解释，名称直接作为指针标签。变量未初始化时不显示对应指针。

### 3.2 Tree

````markdown
```python run id="tree-update" visualize="tree" watch="root" value="val" children="left,right" pointers="node"
class Node:
    def __init__(self, val, left=None, right=None):
        self.val = val
        self.left = left
        self.right = right

root = Node(4, Node(2), Node(7))
node = root.left
node.val = 3
root.right.left = Node(6)
node = root.right.left
```
````

`watch` 指定根节点变量。`value` 指定节点值字段，`children` 指定按顺序读取的子节点字段。`pointers` 中的变量按对象身份匹配树内节点。

多叉树可以使用同一个渲染类型：

````markdown
```python run visualize="tree" watch="root" value="name" children="children"
root = {
    "name": "A",
    "children": [
        {"name": "B", "children": []},
        {"name": "C", "children": []},
    ],
}
```
````

### 3.3 属性约定

| 属性 | 适用类型 | 必填 | 默认值或含义 |
| --- | --- | --- | --- |
| `visualize` | 全部 | 开启可视化时必填 | 仅允许 `array`、`tree` |
| `watch` | 全部 | 是 | 一个变量名，每个代码块只观察一个主体 |
| `pointers` | 全部 | 否 | 逗号分隔的变量名，最多 8 个 |
| `scope` | 全部 | 否 | `module`，也可指定当前代码块中唯一的顶层函数名 |
| `value` | tree | 否 | `value`，节点值的字段名 |
| `children` | tree | 否 | `left,right`，有序子节点字段列表 |

现有 `id`、`timeout`、`run` 与 `exercise` 语义继续适用。第一版每次运行独立创建 Python 进程；已有 `session` 元数据仍不提供持久会话或跨块观察能力。

语法与验证规则：

1. 只有标记 `run` 或 `exercise` 的 Python 代码块可以开启可视化；普通代码围栏保持静态展示，不因这些属性自动运行。
2. `watch`、`pointers`、`value`、`children` 的名字使用 ASCII 标识符 `[A-Za-z_][A-Za-z0-9_]*`，排除 Python 关键字；`scope` 除保留值 `module` 外使用同一规则。
3. 名字是声明，不是表达式。不支持 `arr[0]`、`root.left`、`i+1`、函数调用或 `eval`。
4. 逗号分隔列表会去除每项两侧空白；拒绝空项和重复项。属性值区分大小写。
5. 显式 `pointers=""` 等同于未指定指针；其他必填属性及 `children` 不允许空值。
6. 可视化属性重复出现、未知 `visualize` 值、array 使用 tree 专属字段，以及缺少 `watch`，均为配置错误。带有 `watch` 等观察属性却未声明 `visualize`，同样提示配置错误。
7. 标记为可执行的非 Python 块使用这些属性时提示不支持。配置错误不执行用户代码；编辑器仍展示课程其余部分。
8. 前端和 Rust 必须保留重复属性诊断，不能继续让属性字典的后值覆盖前值而不提示。
9. 普通非可视化属性维持既有兼容策略。本版本不引入任意 Python 配置表达式或外部配置文件。

### 3.4 函数作用域

```text
visualize="array" watch="arr" pointers="j" scope="bubble_sort"
visualize="tree" watch="root" pointers="node" scope="visit"
```

`scope="module"` 时，观察变量从用户代码的模块命名空间读取。即使正在执行同一代码块内的辅助函数，也继续采样模块中的对象，所以可以看到辅助函数对该对象的原地修改。

`scope="函数名"` 时，选择调用栈中最近的、属于该代码块指定顶层函数的活动帧。优先读取该帧中的局部变量，再按 Python 的名字绑定规则读取该模块的全局变量。已经声明为局部但尚未绑定的名字视为未绑定，不能错误地退回同名全局变量。闭包名字同样遵守实际绑定状态，不搜索其他调用者的任意局部变量。

指定函数时，递归调用使用最内层活动调用；辅助函数执行期间继续观察所选调用帧。每次调用都有独立的 `scopeFrameId`。退出递归调用后恢复外层上下文，播放器必须区分这类切换，不能把不同调用的数组当成同一对象的修改。

没有指定函数的活动帧时不采样；函数返回前保存最后一帧。该函数从未调用时返回 `ScopeNotEntered` 诊断，而不伪造空数组或空树。返回后的最后记录仍可回放，不表示局部变量继续存活。

第一版函数作用域只支持无歧义的顶层同步 `def`；不支持方法、嵌套函数、lambda、装饰器包装后的作用域匹配。runner 在执行前验证指定定义。类体和推导式等其他执行帧不会仅因名字相同而被选中。

## 4. Array 观察模型

### 4.1 数据与变化

第一版接受精确的内建 `list`、`tuple` 类型，元素为第 7 节定义的标量。不接受嵌套数组、自定义容器、容器子类、生成器或 NumPy 数组。

- 初始化前为 `unbound`，`[]` 为有效空数组，`None` 为类型错误。
- 支持赋值、原地修改、追加、删除、切片修改、交换和重新绑定后状态的展示。
- 通过别名修改被观察数组，同样能在下一个采样点反映。
- `watch` 绑定变量；变量重新赋值后切换到新对象，不固定跟踪第一次绑定的对象。
- 数组对象使用运行内稳定的 `objectId`；相同内容的新对象仍是重新绑定。
- 每帧立即转换为独立 JSON 数据，不保留可变对象作为历史快照，也不调用用户的 `__deepcopy__`。

数组元素变化由相邻有效快照比较得到。长度改变时按索引展示新状态；不把差异自动命名为“插入”“交换”等算法操作。

### 4.2 指针

只有精确的内建 `int` 可作为下标，`bool` 不视为整数指针。

| 情况 | 展示 |
| --- | --- |
| `0 <= i < len(arr)` | 指向对应元素 |
| `-len(arr) <= i < 0` | 按 Python 负索引定位，同时保留原始负数标签 |
| 其他整数 | 在数组外显示越界标签，不强行夹到边界 |
| 未绑定 | 标记未绑定，不显示箭头 |
| 非整数 | 标记无效类型，不改变代码执行 |

快照同时保存原始值与归一化下标。多个指针指向同一元素时合并布局，保留所有名字。

### 4.3 走楼梯示例

````markdown
```python run id="climb-stairs" visualize="array" watch="dp" pointers="i"
n = 5
dp = [0] * (n + 1)
dp[0] = dp[1] = 1

for i in range(2, n + 1):
    dp[i] = dp[i - 1] + dp[i - 2]

print(dp[n])
```
````

回放可以看到 `dp` 的初始化、每次写入后的状态以及 `i` 的移动，最终数组为 `[1, 1, 2, 3, 5, 8]`。初始填充的 `0` 必须显示为 `0`；第一版不能推断它代表“尚未计算”。`i` 只指向 `dp[i]`，不会隐式产生 `i-1`、`i-2` 指针或依赖箭头。

## 5. Tree 观察模型

### 5.1 节点与字段读取

节点可以是精确的内建 `dict`，或使用普通实例属性存储字段的 Python 对象。字典按键读取，对象按字段名静态读取；同一棵树允许两类节点混合。

对象读取使用 `inspect.getattr_static` 一类的静态访问方式，不执行 `property`、`__getattr__` 或用户自定义描述符。遇到描述符、`__slots__` 成员或无法静态取得的字段时返回明确诊断。节点标签也不通过 `repr`、`str` 或任意序列化钩子生成，避免观察过程主动调用用户代码。[Python 静态属性读取说明](https://docs.python.org/3.13/library/inspect.html#inspect.getattr_static)

字段约定：

- `value` 字段必须存在，且值必须为受支持的标量。
- `children` 中每个字段必须存在；值可以是 `None`、一个子节点，或精确内建 `list` / `tuple` 组成的有序子节点列表。
- 子节点列表中的 `None` 表示空槽，保留槽位编号；不把后续孩子向前重编号。
- 缺失子节点字段与字段值为 `None` 不同：前者是结构错误，后者是有效空子树。
- `root = None` 是有效空树；根变量尚未绑定则为 `unbound`。
- 默认 `left,right` 保留左右槽位，即使只有右孩子也不能把它渲染为左孩子。

多叉树的孩子顺序按 `children` 字段声明顺序，再按列表索引顺序展开。协议分别保存字段与索引，而不是把 `children[0]` 当成可求值表达式。

### 5.2 节点身份与结构约束

节点 ID 根据对象身份生成，不能用节点值充当 ID。两个值都为 `3` 的不同节点必须分开；修改节点值不改变其 ID。节点断开后在同一次运行重新接入，继续使用原 ID。

Python 的 `id()` 只保证对象存活期间的唯一性，生命周期不重叠的对象可能重用同一个值。因此内部注册表必须结合对象引用验证身份，不能只长期保存裸 `id()` 数字。[Python 对象身份说明](https://docs.python.org/3.13/library/functions.html#id)

第一版采用有界对象注册表，为已注册的数组和树节点保留强引用，直至采集结束或关闭。达到对象上限时停止采集并释放注册表。该选择可能延后对象析构；依赖析构器、弱引用回调或垃圾回收时序的程序不属于可视化语义保证范围。

每帧从当前根遍历，校验可达结构：

- 不允许环。
- 不允许同一个节点被多个孩子位置引用，包括同一父节点的两个槽位。
- 断开的节点不属于当前树，保留历史快照但不画在当前树内。
- 结构不满足树约束时，该帧为 `invalid`，显示 `NotATree` 诊断；不无限递归，也不把共享结构静默展开为多个副本。
- 后续代码修复结构后，可以重新产生有效帧。

采用迭代遍历与有界队列，避免采集器对深树递归调用。

### 5.3 节点指针

`pointers="node,parent"` 按对象身份匹配当前可达节点：

- 引用树内节点：显示变量名并指向该节点。
- `None`：显示空引用。
- 未绑定：显示未绑定状态。
- 引用有效但不在当前树内的节点：显示树外引用，不自动改变根或加入该节点。
- 其他值：显示无效类型。

遍历到某个节点不等于它已经“访问完成”。第一版只展示引用位置，不自动推断访问序列、染色规则或平衡树旋转名称。

## 6. Hook 执行机制

### 6.1 Runner 与普通代码隔离

开启可视化后，Rust 将原始代码、规范化配置及内置 runner 写入本次运行目录：

```text
run-dir/
  cell.py
  visualization-config.json
  trace.jsonl
  stdout.txt
  stderr.txt
  runner.py
  liveleaf_viz/...
```

Python 启动 runner，runner 验证配置与代码、安装追踪器，再执行 `compile(source, cell_path, "exec")` 产生的代码对象。第一版不改写用户 AST，也不在 `cell.py` 前面拼接引导代码，避免改变代码行号和 `__future__` 导入位置。执行前可以读取 AST 验证函数作用域和不支持的执行形式，但不做插桩。

用户代码使用独立模块命名空间。runner 自身的配置、文件句柄与辅助函数不注入用户的全局变量。应复现现有脚本执行的 `__name__="__main__"`、`__file__`、`sys.argv`、`sys.path[0]` 和 `sys.modules["__main__"]` 行为，尤其要兼容普通导入和依赖模块注册的类定义。

`workingDirectory`、现有环境变量、Python 可执行文件选择和超时规则继续由 Rust 管理。runner 路径通过参数传递，配置通过 JSON 文件传递，不拼接为 Python 表达式。

### 6.2 追踪范围与采样语义

使用 `sys.settrace()`，按编译后代码对象的文件来源过滤，只接收当前 `cell.py` 中的事件；不追踪 runner、标准库、导入模块或 C 扩展内部代码。该 API 支持源码行、调用、返回与异常事件，行事件发生在相应源码行执行之前。[Python 追踪 API](https://docs.python.org/3.13/library/sys.html#sys.settrace)

| 事件 | 快照含义 | 源码高亮文案 |
| --- | --- | --- |
| `call` | 进入用户代码执行帧时的绑定状态 | 进入函数或代码块 |
| `line` | 该行尚未执行；状态包含之前已完成操作的结果 | 即将执行第 N 行 |
| `return` | 离开执行帧前的最后状态 | 离开当前调用 |
| `exception` | 异常发生时可取得的状态，可能包含部分已完成修改 | 第 N 行发生异常 |

不能把 `line` 快照说成“第 N 行执行后”。单行包含多条语句、列表推导式或一次 C 函数调用时，不承诺表达式级中间状态。`arr.sort()` 通常只能展示调用前后数组，不能展示排序内部过程。

`return` 也可能由异常展开引发，不代表函数正常成功；最终执行状态以 `RunResult.status` 为准。正常结束时通过模块或指定函数的返回事件补足最后状态，异常通过事件及最终退出结果关联。异常被用户捕获后仍可继续产生快照。

第一版保留相关事件，即使内容相同，也可能需要推动源码高亮或显示调用上下文。播放器的结构布局可以复用，采集器不能仅按数组内容相同就删除记录。

### 6.3 作用域与源码位置

追踪事件所在帧与被观察作用域帧分别记录。例如在辅助函数内修改全局 `arr` 时，源码位置属于辅助函数，而观察作用域仍是模块。

- `frameId`：触发事件的用户执行帧标识，在本次采集内稳定。
- `scopeFrameId`：本次读取变量所使用的作用域帧标识。
- `cellLine`：Python 代码块内部行号，从 1 开始。
- `functionName`：事件所属函数或 `<module>`。

对当前围栏代码，文档行号由 Rust 权威解析得到的围栏起始行加 `cellLine` 换算。换行先统一为 LF，不能在写入 `cell.py` 时额外裁剪空行。列布局、列表嵌套和多个幻灯片均需验证行号映射；不能直接使用前端当前的幻灯片内相对范围作为全局行号。

### 6.4 采集失败与执行语义

变量未绑定、类型不符或暂时出现非树结构，只影响对应帧的展示。采集器内部异常、文件写入失败或达到预算时关闭采集，保留诊断，让用户代码继续按原有超时规则执行。不能让追踪回调异常泄漏成用户算法异常。

在 `finally` 中恢复追踪设置、关闭文件并释放注册表。不能吞掉用户的异常、`SystemExit` 或中断。若用户代码替换或关闭追踪器，结束检查应报告 `TraceModified`，不能把缺失的记录标记为完整。

第一版支持本地 CPython 的单线程同步代码。生成器、协程等不支持的控制流在执行前给出可视化诊断并拒绝该次可视化运行；普通未开启可视化的代码块不受此限制。额外线程和导入库内部的修改不具备完整追踪保证，应在使用范围说明中明确。

## 7. 数据协议

### 7.1 公共标量

Python 与 JavaScript 之间必须保持值的含义：

```ts
type VizScalar =
  | null
  | boolean
  | string
  | number
  | { kind: "integer"; decimal: string }
  | { kind: "float"; text: "nan" | "inf" | "-inf" };
```

普通整数仅在 JavaScript 安全整数范围内编码为 number；其他整数使用十进制字符串标签。有限浮点数使用 number，非有限浮点数使用显式标签，不能输出非标准 JSON 的 `NaN` 或 `Infinity`。只接受精确的内建标量类型，不调用用户自定义转换。

字符串按原文显示并进行文本转义。每个字符串最多 256 个 Unicode 码点；超出时该值对应的状态无效，并报告 `ValueTooLarge`，不静默替换为截断值。

### 7.2 快照类型

下列 TypeScript 是跨 Python、Rust、前端的协议约束；实现时 Rust 必须提供相同的序列化模型和验证。

```ts
interface ArrayState {
  objectId: string;
  items: VizScalar[];
}

interface ArrayPointer {
  name: string;
  raw: VizScalar;
  index: number | null;
  status: "inBounds" | "outOfBounds" | "unbound" | "invalidType";
}

interface TreeNode {
  id: string;
  value: VizScalar;
  children: Array<{
    field: string;
    index: number | null; // 单节点字段为 null，子节点列表使用原始索引
    nodeId: string | null; // 显式保存空槽
  }>;
}

interface TreeState {
  rootId: string | null;
  nodes: TreeNode[];
}

interface TreePointer {
  name: string;
  nodeId: string | null;
  status: "matched" | "null" | "outside" | "unbound" | "invalidType";
}

interface TracePosition {
  cellLine: number;
  functionName: string;
  frameId: string;
  scopeFrameId: string;
  documentLine?: number; // Rust 按权威解析结果补充的文档全局行号；直接 python_run 时省略
}

interface FrameBase {
  seq: number;
  event: "call" | "line" | "return" | "exception";
  position: TracePosition;
  status: "ready" | "unbound" | "invalid";
  diagnosticCodes: string[];
}

type VisualizationFrame = FrameBase & (
  | { type: "array"; state: ArrayState | null; pointers: ArrayPointer[] }
  | { type: "tree"; state: TreeState | null; pointers: TreePointer[] }
);
```

`ready` 必须携带对应 state；其他状态的 state 为 null，不用空数组或空树掩盖读取失败。指针的 `unbound` 或 `invalidType` 不使有效主体失效。无法编码的指针原值使用 `raw: null`，通过 status 与合法的 null 值区分；整数越界时仍保留可编码的原始值。

树快照中的节点 ID 必须唯一，所有非空连边都指向本帧存在的节点，根以外每个节点恰有一个父位置，全部节点从根可达。`rootId: null` 时 nodes 必须为空。

### 7.3 配置与最终结果

```ts
interface VisualizationConfig {
  type: "array" | "tree";
  watch: string;
  pointers: string[];
  scope: string; // "module" 或唯一顶层函数名
  tree?: { valueField: string; childFields: string[] };
}

interface VisualizationDiagnostic {
  code: string;
  message: string;
  attribute?: string; // 配置错误对应的围栏属性
  seq?: number;
}

interface VisualizationTrace {
  schemaVersion: 1;
  runId: string;
  cellId: string;
  config: VisualizationConfig;
  frames: VisualizationFrame[];
  capture: {
    status: "complete" | "truncated" | "error";
    reason?: string;
  };
  diagnostics: VisualizationDiagnostic[];
}
```

`tree` 配置在 tree 类型下必填，在 array 下禁止。执行前错误没有可用 trace 时，仍通过现有 RuntimeError 返回配置错误。

前后端的 `CodeCell` 同时增加可选字段 `visualization?: VisualizationConfig` 和 `visualizationDiagnostics?: VisualizationDiagnostic[]`。无可视化声明时两者均省略；配置无效时保留诊断，不生成可执行配置。Rust 模型使用相同的 camelCase JSON 字段。

内部 `PythonRunRequest` 与前端直接运行接口的 `RunRequest` 增加 `visualization?: VisualizationConfig`。`RunCodeCellRequest` 仍只提交源码、cellId 和已有文件上下文，执行配置由 Rust 解析后填入。直接调用 `python_run` 的配置也必须经过同一验证，不能成为绕过验证的第二条路径。

将最终 trace 作为独立 RichOutput 返回，普通 stdout、stderr 保持各自通道：

```ts
{
  kind: "json",
  mime: "application/vnd.liveleaf.visualization+json",
  value: JSON.stringify(trace)
}
```

该 MIME 为项目内部约定。一次运行最多返回一个此类输出。未知 `schemaVersion` 显示不支持的版本，不能强行按 v1 播放。执行成功与采集完整是两个维度：程序可能成功但记录被截断，也可能抛出异常但直到异常退出的记录完整。

## 8. 子进程传输与资源预算

第一版使用运行目录中的 `trace.jsonl`，不把追踪事件混入 stdout，也不使用特殊打印前缀提取事件。用户打印 JSON 或任意文本不应影响协议。

文件按顺序包含 `header`、零或多个 `frame` / `diagnostic`、一个 `captureEnd` 记录。header 包含版本、runId、cellId 和规范化配置；frame 的 seq 从 0 连续递增。运行标识由 Rust 生成，用于防止界面误接旧运行结果，不作为权限凭据。

每条记录都是独立完整的 JSON 行，写入后 flush。诊断可以在状态帧中引用，完整消息按 code 与上下文去重保存。受预算截断时先写 `captureEnd` 并关闭采集，随后用户代码继续运行；采集结束不等于用户程序结束。

| 限制 | 第一版默认上限 | 超出处理 |
| --- | --- | --- |
| 状态帧数 | 2,000 | 停止采集，原因 `frameLimit` |
| JSONL 总字节数 | 8 MiB | 停止采集，原因 `byteLimit` |
| 单条 JSONL 记录 | 256 KiB | 停止采集，原因 `recordLimit` |
| 数组元素 | 256 | 停止采集，原因 `arrayLimit` |
| 单帧树节点数 | 128 | 停止采集，原因 `nodeLimit` |
| 单帧子节点槽位总数 | 512 | 停止采集，原因 `edgeLimit`，空槽也计数 |
| 树深度 | 32，根深度为 0 | 停止采集，原因 `depthLimit` |
| 运行内注册对象数 | 512 | 停止采集，原因 `objectLimit` |
| 诊断条数 | 32 | 去重后合并为一条汇总诊断 |

这些是实现默认值，不在第一版围栏语法中开放调整。写入预算需为 header、截断原因和结束记录预留空间；先计算候选记录字节数再写入。扫描数组长度、孩子列表长度和遍历队列也必须有界，不能等完整构造超大 JSON 后才发现超限。

Rust 在进程退出或被超时终止后、删除运行目录之前读取文件并进行有界验证：

1. 校验实际文件大小、每行大小、版本、运行标识和配置一致性。
2. 校验记录顺序、seq、标量、节点与指针引用，以及各类数量上限。
3. 只接受最后一个有效连续前缀；尾部半行直接丢弃，损坏记录之后的内容不继续拼接。
4. 缺少结束记录且进程超时或异常终止时，标记 `truncated` 并注明原因；进程正常成功却缺少结束记录时标记采集 `error`。
5. 文件不存在或 header 无效时显示采集错误，仍保留正常的 stdout、stderr 和进程结果。

runner 的校验与 Rust 的校验必须同时存在。前端也需验证 JSON 输出，不执行其中的 HTML、脚本或属性表达式。

## 9. 前端回放行为

每个代码块的输出区包含普通输出和一个共用的追踪播放器。文档模式与演示模式复用同一播放器及数据类型，不分别实现解析逻辑。

- 开始回放时先定位第一个 `ready` 帧；没有有效帧则展示对应诊断。
- 前后单步移动一个采集事件；进度定位直接读取完整快照，无需重新执行 Python。
- 源码按事件语义显示“即将执行”或“离开调用”，不暗示已经执行当前高亮行。
- 无效帧明确显示当前主体不可用；如保留上一幅图作为上下文，必须标为“上一有效状态”。
- 显示作用域调用切换；只在相同主体身份及兼容上下文之间计算变化高亮。
- Array 使用带索引的单元格，变化位置突出显示；不能因元素是数字就默认用高度编码。
- Tree 使用稳定节点 ID 和有序父子布局。二叉树保留左右槽位，多叉树保持孩子顺序；同一结构的连续帧复用位置，减少跳动。
- 树布局随结构变化可以过渡，值变化只更新标签。颜色之外同时使用指针名、边标签或状态文字，支持减少动画偏好。
- 窄屏数组允许在该视图内横向滚动；较大树提供局部缩放与平移，不挤压整个课程页面。
- 超时或截断时展示已记录部分，并停止在最后一帧；不能显示成算法已经正常完成。
- 重跑会创建新 runId，替换该代码块上一次的回放结果；前端在发起请求前更新本地请求序号，只应用仍匹配该序号的返回值，避免旧请求迟到时覆盖新结果。不能仅比较收到响应的先后顺序。
- 编辑代码或观察属性后，已有结果标记过期。播放器保存运行时源码快照；不能把旧行号高亮到新源码上。

第一版记录仅保存在内存中的 RunResult，不增加 `.ll.md` 写回、项目包输出持久化或跨重启恢复。未来持久化应另行定义版本与源码关联规则。

## 10. 错误与兼容策略

| 情况 | 用户代码 | 可视化结果 |
| --- | --- | --- |
| 配置语法错误或指定作用域定义无效 | 不执行 | 返回 `VisualizationConfigError`，定位属性 |
| 用户代码 Python 语法错误 | 不执行 | 保留 SyntaxError 与源码位置，无伪造状态 |
| 目标变量尚未赋值 | 正常执行 | 当前帧 unbound |
| 变量整个运行均未绑定 | 正常执行 | 额外汇总 `WatchNeverBound` |
| 指定函数未调用 | 正常执行 | `ScopeNotEntered`，无状态帧 |
| 主体类型错误、缺少节点字段、非树结构 | 正常执行 | 当前帧 invalid，可随后恢复 |
| 指针错误或越界 | 正常执行 | 主体继续显示，指针附带状态 |
| 用户代码运行时异常 | 按原有语义失败 | 保留已采集记录与异常位置 |
| 运行超时 | Rust 按现有规则终止子进程 | 回收有效前缀，显示 timeout |
| 达到采集预算 | 继续执行，仍受运行超时约束 | truncated，显示具体原因 |
| 解释器缺少所需追踪能力、采集器故障 | 保留普通执行能力 | 关闭采集，明确显示采集错误 |
| 用户代码关闭或替换追踪器 | 按用户代码继续执行 | 标记 TraceModified，不宣称完整 |

候选运行环境为 CPython 3.10 及以上，通过能力检查启用；实现验收需覆盖实际支持的解释器版本，不能由版本号推断所有解释器实现均可追踪。`sys.settrace` 属于执行环境相关能力。[Python 兼容性说明](https://docs.python.org/3.13/library/sys.html#sys.settrace)

Hook 用于观察本地代码，不构成安全沙箱。沿用现有“用户点击运行、独立子进程、超时”的执行边界，不自动运行打开的课程，也不为可视化新增网络访问。

## 11. 模块划分与实现顺序

建议新增以下模块，避免把追踪器、数据读取和播放器继续堆入现有入口文件：

```text
src-tauri/python/
  runner.py
  liveleaf_viz/
    collector.py
    values.py
    array.py
    tree.py
src-tauri/src/runtime/
  visualization.rs
src/runtime/
  visualization.ts
src/visualization/
  TracePlayer.vue
  ArrayView.vue
  TreeView.vue
```

Python 源码作为应用内置资源随构建提供，运行时释放到本次目录，不依赖用户安装额外可视化包。Python 采集器只产生状态；Rust 管理进程和验证协议；Vue 负责布局、变化高亮与回放。

按以下顺序交付：

1. **语法与协议**：扩展两端 CodeCell 模型、配置验证、共享 fixture 和输出类型，保留普通代码运行路径。
2. **Runner 与 array**：安装 hook、确定源码行语义、生成 JSONL、处理限额和异常，提供数组回放。
3. **Tree**：实现字段适配、稳定身份、结构校验、有序布局与节点指针，复用相同播放器。
4. **集成验收**：覆盖文档与演示模式、重跑和源码过期、错误展示，添加课程示例与作者语法说明。

## 12. 验收标准

### 12.1 语法与执行链路

- 前端与 Rust 使用同一组课程 fixture，得到相同的规范化配置和错误；覆盖引号、空白、重复属性、未知类型和非法表达式。
- Rust 按收到的 Markdown 源码重新解析配置，不采信前端额外提交的观察对象或代码。
- 普通 Python 块的 stdout、stderr、超时与退出行为保持兼容。
- 用户命名为 `trace`、`config` 等变量时不与 runner 冲突；`__future__` 导入、普通本地导入及模块相关类定义可执行。
- 多幻灯片、CRLF、空行和嵌套围栏的高亮行号映射正确；最后一条赋值的结果可见。

### 12.2 Array

- 走楼梯最终状态为 `[1, 1, 2, 3, 5, 8]`，stdout 为 `8`；赋值前后高亮语义正确。
- 冒泡排序的交换、通过别名修改、追加和删除均可回放；历史快照不随后续修改变化。
- 相同内容重新绑定仍产生新的 objectId；空数组与未绑定能区分。
- 指针覆盖负索引、越界、多指针重叠、未绑定、bool 和无效类型。
- 大整数不损失精度，非有限浮点数可通过标准 JSON 往返；不支持值有明确诊断。

### 12.3 Tree

- 覆盖普通对象二叉树、字典多叉树、空树、仅右孩子、列表空槽和重复节点值。
- 修改值不改变节点 ID；根重新绑定、节点断开后接回、指针移入或移出树均正确。
- 环和共享子节点受控报错，修复后继续观察；深树和超大孩子列表不导致采集器无界遍历。
- 读取 property、描述符、特殊转换方法时不执行用户钩子，而是报告不支持。
- 函数与递归作用域切换保留调用身份，局部变量未绑定时不误读同名全局变量。

### 12.4 失败恢复与界面

- 运行异常或超时后仍能播放最后有效前缀；半行 JSON、错序记录、未知版本与无效节点引用不会导致崩溃。
- 达到采集上限后用户代码仍能继续产生正常 stdout，并最终受原运行超时控制。
- 进度定位与逆向单步不执行 Python；播放暂停不宣称暂停了用户程序。
- 文档和演示模式结果一致，重跑不混入旧结果，编辑后不在新源码上高亮旧行号。
- 关闭应用或重启后不声称保留尚未实现的回放历史。

## 13. 后续扩展

有向图现已作为兼容扩展实现，详见 [有向图扩展协议](./python-graph-visualization.md) 与 [Dijkstra 可运行示例](./examples/dijkstra.ll.md)。下文保留原设计的后续方向。

在 array、tree 的采集协议与回放稳定后，再考虑多观察对象、二维矩阵、图、用户适配器、实时传输及会话运行。精确的比较、交换、访问完成等语义事件，以及动态规划依赖箭头，需要单独设计 AST 插桩或受限的声明式关系；不通过扩大 `watch` 的求值能力隐式加入。
