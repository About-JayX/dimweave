# 飞书项目 API 数据结构参考

> 来源：飞书项目开发者手册。本文件为 Dimweave 飞书 MCP 集成的字段参考。

## 目录

- [核心实体](#核心实体)
- [工作项与字段](#工作项与字段)
- [工作流（节点流 / 状态流）](#工作流)
- [评审与交付物](#评审与交付物)
- [视图与图表](#视图与图表)
- [团队与用户](#团队与用户)
- [排期与工时](#排期与工时)
- [搜索与筛选](#搜索与筛选)
- [资源库](#资源库)
- [其他](#其他)

---

## 核心实体

### Project（空间）

| 字段 | 类型 | 说明 |
|------|------|------|
| project_key | string | 空间 ID |
| name | string | 空间名 |
| simple_name | string | 空间域名（URL slug） |
| administrators | list\<string\> | 管理员 user_key 列表（仅管理员可见） |

### WorkItemInfo（工作项）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int64 | 工作项 ID |
| name | string | 工作项名称 |
| work_item_type_key | string | 工作项类型 key |
| project_key | string | 所属空间 ID |
| template_id | int64 | 模板 ID |
| pattern | string | 工作项模式：Node（节点）/ State（状态） |
| current_nodes | list\<NodeBasicInfo\> | 当前节点信息（状态模式为空） |
| created_by | string | 创建人 user_key |
| updated_by | string | 更新人 user_key |
| created_at | int64 | 创建时间戳（毫秒） |
| updated_at | int64 | 更新时间戳（毫秒） |
| deleted_by | string | 删除人 user_key |
| deleted_at | int64 | 删除时间戳（毫秒） |
| fields | list\<FieldValuePair\> | 其他字段 |
| work_item_status | WorkItemStatus | 工作项状态 |

### WorkItemKeyType（工作项类型）

| 字段 | 类型 | 说明 |
|------|------|------|
| type_key | string | 工作项类型 key |
| name | string | 类型名称 |
| api_name | string | 系统标识 |
| is_disable | int | 1=禁用, 2=启用 |

### WorkItemStatus（工作项状态）

| 字段 | 类型 | 说明 |
|------|------|------|
| state_key | string | 状态 key |
| is_archived_state | bool | 是否完成状态 |
| is_init_state | bool | 是否初始状态 |
| updated_at | int64 | 状态更新时间（毫秒） |
| updated_by | string | 状态更新人 user_key |

### WorkItemRelation（工作项关系）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 关系 ID |
| name | string | 关系名称 |
| relation_type | int | 0=普通关系, 1=父子关系 |
| disabled | bool | 是否禁用 |
| work_item_type_key | string | 本空间工作项类型 key |
| relation_details | list\<RelationDetail\> | 关联工作项 |

---

## 工作项与字段

### FieldValuePair（字段值对）

| 字段 | 类型 | 说明 |
|------|------|------|
| field_key | string | 字段 key（与 field_alias 二选一必填） |
| field_alias | string | 字段对接标识 |
| field_value | Object | 字段值，参考字段与属性解析格式 |
| field_type_key | string | 字段类型（非必填） |
| target_state | TargetState | 状态流流转目标（非必填） |

### SimpleField（字段元信息）

| 字段 | 类型 | 说明 |
|------|------|------|
| field_key | string | 字段 key |
| field_alias | string | 字段对接标识 |
| field_type_key | string | 字段类型 |
| field_name | string | 字段名称 |
| is_custom_field | bool | 是否自定义字段 |
| options | list\<Option\> | 选项 |
| relation_id | string | 关联关系 ID |
| compound_fields | list\<SimpleField\> | 复合子字段 |

### FieldConf（字段配置）

| 字段 | 类型 | 说明 |
|------|------|------|
| field_name | string | 字段名称 |
| field_key | string | 字段 key |
| field_type_key | string | 字段类型 |
| is_required | int64 | 1=必填, 2=非必填, 3=条件必填 |
| is_visibility | int64 | 1=可见, 2=条件可见 |
| is_validity | int64 | 有效性：1=有效, 3=条件 |
| default_value | DefaultValue | 默认值 |
| label | string | 表单项名称 |
| options | list\<OptionConf\> | 选项 |
| compound_fields | list\<FieldConf\> | 复合子字段 |

### FieldValue（选项值）

| 字段 | 类型 | 说明 |
|------|------|------|
| value | string | 选项 ID |
| label | string | 选项名称 |
| disabled | int | 1=禁用, 2=启用 |
| action | int | 0=新增, 1=修改, 2=删除（仅更新时） |
| children | list\<FieldValue\> | 级联子选项 |
| color | string | 选项颜色（red-1, blue-3 等） |

### Option（选项）

| 字段 | 类型 | 说明 |
|------|------|------|
| value | string | 选项 ID |
| label | string | 选项名称 |
| is_disabled | bool | 是否禁用 |
| children | list\<Option\> | 级联子选项 |
| order | int64 | 排序字段 |

### Business（业务线）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 业务线 ID |
| name | string | 业务线名称 |
| project | string | 所属空间 ID |
| labels | list | 标签数组 |
| role_owners | Map\<string, RoleOwner\> | 默认角色及负责人 |
| watchers | list | 默认关注者数组 |
| order | int64 | 排序字段 |
| super_masters | list | 业务线超级管理员数组 |
| parent | string | 父级业务线 ID |
| disabled | bool | 是否弃用 |
| level_id | int | 层级 ID（顶层=1） |

---

## 工作流

### WorkflowNode（工作流节点）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 节点 ID |
| state_key | string | 节点 key |
| name | string | 节点名称 |
| status | int | 1=未开始, 2=进行中, 3=已完成 |
| fields | list\<FieldValuePair\> | 节点完成表单字段 |
| owners | list\<string\> | 负责人 user_key |
| node_schedule | Schedule | 节点总排期 |
| schedules | list\<Schedule\> | 差异化排期 |
| sub_tasks | list\<SubTask\> | 子任务 |
| actual_begin_time | string | 实际开始时间 |
| actual_finish_time | string | 实际结束时间 |
| role_assignee | list\<RoleOwner\> | 角色负责人 |

### StateFlowNode（状态流节点）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 状态 ID |
| name | string | 状态名称 |
| status | int | 1=未开始, 2=进行中, 3=已完成 |
| owners | list\<RoleOwner\> | 负责人 |
| fields | list\<FieldValuePair\> | 字段 |
| actual_begin_time | string | 状态开始时间 |
| actual_finish_time | string | 状态结束时间 |

### StateFlowConf（状态流配置）

| 字段 | 类型 | 说明 |
|------|------|------|
| state_key | string | 状态 ID |
| name | string | 状态名称 |
| state_type | int | 1=起始, 2=过程, 3=归档 |
| authorized_roles | list\<string\> | 授权角色 key |

### Connection（状态连接）

| 字段 | 类型 | 说明 |
|------|------|------|
| source_state_key | string | 开始节点 ID |
| target_state_key | string | 目标节点 ID |
| transition_id | int64 | 状态流转 ID |

### TargetState

| 字段 | 类型 | 说明 |
|------|------|------|
| state_key | string | 目标状态 key |
| transition_id | int64 | 流转 ID |

### SubTask（子任务）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 任务 ID |
| name | string | 任务名称 |
| schedules | list\<Schedule\> | 排期 |
| order | double | 排序字段 |
| passed | bool | 是否通过 |
| owners | list\<string\> | 负责人 user_key |
| assignee | list\<string\> | 子任务负责人（非角色联动时） |
| role_assignee | list\<RoleOwner\> | 角色负责人（角色联动时） |
| deliverable | list\<FieldValuePair\> | 交付物 |
| fields | list\<FieldValuePair\> | 自定义字段 |

---

## 评审与交付物

### FinishedInfoItem（评审信息）

| 字段 | 类型 | 说明 |
|------|------|------|
| node_id | string | 节点 ID |
| summary_mode | int64 | 汇总模式：calculation / independence |
| opinion | FinishedOpinionInfo | 评审意见 |
| conclusion | FinishedConclusionInfo | 评审结论 |

### Deliverable（交付物）

| 字段 | 类型 | 说明 |
|------|------|------|
| deliverable_uuid | string | 交付物唯一标识 |
| deliverable_type | string | work_item_deliverable / field_deliverable |
| deliverable_info | DeliverableInfo | 交付物信息 |

### Comment（评论）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int | 评论 ID |
| work_item_id | int64 | 所属工作项 ID |
| work_item_type_key | string | 所属工作项类型 |
| created_at | int | 创建时间（毫秒） |
| operator | string | 评论人 |
| content | string | 评论内容 |

---

## 视图与图表

### ViewConf（视图配置）

| 字段 | 类型 | 说明 |
|------|------|------|
| view_id | string | 视图 ID |
| view_url | string | 视图 URL |
| name | string | 视图名称 |
| view_type | int64 | 0=未知, 1=条件视图, 2=固定视图 |
| auth | int64 | 1=开放, 2=关闭 |
| system_view | int64 | 1=系统视图, 2=非系统视图 |
| collaborators | list\<string\> | 协作者 |
| created_at | int64 | 创建时间 |
| created_by | string | 创建者 |
| quick_filters | list\<QuickFilter\> | 快捷筛选条件 |

---

## 团队与用户

### Team（团队）

| 字段 | 类型 | 说明 |
|------|------|------|
| team_id | int | 团队 ID |
| team_name | string | 团队名称 |
| user_keys | list\<string\> | 人员列表 |
| administrators | list\<string\> | 管理员列表 |

### UserDetail（用户详情）

| 字段 | 类型 | 说明 |
|------|------|------|
| user_key | string | 用户 ID |
| username | string | 用户名称 |
| email | string | 邮箱 |
| name_cn | string | 中文名 |
| name_en | string | 英文名 |

### Dimweave 负责人下拉：单 team 优化

当前实现用项目 team 成员代替 MQL `GROUP BY current_status_operator`（后者只返回 top-N 子集）。

**调用链（3 次 MCP tool call）：**

1. `list_project_team(project_key)` → 解析 `data[].{name, team_id}`
2. 选择与项目名后缀匹配的单个 team（project name 通过 `--` 分割后缀，匹配 team name 前缀）
3. `list_team_members(team_id, page_size=200)` → 解析 `members[].user_key`
4. `search_user_info(user_keys)` → 解析顶层数组 `[].name_cn`

**真实响应结构注意：**

- `list_team_members` 顶层字段是 `members`（不是 `data`）
- `search_user_info` 返回顶层 JSON 数组（不是 `{data: [...]}`）

如果没有匹配的 team（项目名没有 `--` 分隔符，或无 team 前缀匹配），返回空列表。

### RoleOwner（角色负责人）

| 字段 | 类型 | 说明 |
|------|------|------|
| role | string | 角色 ID |
| name | string | 角色名称 |
| owners | list\<string\> | 负责人 user_key |

### RoleConfDetail（角色配置详情）

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 角色 ID |
| name | string | 角色名称 |
| is_owner | bool | 是否需求负责人 |
| role_appear_mode | int | 1=默认出现, 2=默认不出现, 3=条件出现 |
| member_assign_mode | int | 1=自行添加, 2=指定人员, 3=创建人, 4=按条件 |
| members | list\<string\> | 指定分配成员 |
| is_member_multi | bool | 是否允许多人 |
| role_alias | string | 角色对接标识 |
| lock_app_id | string | 锁定插件 ID |
| lock_scope | list\<string\> | 锁定范围 |

---

## 排期与工时

### Schedule（排期）

| 字段 | 类型 | 说明 |
|------|------|------|
| owners | list\<string\> | 负责人 user_key |
| estimate_start_date | int64 | 排期开始时间（毫秒，0=未填写） |
| estimate_end_date | int64 | 排期结束时间（毫秒，0=未填写） |
| points | float64 | 估分 |
| is_auto | bool | 自动补充模式（默认 true） |
| actual_work_time | float64 | 实际工时（精确到小数点后一位） |

### CreateWorkingHourRecord（创建工时记录）

| 字段 | 类型 | 说明 |
|------|------|------|
| resource_type | string | sub_task / node |
| resource_id | string | 关联对象 ID |
| work_time | string | 实际工时 |
| work_description | string | 工作描述 |

---

## 搜索与筛选

### SearchParam（搜索参数）

| 字段 | 类型 | 说明 |
|------|------|------|
| param_key | string | 字段 key |
| value | interface{} | 搜索值 |
| operator | string | 操作符 |
| pre_operator | string | 前置操作符（复合字段） |

### SearchUser（用户搜索）

| 字段 | 类型 | 说明 |
|------|------|------|
| user_keys | list\<string\> | 用户 user_key 列表 |
| field_key | string | 人员字段 key（创建人/关注人/经办人/报告人） |
| role | string | 角色 ID |

### PagedWorkItemIds（分页工作项）

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_ids | list\<int64\> | 工作项 ID 列表 |
| page_num | int64 | 当前页码 |
| page_size | int64 | 页大小 |
| total | int64 | 总数 |

### MQL 语法

```sql
SELECT fieldList
FROM `project_key`.`工作项类型名`
WHERE conditionExpression
[ORDER BY fieldList [ASC|DESC]]
[LIMIT [offset,] row_count]
```

- `LIMIT row_count` — 返回指定行数
- `LIMIT offset, row_count` — 从偏移位置开始
- 实际每页上限 **50 条**（服务端限制，不受 LIMIT 值控制）
- 内置函数：`current_login_user()` 返回当前用户
- `SearchParam.key` 和空间字段的 `field_key` 是相同的（官方 FAQ 原文）

### issue 类型实测字段映射（workspace `manciyuan`，2026-04-10）

> 通过 `list_workitem_field_config(project_key=manciyuan, work_item_type=issue)` 实测获取。

人员相关系统字段：

| field_key | field_name | field_type | 说明 |
|-----------|-----------|------------|------|
| `current_status_operator` | 当前负责人 | multi-user | 当前状态的负责人，即实际经办人 |
| `current_status_operator_role` | 当前状态授权角色 | multi-select | 当前状态授权的角色列表 |
| `owner` | 创建者 | user | 工作项创建人 |
| `updated_by` | 更新人 | user | 最近更新人 |
| `watchers` | 关注人 | multi-user | 关注者列表 |

注意：`operator` **不是** `issue` 类型下的合法 `field_key`，不能用于 MQL SELECT/WHERE。

### field_key 与 role_id 的区别（2026-04-10 实测）

- **field_key**（如 `current_status_operator`）：用于 MQL SELECT/WHERE/GROUP BY，来源于 `list_workitem_field_config`。
- **role_id**（如 `operator`、`reporter`）：用于角色配置，来源于 `list_workitem_role_config`；出现在 `get_workitem_brief` 返回的 `work_item_attribute.role_members` 中。

实测发现 `current_status_operator`（当前负责人）在实际 issue 样本中返回的人员与 `role_members.reporter` 一致，并非真正的经办人。真正的经办人在 `get_workitem_brief(...).work_item_attribute.role_members.operator`。

当前实现策略：MQL 仅用于 issue 列表发现（ID/标题/状态），assignee 从 `get_workitem_brief` 详情的 `role_members.operator` 获取，team_members 从已充实的 assignee 聚合派生。

### 服务端过滤实现（2026-04-10 实测）

#### 状态过滤：`work_item_status` 标签值 MQL

`work_item_status` 是合法的 MQL `field_key`，可用于 WHERE 条件：

```
work_item_type_key = "bug" AND work_item_status = "已关闭"
```

**关键发现：** MQL 中 `work_item_status` 的值必须使用**状态标签**（如 `已关闭`、`处理中`），而非内部 key（如 `CLOSED`）。标签值来源于 `group_by(work_item_status)` 返回的 `display_value`。

当前实现通过 `fetch_status_options()` 调用 `group_by(work_item_status)` 获取可用状态标签列表，前端下拉菜单直接使用这些标签值构建 MQL WHERE 子句。

#### 经办人过滤：团队成员 API + 详情匹配

经办人（`operator`）**不是** MQL `field_key`，无法在 MQL WHERE 中过滤。当前实现采用两阶段方案：

1. **全局经办人选项**：通过 `list_project_team` → `list_team_members(page_token)` → `search_user_info(user_keys)` 获取项目团队成员名称列表，而非从已加载的 issue 中聚合。这样即使 issue 列表为空或仅加载了部分页，经办人下拉也能显示完整选项。
2. **逐页匹配过滤**：`scan_assignee_page()` 从 MQL 查询结果中逐页加载 issue，对每条 issue 调用 `get_workitem_brief` 获取详情中的 `role_members.operator`，与目标经办人比较。匹配项累积到一页（50 条）后返回，未匹配的 issue 被跳过但游标继续前进。

游标状态（`IssueQueryCursor`）保存当前过滤条件和原始偏移量：当过滤条件变化时游标重置，条件不变时继续上次扫描位置。

---

## 资源库

### ResourceWorkitemInfo（资源实例）

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 资源实例 ID |
| name | string | 资源实例名称 |
| work_item_type_key | string | 工作项类型 key |
| project_key | string | 所属空间 key |
| created_by | UserDetail | 创建人 |
| updated_by | UserDetail | 更新人 |
| owner | UserDetail | 负责人 |
| role_owners | list\<RoleOwner\> | 角色信息 |

---

## 其他

### MultiText（富文本）

| 字段 | 类型 | 说明 |
|------|------|------|
| field_key | string | 字段 key |
| field_value | string | HTML 内容 |

### TimeInterval（时间区间）

| 字段 | 类型 | 说明 |
|------|------|------|
| start | int64 | 开始时间（毫秒） |
| end | int64 | 结束时间（毫秒） |

### WFState.status 对应表

| 状态 | 值 |
|------|-----|
| UNREACH（未开始） | 1 |
| REACHED（进行中） | 2 |
| PASSED（已完成） | 3 |

### NumberConfig（数字字段配置）

| 字段 | 类型 | 说明 |
|------|------|------|
| scaling_ratio | string | "1"=不缩放, "0.01"=百分比, "10000"=万 |
| display_digits | int64 | -1=不限, 0=整数, 1-6=小数位数 |
| thousandth | bool | 是否开启千分位 |

### AI_info

| 字段 | 类型 | 说明 |
|------|------|------|
| props | list\<props\> | AI 属性列表 |
| status | string | AI 运行状态 |
| app_identity | object | AI 应用标识（app_key + point_key） |

---

## 补充数据结构

> 以下为原始文档中的完整数据结构定义，确保无遗漏。


### ConfirmForm

| 字段 | 类型 | 说明 |
|------|------|------|
| action | int | 操作类型：1 为新增，2 为删除。当执行删除操作时，仅需提供 state_key。 |
| state_key | string | 状态 ID |

### DeliveryRelatedInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| 交付物关联挂载工作项信息 |  | DeliveryRelatedInfo 字段 类型 说明 |

### DeliveryRelatedInfoItem

| 字段 | 类型 | 说明 |
|------|------|------|
| project_key | string | 工作项的空间 key |
| work_item_id | int64 | 工作项 ID |
| work_item_type_key | string | 工作项类型 |
| name | string | 工作项名称 |

### ExPand

| 字段 | 类型 | 说明 |
|------|------|------|
| need_workflow | bool | 是否需要返回工作流信息（仅工作流模式可以使用） |
| need_multi_text | bool | 是否需要返回富文本信息 |
| relation_fields_detail | bool | 是否需要返回工作项关联详细信息 |
| need_user_detail | bool | 是否需要返回用户信息 |
| need_union_deliverable | bool | 是否融合需要交付物 |
| need_wbs_relation_chain_entity | bool | 是否需要返回 WBS 链路实例 |
| need_wbs_relation_chain_path | bool | 是否需要返回 WBS 链路层级 |
| need_group_uuid_for_compound | bool | 是否需要返回复合字段补充组标识 |

### FinishedConclusionInfoItem

| 字段 | 类型 | 说明 |
|------|------|------|
| 结论 |  | FixView 字段 类型 说明 |
| view_id | string | 视图 ID |
| name | string | 视图名称 |
| created_by | string | 创建人 user_key |
| created_at | int | 创建时间，毫秒精度 |
| modified_by | string | 最后一次修改人 user_key |
| work_item_id_list | list | 固定视图包含的工作项 ID 列表 |
| editable | bool | 当前视图是否可编辑 |

### FinishedConclusionOption

| 字段 | 类型 | 说明 |
|------|------|------|
| node_id | string | 节点名称 |
| finished_conclusion_option | list<FinishedConclusionResultItem> |  配置的统一评审结论 Label，计划废弃。使用finished_owners_conclusion_option和finished_overall_conclusion_option替代；如果整体配置开启则和finished_overall_conclusion_option一致；如果整体不开启而负责人配置开启，则和finished_owners_conclusion_option一致 |
| finished_owners_conclusion_option | list<FinishedConclusionResultItem> | 配置的负责人评审结论 Label |
| finished_overall_conclusion_option | list<FinishedConclusionResultItem> | 配置的整体评审结论 Label |

### FinishedConclusionResultItem

| 字段 | 类型 | 说明 |
|------|------|------|
| key | string | 选项 key |
| label | string | 选项标签 |
| origin_label | string | 选项原始标签 |

### FinishedOwnerConclusionInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| owner | string | 用户 user_key |

### FinishedOwnerOpinionInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| owner | string | 用户 user_key |
| finished_opinion_result | string | 意见 |

### FinishhedInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| project_key | string | 空间 key |
| work_item_id | int64 | 工作项 |
| finished_infos | List<FinishedInfoItem> | 评审信息列表 |

### InstanceDeliverableItem

| 字段 | 类型 | 说明 |
|------|------|------|
| name | string | 交付物工作项名称 |
| work_item_id | int64 | 交付物工作项ID |
| deletable | bool | 是否可删除 |
| must_complete | bool | 是否完成拦截 |
| state_key | string | 状态 |
| state_name | string | 状态名 |
| owners | list<string> | 负责人, user_key list |
| remark | string | 备注 |

### MultiSignal

| 字段 | 类型 | 说明 |
|------|------|------|
| status | string | 多值外信号状态 |
| detail | list<MultiSignalDetail> | 多值外信号详细信息 |

### MultiSignalDetail

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 多值外信号的唯一 ID。 |
| title | string | 标题 |
| status | string | 状态（"passed"、"processing"、"rejected"） |
| view_link | string | 可跳转链接 |
| query_link | ~~~~ | 查询链接（已下线） |

### MultiTextDetail

| 字段 | 类型 | 说明 |
|------|------|------|
| 富文本字段的值 |  | MultiTextDetail 字段 类型 说明 |
| doc | string | 富文本 doc 信息 |
| doc_text | string | 富文本纯文本信息 |
| is_empty | bool | 是否为空 |
| notify_user_list | list<string> | 加了 at 用户时，通知的用户列表 |
| notify_user_type | string | 加了 at 用户时，通知的用户类型 |

### NodeTask

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 节点 ID |
| state_key | string | 节点 key |
| sub_tasks | list<SubTask> | 节点下子任务列表 |

### NodesConnections

| 字段 | 类型 | 说明 |
|------|------|------|
| workflow_nodes | list<WorkflowNode> | 工作项下的工作流节点数组 |
| connections | list<Connection> | 工作项下的连接数组 |
| state_flow_nodes | list<StateFlowNode> | 工作项下的状态流节点数组 |

### ProductPrice

| 字段 | 类型 | 说明 |
|------|------|------|
| product_key | string | 商品key |
| version | int64 | 付费方案版本 |
| price_key | string | 付费方案内部 key |
| name | string | 付费方案名称 name_i18n map<string, string> 付费方案多语言名称 |
| external_platform | string | 下游平台：Lark |
| external_price_key | string | 付费方案实际key |
| payment_mode | string | 付费模式： Trial 试用｜Free 免费 ｜ Period 周期付费｜Buyout 一次性付费 |
| price | int64 | 价格 |
| currency_code | string | 货币： CNY_FEN  人民币分 |
| period_unit | string | 周期单位：Day｜Month｜Year｜ForEver |
| trial_period | int64 | 试用时间 |
| trial_product_price | string | 试用对应商品付费方案key |
| publish_status | string | 付费方案上架状态：Created（已创建）、Released（已上架）、Unreleased（已下架） |
| purchasable_tenants | list<string> | 可买租户（F 码）列表 |
| extra | map<string, string> | 额外信息 |

### ProjectRelationInstance

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 当前工作项 ID |
| work_item_name | string | 当前工作项名称 |
| instances | list<RelationInstance> | 关联关联工作项 ID |

### ProjectRelationRule

| 字段 | 类型 | 说明 |
|------|------|------|
| remote_project_key | string | 联动空间 key |
| remote_project_name | string | 联动空间名称 |
| rules | list<RelationRule> | 规则数组 |

### QueryLink

| 字段 | 类型 | 说明 |
|------|------|------|
| url | string | 链接 |
| method | string | 请求方法 |
| headers | interface{} | 请求头 |
| body | interface{} | 请求体 |
| params | interface{} | 请求参数 |

### QuickFilter

| 字段 | 类型 | 说明 |
|------|------|------|
| quick_filter_id | string | 快捷筛选id |
| quick_filter_name | string | 快捷筛选名称 |

### RelationBindInstance

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 关联工作项id |
| chat_group_merge | int | 是否合并群，1是2否 |

### RelationInstance

| 字段 | 类型 | 说明 |
|------|------|------|
| relation_work_item_id | int64 | 关联工作项id |
| relation_work_item_name | string | 关联工作项名称 |
| relation_work_item_type_name | string | 关联工作项类型名称 |
| relation_work_item_type_key | string | 关联工作项类型key |
| project_relation_rule_id | string | 空间关联规则id |
| project_relation_rule_name | string | 空间关联规则名称 |
| relation_project_key | string | 联动空间key |
| relation_project_name | string | 联动空间名称 |

### RelationRule

| 字段 | 类型 | 说明 |
|------|------|------|
| id | string | 空间关联规则id |
| name | string | 空间关联规则名称 |
| disabled | int | 是否禁用,1启用2禁用 |
| work_item_relation_id | string | 工作项关联关系id |
| work_item_relation_name | string | 工作项关联关系名称 |
| current_work_item_type_key | string | 当前工作项类型key |
| current_work_item_type_name | string | 当前工作项类型名称 |
| remote_work_item_type_key | string | 关联工作项类型key |
| remote_work_item_type_name | string | 关联工作项类型名称 |
| chat_group_merge | int | 是否合并群，1是2否 |

### RequiredDeliverable

| 字段 | 类型 | 说明 |
|------|------|------|
| deliverable | int64 | 交付物ID |
| finished | bool | 是否完成 |

### RequiredFormItem

| 字段 | 类型 | 说明 |
|------|------|------|
| class | string | 表单项类型 字段 field 角色 role 控件 control |
| key | string | 表单项的key 字段类型：field_key 角色类型：role_key 空间类型：控件ID，如节点信息控件的key是"workflow_state_info" |
| field_type_key | string | 字段类型表单项的字段类型 |
| finished | bool | 是否完成 |
| not_finished_owner | list<string> | 未完成人员 |
| sub_field | list<RequiredField> | 复合字段类型表单项的必填子字段 |

### RequiredStateInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| state_key | string | 节点node_id |
| node_fields | list<RequiredField> | 节点信息控件的必填节点字段 |
| finished | bool | 是否完成 |

### RequiredTask

| 字段 | 类型 | 说明 |
|------|------|------|
| task_id | int64 | 子任务ID |
| finished | bool | 是否完成 |

### ResourceCreateInstanceResponseData

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 创建出来的实例id |

### ResourceCreateInstanceResponseDataIgnoreCreateInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| 创建时忽略的信息 |  | ResourceCreateInstanceResponseDataIgnoreCreateInfo 字段 类型 说明 |
| field_keys | list<string> | 忽略创建的字段 key，如果一个字段因为资源字段的原因在创建时被忽略了则会返回 |
| role_ids | list<string> | 忽略创建的角色 id，如果一个角色因为资源角色的原因在创建时被忽略了则会返回 |

### ResourceDetailWorkItemInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int64 | 资源实例id |
| name | string | 资源实例名字 |
| work_item_type_key | string | 资源实例工作项key |
| project_key | string | 资源实例所属空间key |
| template_type | string | 资源实例所属模板类型 |
| created_by | string | 资源实例创建人 |
| updated_by | string | 资源实例更新人 |
| created_at | int64 | 资源实例创建时间 |
| updated_at | int64 | 资源实例更新时间 |
| fields | list<FieldValuePair> | 资源实例字段值 |
| simple_name | string | 资源实例简单名字 |
| template_id | int64 | 资源实例模板id |
| multi_texts | list<MultiText> | 富文本 |
| relation_fields_detail | list<RelationFieldDetail> | 关系字段 |
| user_details | list<UserDetail> | 用户信息 |

### RoleAssign

| 字段 | 类型 | 说明 |
|------|------|------|
| role | string | 角色ID |
| name | string | 角色名称 |
| default_appear | int64 | 展现“默认出现”“默认不出现”“条件出现”，则展示为条件出现，具体条件用户自行到空间查看 |
| Appear     = 1 //默认出现 | NoAppear   = 2 //默认不出现 | CondAppear = 3 //条件出现 |
| deletable |  | int64 |

### ScheduleConstraintRule

| 字段 | 类型 | 说明 |
|------|------|------|
| sub_task | bool | 任务 |
| node | bool | 节点 |
| sub_process_node | bool | 子流程 |
| wbs_sub_instance_type | map<string, bool> | 子项，key 为工作项 type key |

### SearchGroup

| 字段 | 类型 | 说明 |
|------|------|------|
| search_params | list<SearchParam> | 固定参数 |
| conjunction | string | 枚举 AND，OR |
| search_groups | list<SearchGroup> | 筛选组 |

### StateTime

| 字段 | 类型 | 说明 |
|------|------|------|
| state_key | string | 节点key |
| start_time | int64 | 节点开始时间 |
| end_time | int64 | 节点结束时间 |
| name | string | 节点名称 |

### SubDetail

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 对应的工作项id |
| work_item_name | string | 工作项名称 |
| node_id | string | 节点id |

### SubStatus

| 字段 | 类型 | 说明 |
|------|------|------|
| checked_time | int64 | 完成时间 |
| owner | string | 节点负责人的user_key |
| status | int | 确认的状态；0：未确认；1：已确认 |

### SymbolSetting

| 字段 | 类型 | 说明 |
|------|------|------|
| display | string | custom 自定义符号 none 不显示符号 |
| value | string | 符号值，长度限制4个字符 |
| layout | string | left 居左显示 right居右显示 |

### TaskConf

| 字段 | 类型 | 说明 |
|------|------|------|
| action | int | 1新增2删除（只需传任务ID）3修改 |
| name | string | 任务名称 |
| id | string | 任务ID |
| deliverable_field_id | string | 交付物字段ID |
| pass_mode | int | 任务完成方式，1人工确认完成2自动完成 |
| node_pass_required_mode | int | 是否作为节点完成必要条件，1是2否 |

### TeamDataScope

| 字段 | 类型 | 说明 |
|------|------|------|
| team_id | string | 团队ID |
| cascade | bool | 是否级联。级联会联动带上下级团队 true级联 false非级联 |

### TeamOption

| 字段 | 类型 | 说明 |
|------|------|------|
| team_data_scopes | list<TeamDataScope> | 当team_mode=custom时，指定团队范围 |
| team_mode |  | string 团队类型 all本空间全部可见的团队 custom指定团队范围 |

### TemplateConf

| 字段 | 类型 | 说明 |
|------|------|------|
| template_id | int64 | 流程模板id |
| template_name | string | 流程模板名称 |
| is_disabled | int64 | 是否禁用 |
| True   = 1 //禁用 | False  = 2 //启用 | version int64 模板当前版本号 |

### TemplateDetail

| 字段 | 类型 | 说明 |
|------|------|------|
| workflow_confs | list<WorkflowConf> | 节点流配置 |
| state_flow_confs | list<StateFlowConf> | 状态流配置 |
| connections | list<Connection> | 连接信息 |
| template_id | int64 | 模板id |
| template_name | string | 模板名 |
| version | int64 | 版本号 |
| is_disabled | int64 | 是否禁用，1禁用2启用 |

### TenantInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| display_id | string | 租户 F 码 |
| tenant_name | string | 租户名称 |
| tenant_key | string | 租户 key |
| lark_tenant_id | string | 飞书租户 ID |

### TransRequiredInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| form_items | list<RequiredFormItem> | 必填表单项 |
| node_fields | list<RequiredField> | 必填节点字段 |
| tasks | list<RequiredTaskRequiredTask> | 必填子任务 |
| deliverables | list<RequiredDeliverable> | 必填交付物 |

### UnionDeliverable

| 字段 | 类型 | 说明 |
|------|------|------|
| field_deliverables | list<FieldDeliverableItem> | 字段交付物 |
| instance_deliverables | list<InstanceDeliverableItem> | 实例交付物 |

### UpdateWorkingHourRecord

| 字段 | 类型 | 说明 |
|------|------|------|
| id | int64 | 工时记录id（必填） |
| work_time | string | 实际工时（选填） |
| work_description | string | 工作描述（选填） |

### WBSRelationChainEntity

| 字段 | 类型 | 说明 |
|------|------|------|
| workItem_id | int64 | 工作项id |
| wbs_relation_chain_entity | list<WBSRelationChainEntityItem> | WBS链路实例数组 |

### WBSRelationChainEntityItem

| 字段 | 类型 | 说明 |
|------|------|------|
| project_key | string | 空间key workItem_type_key |
| string | 工作项类型key | workItem_id int64 对应层级的工作项id |
| type | string | 节点类型 rootWorkItem(根工作项) subWorkItem（子流程工作项） node（节点） subTask（子任务） workItem_name |
| string | 对应层级的工作项名称（type=rootWorkItem/subWorkItem时返回） | state_key |
| string | 节点key（type=node时返回） | node_name |
| string | 节点名称（type=node时返回） | sub_task_name |
| string | 子任务名称（type=subTask时返回） | sub_task_id int64 子任务id（type=subTask时返回） |
| level | int64 | 工作项所属层级 |

### WBSRelationChainPath

| 字段 | 类型 | 说明 |
|------|------|------|
| WBS链路层级 | relation_chain_entity | WBSRelationChainEntity |
| WBS链路实例 |  | WBSRelationChainPath 字段 类型 说明 workItem_id int64 工作项id |
| wbs_relation_chain_path | list<WBSRelationChainPathItem> | WBS链路层级数组 |

### WBSRelationChainPathItem

| 字段 | 类型 | 说明 |
|------|------|------|
| project_key | string | 空间key workItem_type_key |
| string | 工作项类型key | type |

### WBSWorkItem

| 字段 | 类型 | 说明 |
|------|------|------|
| node_uuid | string | 子节点ID |
| work_item_id | int64 | 子工作项ID, 不是子工作项 - 该字段不返回 |
| type | string | 子工作项类型 node/sub_workitem/sub_task |
| wbs_status | string | 子工作项所属状态, 就是泳道图中的状态 |
| sub_work_item | list<WBSWorkItem> | 子工作项数组 当且仅当type="node","sub_workitem"该字段有值 |
| name | string | 子工作项名称 |
| deliverable | list<FieldValuePair> | 交付物 |
| wbs_status_map | map<string, string> | wbs状态映射<status_key,status_name> |

### WbsViewResponse

| 字段 | 类型 | 说明 |
|------|------|------|
| template_key | string | 模板key |
| related_sub_work_items | list<WBSWorkItem> | 子工作项数组 |

### WorkItemCreateInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 创建出的工作项资源库实例id |

### WorkflowConf

| 字段 | 类型 | 说明 |
|------|------|------|
| action | int | 1新增2删除（只需传state_key）3修改 |
| state_key | string | 节点id |
| name | string | 节点名称 |
| tags | list<string> | 对接标识 |
| pre_node_state_key | list<string> | 前序节点id |
| owner_usage_mode | int | 负责人分配方式(1自行添加/2指定角色/3全部默认分配/4条件默认分配) |
| owner_roles | list<string> | 负责人角色key |
| owners | list<string> | 负责人userkey |
| need_schedule | bool | 节点是否需填写排期与估分，true需要，false不需要 |
| different_schedule | bool | 负责人为多人时默认开启差异化排期，true需要，false不需要 |
| visibility_usage_mode | int | 节点可见性，1默认可见，2条件可见 |
| completion_tips | string | 节点完成提示 |
| deletable | bool | 是否允许删除节点 |
| deletable_operation_role | list<string> | 删除操作授权角色 |
| pass_mode | int | 节点完成方式 1自动完成/2单人确认/3多人确认 |
| is_limit_node | bool | 是否为限制节点 |
| done_operation_role | list<string> | 完成操作授权角色 |
| done_schedule | bool | 节点流转估分排期是否必填 |
| done_allocate_owner | bool | 节点流转负责人是否必填 |
| task_confs | list<TaskConf> | 完成任务 |

### conform_fields

| 字段 | 类型 | 说明 |
|------|------|------|
| value | string | 字段值 |

### node_field

| 字段 | 类型 | 说明 |
|------|------|------|
| string | 字段键 | field_type |
| string | 字段类型 |  |

### node_intercept_config

| 字段 | 类型 | 说明 |
|------|------|------|
| node_ids | list<string> | 要拦截的节点 ID 列表 |
| template_id | string | 节点模板 ID |

### project_intercept_config

| 字段 | 类型 | 说明 |
|------|------|------|
| events | list<int64> | 一组需要精细化控制的事件范围，推荐开发者按同大类做配置。 |
| work_item_type | list<string> | 工作项类型 key。 |

### prop_value

| 字段 | 类型 | 说明 |
|------|------|------|
| 属性值 |  | prop_value 字段名 类型 说明 |
| field | list<field> | 普通字段列表 |

### state_intercept_config

| 字段 | 类型 | 说明 |
|------|------|------|
| template_id | string | 节点模板 ID |
| transition_ids | list<string> | 要拦截的状态 ID 列表 |

### sub_workitems

| 字段 | 类型 | 说明 |
|------|------|------|
| relation_id | string | 关联 ID |
| workitem_ids | list<string> | 关联的工作项 ID 列表，用于标识与当前工作相关联的多个子工作项。 |
| work_item_type | string | 关联工作项类型 |

### work_item_intercept_config

| 字段 | 类型 | 说明 |
|------|------|------|
| fields | list<string> | 要拦截的字段范围，仅对 修改字段（1009）、批量修改字段（1010）事件适用，校验时需结合 events 字段。 |
| template_ids | list<string> | 要拦截的流程模板列表，仅对 模板升级（1006）、批量模板升级（1007）事件适用 |
| work_item_type | string | 工作项类型 |

### CompInfo（搜索结果）

| 字段 | 类型 | 说明 |
|------|------|------|
| ID | string | 工作项/视图 ID |
| name | string | 工作项/视图名称 |
| ViewScopeKey | string | 视图类型：story/issue/version/sprint/chart/自定义 type_key |
| WorkItemTypeKey | string | 暂为空 |
| ProjectKey | string | 所属空间 ID |
| CreatedBy | int64 | 创建人 |
| CreatedAt | int64 | 创建时间戳（毫秒） |
| SearchHit | list\<string\> | 命中结果字段 key |

### RoleConfCreate（创建角色配置）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| id | string | 否 | 角色 ID（不传则随机生成 role_xxx） |
| name | string | 是 | 角色名称 |
| is_owner | bool | 否 | 是否需求负责人（默认 false） |
| auto_enter_group | bool | 否 | 成员自动入群（默认 false） |
| member_assign_mode | int | 否 | 1=自行添加, 2=指定人员, 3=创建人（默认 1） |
| members | list\<string\> | 否 | 指定分配成员（mode=2 时必传） |
| is_member_multi | bool | 否 | 限制为单人（默认 true） |
| role_alias | string | 否 | 对接标识 |
| lock_scope | list\<string\> | 否 | 锁定范围：all/basic/role/member/auth |

### RoleConfUpdate（更新角色配置）

| 字段 | 类型 | 必填 | 说明 |
|------|------|------|------|
| name | string | 否 | 角色名称 |
| is_owner | bool | 否 | 是否需求负责人 |
| auto_enter_group | bool | 否 | 成员自动入群 |
| member_assign_mode | int | 否 | 成员分配方式 |
| members | list\<string\> | 否 | 指定分配成员 |
| is_member_multi | bool | 否 | 限制为单人 |
| role_alias | string | 否 | 对接标识 |
| lock_scope | list\<string\> | 否 | 锁定范围 |

### TenantEntitlement（租户权益）

| 字段 | 类型 | 说明 |
|------|------|------|
| product | Product | 购买商品 |
| product_price | ProductPrice | 购买付费方案 |
| status | string | Trial/Occupied/Expired/Aborted |
| start_time | int64 | 开始时间（毫秒） |
| use_deadline | int64 | 到期时间（毫秒） |
| upgradable | bool | 是否可升级 |
| renewable | bool | 是否可续费 |
| rebuyable | bool | 是否可重新购买 |
| latest_product | Product | 最新商品信息 |

### RichText（富文本）

| 字段 | 类型 | 说明 |
|------|------|------|
| doc | string | 元属性 |
| doc_html | string | 渲染样式 |
| doc_text | string | 文本字段 |
| is_empty | bool | 是否为空 |

### DeliverableInfo（交付物详情-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| name | string | 交付物工作项名称 |
| work_item_id | int64 | 交付物工作项 ID |
| associated_deliverables_template | int64 | 交付物来源工作项 |
| template_resources | bool | 是否交付物模板资源 |
| template_type | string | 交付物工作项类型 |
| deleted | bool | 是否删除 |
| delivery_related_info | DeliveryRelatedInfo | 关联挂载信息 |

### DeliveryRelatedInfo

| 字段 | 类型 | 说明 |
|------|------|------|
| root_work_item | DeliveryRelatedInfoItem | 根工作项 |
| source_work_item | DeliveryRelatedInfoItem | 直接挂载工作项 |

### FieldDeliverableItem（字段交付物-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| field_info | FieldValuePair | 字段信息 |
| placeholder | string | 占位文本 |
| remark | string | 备注 |
| status | int64 | 1=未提交, 2=已提交 |

### FinishedOpinionInfo（评审意见-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| finished_opinion_result | string | 汇总意见 |
| owners_finished_opinion_result | list\<FinishedOwnerOpinionInfo\> | 人员意见 |

### FinishedConclusionInfo（评审结论-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| finished_conclusion_result | string | 汇总结论 |
| owners_finished_conclusion_result | list\<FinishedOwnerConclusionInfo\> | 人员结论 |

### FinishedOwnerConclusionInfo（人员评审结论-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| finished_conclusion_result | string | 个人结论 |

### NumberConfig（补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| symbol_setting | SymbolSetting | 标志配置 |

### Product（商品）

| 字段 | 类型 | 说明 |
|------|------|------|
| product_key | string | 商品 key |
| product_type | string | Plugin/Solution/ProjectTemplate |
| name | string | 商品名称 |
| product_version | int64 | 商品版本 |
| short_desc | string | 简述 |
| short_desc_i18n | map\<string, string\> | 简述（多语言） |
| long_desc | map\<string, RichText\> | 详细描述（富文本） |
| long_desc_i18n | map\<string, RichText\> | 详细描述（多语言） |
| icon_url | string | 商品图标 |
| icon_url_i18n | map\<string, string\> | 商品图标（多语言） |
| cover_url | string | 详情页图 |
| cover_url_i18n | map\<string, string\> | 详情页图（多语言） |
| external_platform | string | 外部平台：Lark |
| external_product_key | string | 飞书商品 key |
| sales_tenant | list\<TenantInfo\> | 租户信息 |
| product_prices | list\<ProductPrice\> | 付费方案 |
| product_price_display | RichText | 价格展示（富文本） |
| product_price_display_i18n | map\<string, RichText\> | 价格展示（多语言） |
| approval_status | string | 审批状态 |
| publish_status | string | 上架状态 |
| app_key | string | 插件 key |

### ResourceCreateInstanceResponseData（补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| work_item_id | int64 | 创建出的实例 ID |
| ignore_create_info | Object | 创建时忽略的信息（含 field_keys, role_ids） |

### ResourceWorkitemInfo（补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| template_version | int64 | 模板版本 |

### project_intercept_config（补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| project_intercept_config_id | string | 拦截配置 ID |

### props（AI 属性-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| prop_key | string | 属性键 |
| prop_type | list\<string\> | 属性类型列表 |
| prop_value | prop_value | 属性值 |

### sub_tasks（子任务引用-补充字段）

| 字段 | 类型 | 说明 |
|------|------|------|
| task_id | int64 | 子任务 ID |
| is_finished | boolean | 是否已完成 |

---

## Dimweave 集成备注

### workspace_hint 与 project_key 的关系

- Dimweave 配置中的 `workspace_hint` 存储的是飞书项目空间的 `simple_name`（URL slug，如 `manciyuan`）
- 飞书 MCP 服务端实测接受 `simple_name` 作为以下工具参数中的 `project_key` 选择器：
  - `search_project_info`
  - `search_by_mql`
  - `get_workitem_brief`
  - `list_workitem_comments`
- `search_project_info(project_key=simple_name)` 会返回 canonical `project_key` 和 `simple_name`
- 当前实现直接将 `workspace_hint` 传递给这些工具调用，不做额外的 project_key 解析
