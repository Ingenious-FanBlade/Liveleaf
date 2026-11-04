# Python Hook · 算法可视化

点击 Run 执行普通 Python 代码，执行结束后可以播放、暂停、前后单步或拖动进度条。黄色表示相邻兼容状态中的变化，蓝色边框表示首个观察变量的位置，橙色边框表示其他观察变量的位置。图内只保留数值、节点名和数组下标；悬浮或展开“观察详情”可查看变量信息。播放器中的高亮行表示“即将执行”，代码已在回放前执行完毕。

```python run id="climb-stairs" visualize="array" watch="dp" pointers="i"
n = 5
dp = [0] * (n + 1)
dp[0] = dp[1] = 1

for i in range(2, n + 1):
    dp[i] = dp[i - 1] + dp[i - 2]

print(dp[n])
```

最终数组为 `[1, 1, 2, 3, 5, 8]`，输出 `8`。

---

# 二叉树的先序遍历

先序遍历按照 **根 → 左子树 → 右子树** 访问节点。下面的 `node` 指针跟随最内层 `preorder` 调用；递归返回时，指针恢复到外层调用中的节点。

```text
        A
       / \
      B   C
     / \   \
    D   E   F
```

```python run id="binary-tree-preorder" visualize="tree" watch="root" value="val" children="left,right" pointers="node" scope="preorder" timeout=5000
class Node:
    def __init__(self, val, left=None, right=None):
        self.val = val
        self.left = left
        self.right = right

root = Node("A",
            Node("B", Node("D"), Node("E")),
            Node("C", None, Node("F")))
visited = []

def preorder(node):
    if node is None:
        return
    visited.append(node.val)
    preorder(node.left)
    preorder(node.right)

preorder(root)
print(" → ".join(visited))
```

预期输出：`A → B → D → E → C → F`。

回放中可观察蓝色边框从 A 进入 B、D，返回后进入 E，再进入 C、F。F 保持在 C 的右孩子位置。空子树调用时没有节点被蓝框标记，“观察详情”中显示“空引用”。`node` 仅代表当前引用，访问顺序由算法中的 `visited` 和最终输出说明。

::: tip
将进度条拖到递归返回附近，观察作用域切换提示和蓝色边框的位置。每次调用都有独立身份；回放不会再次执行 Python。
:::

本地可视化支持 CPython 3.10 及以上的单线程同步代码；不追踪导入库内部、额外线程或 C 扩展的内部步骤。生成器与异步控制流会在执行前拒绝。记录只在当前应用内存中保存。
