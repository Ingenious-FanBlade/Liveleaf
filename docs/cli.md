# Liveleaf CLI

CLI 为外部 AI、脚本和自动化提供无界面的课件操作入口。它与桌面端共享 Rust 解析器、代码块执行入口、Python 运行器和 `.llpkg` 打包器，不启动 Tauri 窗口，也不需要 Vite 服务。

## 构建与启动

```powershell
npm run cli:build
.\src-tauri\target\release\liveleaf-cli.exe --help
```

构建需要现有项目的 Rust/Tauri 构建环境。生成的 CLI 是单独的控制台可执行文件；当前构建命令不会自动把它加入桌面安装包。执行 Python 仍需要本机 Python，发现方式与桌面端一致（Windows 查找 `python`、`py`，其他平台查找 `python3`、`python`）。

开发时可以使用 `npm run cli -- spec --json`。AI 或其他程序应直接调用编译后的 CLI，避免 npm 的启动日志混入 stdout。Linux/macOS 的二进制没有 `.exe` 后缀。

## 指令

```powershell
# 当前实际支持的语法、运行能力、可视化约束和完整示例
liveleaf-cli spec --json

# 静态校验：不执行 Python，不写入文件，不请求外部 URL
liveleaf-cli validate lesson.ll.md --json

# 检查结构：包含幻灯片、代码块、源码位置、可视化配置和资源元数据
liveleaf-cli inspect lesson.ll.md --json

# 按 ID 运行一个代码块，返回 stdout、stderr、异常和可视化回放数据
liveleaf-cli run lesson.ll.md --cell hello --json

# 校验通过后，打包源码和本地资源
liveleaf-cli pack lesson.ll.md --output lesson.llpkg --json

# 显式替换已有项目包
liveleaf-cli pack lesson.ll.md --output lesson.llpkg --force --json
```

`--json` 可以放在指令之前或之后。`--output` 可缩写为 `-o`，带值参数也支持 `--cell=hello` 等写法。

除 `spec` 外，所有指令都接受一个 UTF-8 `.ll.md` / `.md` 源文件。输入应为课件源码，不能直接传 `.llpkg`。UTF-8 BOM 和 CRLF 换行会被处理，诊断行号从 1 开始。

## 标准输入

用 `--stdin` 或文件名 `-` 读取尚未保存的课件。可以通过 `--base-dir` 指定资源根目录和 Python 工作目录；默认使用 CLI 当前目录。

```powershell
Get-Content -Raw -Encoding utf8 lesson.ll.md |
  liveleaf-cli validate --stdin --base-dir D:\courses\lesson --json
```

从文件读取时，资源根目录和 Python 工作目录固定为源文件所在目录。相对输出路径相对于 CLI 当前目录。`--base-dir` 只适用于标准输入。

## JSON 协议

启用 `--json` 后，stdout 始终为一个 JSON 对象，包括参数错误、文件读取失败、校验失败和 Python 异常。Python 的输出被封装在 `data.result.stdout` / `data.result.stderr` 内，不会混入 CLI 输出。

```json
{
  "schemaVersion": 1,
  "command": "validate",
  "ok": false,
  "data": {
    "file": "D:\\courses\\lesson.ll.md",
    "baseDirectory": "D:\\courses",
    "title": "Example",
    "slideCount": 1,
    "codeCellCount": 1,
    "valid": false,
    "errorCount": 1,
    "warningCount": 0,
    "resources": []
  },
  "diagnostics": [
    {
      "severity": "error",
      "code": "InvalidTimeout",
      "message": "timeout must be an integer from 1 to 3600000 milliseconds.",
      "line": 3,
      "cellId": "hello",
      "attribute": "timeout",
      "resource": null
    }
  ],
  "error": {
    "code": "ValidationFailed",
    "message": "Course has validation errors. No code was executed and no package was written."
  }
}
```

- `schemaVersion` 标记 CLI 响应结构版本；语法能力另见 `spec` 的 `syntaxVersion`。
- `ok` 表示指令是否成功；成功时 `error` 为 null。
- `diagnostics` 包含静态错误和警告；无精确位置时 `line` 为 null。
- `inspect` 在 `data.document` 中返回解析器的文档模型，即使存在校验错误也保留结果。
- `run` 在 `data.result` 中返回与桌面端相同的运行结果，包括可视化 MIME 输出。
- `pack` 在 `data.package` 中返回输出路径、资源计数和打包警告。
- 参数、I/O 等操作错误通常返回 `data: null` 和具体的 `error.code` / `message`。

| 退出码 | 含义 |
| --- | --- |
| 0 | 成功，可能存在警告 |
| 1 | 课件校验失败、找不到目标代码块或 Python 执行失败/超时 |
| 2 | 参数、输入输出、运行环境或打包操作错误 |

## 校验范围与行为

静态校验包含：代码块 ID 非空且唯一、可运行语言、属性合法性与引号闭合、限时范围、可视化配置、代码围栏、教学容器及列布局的闭合关系，以及本地资源是否存在、是否越出项目目录。所有指令使用同一份校验结果；`run` 和 `pack` 在文档有错误时不会执行或写包。`inspect` 会同时返回结构和错误。

普通静态代码块允许任意语言。可运行代码块目前只接受 `python`。省略 ID 时仍使用解析器的自动 ID；AI 生成课件应使用明确、稳定的 ID。`session` 只传递会话名称，不会在独立运行之间保留 Python 状态。

静态校验不检查 Python 语法、算法正确性或页面溢出，也不会推断 `watch` 变量运行时是否有效。请分别用 `run` 检查执行结果，用桌面预览/Playwright 检查视觉效果。

本地资源遵循现有项目包规则：必须位于源文件目录内，符号链接解析后也不能越界。HTTP/HTTPS 等外部资源保留为 URL，并产生警告；不会联网验证或下载。资源内容会被读取以计算大小及 SHA-256，`validate` / `inspect` 不会创建项目包。代码示例中的媒体指令不会被当作真实资源。

`pack` 默认拒绝覆盖已有文件；`--force` 允许替换目标文件，但始终拒绝覆盖源文档。包先写入目标目录的临时文件，完成后才发布到目标路径。失败时清理临时包。`run` 会执行本机 Python，其权限、超时行为与桌面运行器相同，不是代码安全沙箱。

## 验证

```powershell
npm run cli:test
cargo test --manifest-path src-tauri/Cargo.toml
```

CLI 集成测试启动实际可执行文件，检查 JSON/退出码、静态校验、源码位置、资源边界、运行错误与超时、可视化输出和项目包内容。未安装 Python 时只跳过实际执行测试。
