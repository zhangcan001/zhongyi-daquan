# 人纪样例导入说明

本目录是用于打通《中医大全》人纪数据包导入闭环的小样例，不包含 PDF，不调用 AI。

## 文件说明

- `import_manifest.example.json`：样例 manifest。真实包应命名为 `import_manifest.json`。
- `json/annotation_items_import.sample.json`：注解主文件，类型为 `annotation_items_v1`。
- `json/canonical_items_import.sample.json`：中药主条目样例。
- `json/classic_chapters.sample.json`：经典篇章主条目样例。
- `json/classic_passages.sample.json`：经典条文样例。
- `json/formula_items_import.sample.json`：方剂主条目样例。
- `json/acupoint_items_import.sample.json`：穴位主条目样例。
- `json/meridian_items_import.sample.json`：经络主条目样例。

## 验收方式

在软件的智能导入中心选择本目录，系统应识别 `import_manifest.example.json`，并把 `annotation_items_v1` 作为主文件生成 ImportPlan。

普通界面只应展示数据包名称、数据包类型、导入意图、新增主条目数量、附加注解数量、跳过重复数量、待确认数量、错误数量和是否可回滚。

技术字段、manifest 文件明细、primary、auxiliary、target、auto_stage 和动作明细只应出现在高级详情中。
