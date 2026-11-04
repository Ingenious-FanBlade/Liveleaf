use serde_json::{json, Value};

pub(super) fn spec() -> Value {
    json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "syntaxVersion": 1,
        "sourceFormat": ".ll.md",
        "packageFormat": ".llpkg",
        "slideSeparator": "--- (standalone, outside code fences)",
        "directives": crate::validation::DIRECTIVES,
        "layouts": { "columns": "::: columns\n::: column width=45%\nContent\n:::\n::: column width=55%\nContent\n:::\n:::" },
        "code": {
            "cellKinds": ["run", "exercise"],
            "runtimeLanguages": ["python"],
            "staticLanguages": "Any language; non-Python runnable cells are rejected by validation.",
            "attributes": {
                "id": {"type": "string", "unique": true, "default": "slide-N-cell-M; explicit IDs recommended"},
                "timeout": {"type": "integer", "unit": "ms", "minimum": 1, "maximum": 3600000, "default": 5000},
                "session": {"type": "string", "sharedState": false, "description": "Passed as LIVELEAF_SESSION_ID; each run still uses a fresh Python process."}
            },
            "stdin": false,
            "workingDirectory": "Source file's directory; --base-dir or current directory for stdin input.",
            "sandboxed": false
        },
        "visualization": {
            "types": ["array", "tree", "graph", "dynamic-table"],
            "attributes": ["visualize", "watch", "pointers", "scope", "value", "children", "node_states", "edge_states", "tables"],
            "watch": "Required ASCII Python identifier; selects the variable to observe.",
            "pointers": "Comma-separated unique Python identifiers; at most 8; unsupported for dynamic-table.",
            "scope": {"default": "module", "description": "module or a Python function name"},
            "tree": {"value": "value", "children": "left,right"},
            "graph": {"node_states": "Optional dictionary variable", "edge_states": "Optional dictionary variable"},
            "tables": "Optional comma-separated variable names for independent dynamic tables.",
            "execution": "CPython 3.10+, single-threaded synchronous code; replay is collected after execution.",
            "outputMime": crate::runtime::visualization::MIME,
            "dataContracts": {
                "array": "A list of scalar values.",
                "tree": "A node object/dictionary with a scalar value and child fields, or None.",
                "graph": "An adjacency dictionary: {node: {neighbor: weight}}.",
                "dynamic-table": "A list/tuple of scalar records, rows (short rows padded with None), or scalar values; row forms must be consistent."
            }
        },
        "media": {
            "image": "![label](./assets/image.png)",
            "video": "::video[label](./assets/video.mp4){controls start=12 width=80% poster=./assets/poster.png}",
            "audio": "::audio[label](./assets/audio.mp3){controls}",
            "resourcePolicy": "Local resources must stay inside the source directory, including after symlink resolution. External URLs are retained, not downloaded."
        },
        "unsupported": ["Shared Python sessions", "Frontmatter metadata interpretation", "Presenter notes", "Exercise tests attribute", "Embedded raw HTML"],
        "validation": {"executesCode": false, "checksPythonSyntax": false, "checksLayout": false, "checksRemoteUrls": false},
        "examples": {
            "lesson": "# Variables\n\n```python run id=hello timeout=5000\nname = 'Ada'\nprint(name)\n```\n\n---\n\n# Practice\n\n```python exercise id=practice\n# TODO: print your name\n```\n\n::: solution collapsed\nUse print(name).\n:::\n",
            "array": "```python run id=array-demo visualize=array watch=values pointers=i\nvalues = [3, 1, 2]\nfor i in range(len(values)):\n    values[i] *= 2\nprint(values)\n```",
            "tree": "```python run id=tree-demo visualize=tree watch=root\nroot = {'value': 'A', 'left': None, 'right': None}\nprint(root['value'])\n```",
            "graph": "```python run id=graph-demo visualize=graph watch=graph\ngraph = {'A': {'B': 2}, 'B': {}}\nprint(graph)\n```",
            "dynamic-table": "```python run id=table-demo visualize=dynamic-table watch=rows\nrows = [{'name': 'A', 'distance': 0}]\nrows[0]['distance'] = 2\nprint(rows)\n```"
        }
    })
}
