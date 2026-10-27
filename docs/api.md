# 简易 TCP 连接探测服务：HTTP 接口文档

## 1. 基本信息

- 默认监听地址：`127.0.0.1:8080`
- 本地基础地址：`http://127.0.0.1:8080`
- 接口版本：`v1`
- 请求与响应编码：UTF-8
- 请求与响应格式：JSON
- 时间戳：UTC 的 RFC 3339 格式，例如 `2026-10-27T08:00:00Z`

除健康检查外，接口路径统一以 `/api/v1` 开头。

## 2. 通用约定

### 2.1 请求头

提交任务时必须携带：

```http
Content-Type: application/json
```

服务生成请求 ID，并在响应头中返回：

```http
X-Request-ID: 019a0000-0000-7000-8000-000000000000
```

### 2.2 通用错误响应

所有接口级错误使用统一结构：

```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "targets must contain between 1 and 100 items"
  },
  "request_id": "019a0000-0000-7000-8000-000000000000"
}
```

字段说明：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `error.code` | string | 稳定的机器可读错误码 |
| `error.message` | string | 便于定位问题的人类可读信息 |
| `request_id` | string | 请求 ID，与 `X-Request-ID` 响应头一致 |

常见接口级错误码：

| HTTP 状态码 | 错误码 | 含义 |
| --- | --- | --- |
| 400 | `INVALID_JSON` | 请求体不是合法 JSON |
| 400 | `INVALID_REQUEST` | 字段缺失、类型错误、包含未知字段或取值不合法 |
| 404 | `TASK_NOT_FOUND` | 任务不存在或结果已过期 |
| 429 | `RATE_LIMITED` | 任务提交过于频繁 |
| 500 | `INTERNAL_ERROR` | 未预期的服务端错误 |
| 503 | `QUEUE_FULL` | 后台任务队列已满 |

## 3. 提交探测任务

```http
POST /api/v1/probe-tasks
```

创建一个后台 TCP 探测任务。接口校验并接收任务后立即返回，不等待探测完成。

### 3.1 请求体

| 字段 | 类型 | 必填 | 规则 |
| --- | --- | --- | --- |
| `timeout_ms` | integer | 否 | 单目标连接超时，默认 `3000`，范围 `100`～`10000` |
| `targets` | array | 是 | 目标数组，长度为 `1`～`100` |
| `targets[].host` | string | 是 | 主机名或 IP 地址，不能为空，最长 253 个字符 |
| `targets[].port` | integer | 是 | TCP 端口，范围 `1`～`65535` |

`host` 约定：

- 不包含 `http://` 等协议前缀，不包含路径、查询参数或端口。
- IPv4 示例：`192.0.2.10`。
- IPv6 示例：`2001:db8::10`，不使用方括号。
- 域名示例：`example.com`。
- 服务拒绝未知字段，避免字段拼写错误被静默忽略。

### 3.2 请求示例

```bash
curl -i -X POST 'http://127.0.0.1:8080/api/v1/probe-tasks' \
  -H 'Content-Type: application/json' \
  -d '{
    "timeout_ms": 3000,
    "targets": [
      {"host": "127.0.0.1", "port": 8080},
      {"host": "example.com", "port": 443}
    ]
  }'
```

### 3.3 成功响应

状态码：`202 Accepted`

响应头包含：

```http
Location: /api/v1/probe-tasks/019a1e4c-2bb7-7b31-9502-b7301e663001
```

响应体：

```json
{
  "task_id": "019a1e4c-2bb7-7b31-9502-b7301e663001",
  "status": "queued",
  "created_at": "2026-10-27T08:00:00Z"
}
```

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `task_id` | string | 任务唯一标识，查询时使用 |
| `status` | string | 创建成功时固定为 `queued` |
| `created_at` | string | 任务创建时间 |

即使调用时任务已快速开始，提交响应仍只表达“已接收”，客户端应通过查询接口获得最新状态。

### 3.4 失败响应

- 任一字段不合法：`400 INVALID_REQUEST`，整个任务不创建。
- 请求体超过 64 KiB：`400 INVALID_REQUEST`。
- 当前客户端超过提交频率：`429 RATE_LIMITED`，包含 `Retry-After` 响应头，单位为秒。
- 后台队列已满：`503 QUEUE_FULL`，客户端可稍后重试。

