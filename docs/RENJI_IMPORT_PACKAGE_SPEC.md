# 人纪 PDF 标准导入数据包规范

## 基本原则

《中医大全》主软件不解析 PDF，不读取 PDF 原文，也不直接处理 OCR、版面切分或图片资料。人纪 PDF 必须先由外部智能体或外部数据处理工具整理成标准 `import_manifest` 数据包，主软件只负责导入该标准包、生成导入计划、写入主条目或注解、重建搜索索引和支持回滚。

人纪系列默认：

- `import_intent`: `annotation_enrichment`
- 主文件优先使用 `json/annotation_items_import.json`
- 主条目和注解必须分离
- 主条目进入 canonical 文件，注解进入 annotation 文件
- 真实发布包根目录文件名必须是 `import_manifest.json`
- 样例包可以使用 `import_manifest.example.json` 方便辨识

## Manifest 要求

推荐结构：

```json
{
  "package_name": "renji_sample_private_import",
  "schema_version": "renji_import_package_v1",
  "import_profile": "renji_private_v1",
  "import_intent": "annotation_enrichment",
  "duplicate_policy": "attach_annotation",
  "files": [
    {
      "path": "json/annotation_items_import.json",
      "type": "annotation_items_v1",
      "target": "knowledge_annotations",
      "primary": true,
      "auto_stage": true
    }
  ]
}
```

## 主条目与注解分离

外部智能体必须先判断资料属于主条目还是注解：

- 主条目：药物、方剂、穴位、经络、经典篇章、经典条文等稳定知识对象。
- 注解：倪注、讲解、课件整理、学习笔记、页码摘录、段落说明等来源性资料。

主条目输出到：

- `json/canonical_items_import.json`
- `json/classic_chapters.json`
- `json/classic_passages.json`
- `json/formula_items_import.json`
- `json/acupoint_items_import.json`
- `json/meridian_items_import.json`

注解输出到：

- `json/annotation_items_import.json`

主软件导入时以注解文件为主文件，按 `canonical_key` 匹配已有主条目。匹配成功则写入 `knowledge_annotations`；匹配不到时不允许整批失败，可作为新条目或待确认项处理。

## annotation_items_v1 字段规范

必填字段：

- `canonical_key`: 标准主条目标识。
- `annotation_type`: 注解类型，建议 `ni_note`、`lecture_note`、`clinical_note`、`commentary`。
- `source_title`: 来源标题，例如 PDF 文件名或课程名。
- `source_note`: 来源页码、章节或段落位置。
- `content`: 注解正文。

推荐字段：

- `target_type`: 主条目类型。
- `target_name`: 主条目名称。
- `summary`: 注解摘要。
- `tags`: 字符串数组。
- `detail`: 对象，保留页码、章节、外部处理置信度等技术信息。

示例：

```json
{
  "canonical_key": "herb:人参",
  "target_type": "herb",
  "target_name": "人参",
  "annotation_type": "ni_note",
  "source_title": "神农本草经人纪讲义",
  "source_note": "renji_shennong_bencao_private_import.pdf p.12",
  "content": "倪注：人参用于学习整理的补气重点。",
  "tags": ["神农本草经", "人参", "倪注"]
}
```

## canonical_key 规范

`canonical_key` 使用小写类型前缀和中文名，类型与名称之间用英文冒号。

- `herb:人参`
- `formula:桂枝汤`
- `acupoint:足三里`
- `meridian:足阳明胃经`
- `classic_chapter:上古天真论篇第一`
- `classic_passage:伤寒论:太阳病:001`

经典条文可包含经典名、篇章名和条文号。主软件用于匹配时可优先使用条文篇章名，完整 key 必须保留在 `detail.canonical_key` 中，便于审计和后续精确关联。

## 五类 PDF 处理规则

### 针灸

输出对象：

- `meridian`: 经络主条目。
- `acupoint`: 穴位主条目。
- `annotation`: 针灸手法、补泻说明、倪注、学习笔记。

注解示例 key：

- `acupoint:足三里`
- `meridian:足阳明胃经`

### 黄帝内经

输出对象：

- `classic_chapter`: 篇章主条目。
- `classic_passage`: 原文条文。
- `annotation`: 讲解、段落说明、课程笔记。

注解示例 key：

- `classic_chapter:上古天真论篇第一`

### 神农本草经

输出对象：

- `herb`: 中药主条目。
- `annotation`: 本经讲解、倪注、学习笔记。

注解示例 key：

- `herb:人参`

### 伤寒论

输出对象：

- `classic_passage`: 条文主条目。
- `formula`: 方剂主条目。
- `annotation`: 条文讲解、方剂讲解、倪注。

注解示例 key：

- `classic_passage:伤寒论:太阳病:001`
- `formula:桂枝汤`

### 金匮要略

输出对象：

- `classic_passage`: 条文主条目。
- `formula`: 方剂主条目。
- `annotation`: 条文讲解、方剂讲解、倪注。

注解示例 key：

- `formula:理中丸`

## 禁止事项

- 不要把 PDF 放入主软件源码。
- 不要让主软件解析 PDF。
- 不要在主软件里真实调用 AI。
- 不要把注解混入主条目的 `content` 作为唯一来源。
- 不要让单条目标不存在导致整批导入失败。