限流响应示例：

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 17
Content-Type: application/json
```

```json
{
  "error": {
    "code": "RATE_LIMITED",
    "message": "task submission rate limit exceeded"
  },
  "request_id": "019a0000-0000-7000-8000-000000000000"
}
```

## 4. 查询探测任务

```http
GET /api/v1/probe-tasks/{task_id}
```

返回任务进度及当前已有的目标结果。客户端可轮询该接口，直到任务状态为 `completed`。第一版不提供 Webhook 或长连接推送。

### 4.1 路径参数

| 参数 | 类型 | 说明 |
| --- | --- | --- |
| `task_id` | string | 提交任务时返回的任务 ID |

### 4.2 请求示例

```bash
curl -i 'http://127.0.0.1:8080/api/v1/probe-tasks/019a1e4c-2bb7-7b31-9502-b7301e663001'
```

### 4.3 执行中响应

状态码：`200 OK`

```json
{
  "task_id": "019a1e4c-2bb7-7b31-9502-b7301e663001",
  "status": "running",
  "created_at": "2026-10-27T08:00:00Z",
  "started_at": "2026-10-27T08:00:00.010Z",
  "finished_at": null,
  "total": 2,
  "completed": 1,
  "targets": [
    {
      "index": 0,
      "host": "127.0.0.1",
      "port": 8080,
      "status": "success",
      "started_at": "2026-10-27T08:00:00.011Z",
      "finished_at": "2026-10-27T08:00:00.012Z",
      "duration_ms": 1,
      "resolved_ip": "127.0.0.1",
      "error": null
    },
    {
      "index": 1,
      "host": "example.com",
      "port": 443,
      "status": "pending",
      "started_at": null,
      "finished_at": null,
      "duration_ms": null,
      "resolved_ip": null,
      "error": null
    }
  ]
}
```

`queued` 状态下，`started_at` 和 `finished_at` 为 `null`，`completed` 为 `0`，全部目标为 `pending`。

### 4.4 已完成响应

状态码：`200 OK`

```json
{
  "task_id": "019a1e4c-2bb7-7b31-9502-b7301e663001",
  "status": "completed",
  "created_at": "2026-10-27T08:00:00Z",
  "started_at": "2026-10-27T08:00:00.010Z",
  "finished_at": "2026-10-27T08:00:01.020Z",
  "total": 2,
  "completed": 2,
  "targets": [
    {
      "index": 0,
      "host": "127.0.0.1",
      "port": 8080,
      "status": "success",
      "started_at": "2026-10-27T08:00:00.011Z",
      "finished_at": "2026-10-27T08:00:00.012Z",
      "duration_ms": 1,
      "resolved_ip": "127.0.0.1",
      "error": null
    },
    {
      "index": 1,
      "host": "example.com",
      "port": 443,
      "status": "failed",
      "started_at": "2026-10-27T08:00:00.013Z",
      "finished_at": "2026-10-27T08:00:01.020Z",
      "duration_ms": 1007,
      "resolved_ip": null,
      "error": {
        "code": "CONNECTION_TIMEOUT",
        "message": "TCP connection timed out"
      }
    }
  ]
}
```

### 4.5 响应字段

任务字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `task_id` | string | 任务唯一标识 |
| `status` | string | `queued`、`running` 或 `completed` |
| `created_at` | string | 创建时间 |
| `started_at` | string 或 null | 首次开始执行时间 |
| `finished_at` | string 或 null | 全部目标完成时间 |
| `total` | integer | 目标总数 |
| `completed` | integer | 已完成目标数，成功和失败均计入 |
| `targets` | array | 按请求中的原始顺序返回的目标结果 |

目标字段：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `index` | integer | 目标在提交数组中的下标，从 0 开始 |
| `host` | string | 原始主机名或 IP 地址 |
| `port` | integer | 原始端口 |
| `status` | string | `pending`、`running`、`success` 或 `failed` |
| `started_at` | string 或 null | 目标开始探测时间 |
| `finished_at` | string 或 null | 目标探测结束时间 |
| `duration_ms` | integer 或 null | 目标探测耗时，未完成时为 `null` |
| `resolved_ip` | string 或 null | 连接成功时使用的 IP；未成功时为 `null` |
| `error` | object 或 null | 失败原因；非 `failed` 状态时为 `null` |

目标错误码：

| 错误码 | 含义 |
| --- | --- |
| `DNS_RESOLUTION_FAILED` | 域名无法解析或未解析出可用地址 |
| `CONNECTION_TIMEOUT` | 在允许时间内未建立 TCP 连接 |
| `CONNECTION_REFUSED` | 目标明确拒绝连接 |
| `CONNECTION_FAILED` | 其他 TCP 连接或网络错误 |

目标错误中的 `message` 用于说明原因，客户端程序应依赖 `code`，不应解析 `message`。

### 4.6 失败响应

任务不存在或已过期：

```http
HTTP/1.1 404 Not Found
Content-Type: application/json
```

```json
{
  "error": {
    "code": "TASK_NOT_FOUND",
    "message": "probe task not found"
  },
  "request_id": "019a0000-0000-7000-8000-000000000000"
}
```

## 5. 服务健康检查

```http
GET /health
```

用于确认 HTTP 服务进程正常运行，不执行任何目标探测。

请求示例：

```bash
curl -i 'http://127.0.0.1:8080/health'
```

成功响应：

```http
HTTP/1.1 200 OK
Content-Type: application/json
```

```json
{
  "status": "ok"
}
```

## 6. 默认限制汇总

| 项目 | 默认值 |
| --- | --- |
| 单任务目标数 | 1～100 |
| 单目标超时 | 3000 毫秒，可配置范围 100～10000 毫秒 |
| 请求体大小 | 最大 64 KiB |
| 全局目标并发数 | 10 |
| 提交限流 | 每个客户端 IP 每分钟 10 次 |
| 任务结果保留时间 | 完成后 1 小时 |

上述运行参数后续可在服务配置中调整；输入字段的有效范围属于接口约定，不随运行配置放宽。
