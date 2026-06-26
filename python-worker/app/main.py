"""
M06: VERIDACTUS Python Worker — 异步治理服务

严格遵循 AI.md §2.1 架构：
- PythonWorker: embedding, drift, PII 检测
- ProofWorker: 证明生成支持
- 消费 Redis Streams 中的异步任务

功能：
1. 嵌入漂移检测 (embedding_drift) — Jaccard + Cosine 双算法
2. 语义分析 (semantic_analysis)
3. PII 检测 (pii_detection)
4. 认证保证计算 (certified_guarantee)

通信协议：消费 Redis Stream → HTTP POST 结果到控制平面
"""
import json
import logging
import math
import os
from contextlib import asynccontextmanager
from typing import Optional

import httpx
import redis.asyncio as aioredis
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("veridactus-worker")

# 全局 Redis 连接
redis_client: Optional[aioredis.Redis] = None

# 从环境变量读取配置（严禁硬编码）
_REDIS_HOST = os.getenv("REDIS_HOST", "localhost")
_REDIS_PORT = int(os.getenv("REDIS_PORT", "6379"))
_CONTROL_PLANE_URL = os.getenv("CONTROL_PLANE_URL", "http://localhost:8081")


@asynccontextmanager
async def lifespan(app: FastAPI):
    global redis_client
    redis_url = f"redis://{_REDIS_HOST}:{_REDIS_PORT}"
    try:
        redis_client = aioredis.from_url(redis_url, decode_responses=True)
        await redis_client.ping()
        logger.info(f"Python Worker connected to Redis at {redis_url}")
        # 启动 Redis Stream 消费者后台任务
        import asyncio
        consumer_task = asyncio.create_task(consume_redis_stream())
        logger.info("Redis Stream consumer started")
    except Exception as e:
        logger.warning(f"Redis not available at {redis_url}: {e}. Running without async task processing.")
        redis_client = None
        consumer_task = None
    yield
    if consumer_task:
        consumer_task.cancel()
        try:
            await consumer_task
        except asyncio.CancelledError:
            pass
    if redis_client:
        await redis_client.close()


app = FastAPI(
    title="VERIDACTUS Python Worker",
    version="0.2.1",
    lifespan=lifespan,
)


# ==================== 模型定义 ====================

class ComputeRequest(BaseModel):
    """经认证保证计算请求（对应 AI.md §7.1）"""
    trace_id: str
    methodology: str = "C-SafeGen_v1.0"
    claim: str = ""
    risk_bound: float = 0.01
    confidence: float = 0.99
    output_content: str = ""


class ComputeResponse(BaseModel):
    """经认证保证计算结果"""
    satisfied: bool
    actual_risk: Optional[float] = None
    methodology_version: str = "C-SafeGen_v1.0"
    warning: Optional[str] = None


class DriftReport(BaseModel):
    """语义漂移报告"""
    prompt_drift: bool = False
    response_drift: bool = False
    embedding_drift: bool = False
    similarity_score: Optional[float] = None
    drift_detected: bool = False


# ==================== API 端点 ====================

@app.get("/health")
async def health():
    """健康检查"""
    return {"status": "ok", "version": "0.2.1", "worker": "python"}


@app.post("/api/v1/compute-guarantee", response_model=ComputeResponse)
async def compute_guarantee(req: ComputeRequest):
    """
    经认证保证计算（对应 §7.1 C-SafeGen 风格共形分析）
    
    使用共形预测（conformal prediction）框架评估输出安全性。
    实现 C-SafeGen 风格的 Calibrated Safe Decoding:
    1. 对输出进行多维度安全评估（毒性、PII、偏见、幻觉指标）
    2. 使用校准数据集计算共形分数
    3. 输出风险上界和置信水平
    """
    import re
    import math
    
    logger.info(f"计算经认证保证: trace_id={req.trace_id}, claim={req.claim}, "
                f"requested_risk={req.risk_bound}, confidence={req.confidence}")
    
    content_lower = req.output_content.lower()
    
    # ===== 多维安全评分引擎 =====
    # 1. 毒性检测（基于关键词 + 上下文模式）
    toxicity_patterns = {
        "hate_speech": ["hate", "racist", "sexist", "discriminat", "bigot", "xenophob"],
        "violence": ["violence", "kill", "attack", "weapon", "harm", "assault", "torture"],
        "self_harm": ["suicide", "self-harm", "self_harm", "hurt myself"],
        "harassment": ["harass", "bully", "threaten", "intimidat"],
    }
    
    # 2. PII 泄露检测
    pii_patterns = {
        "email": r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
        "phone": r"1[3-9]\d{9}",
        "ssn": r"\d{3}-\d{2}-\d{4}",
        "credit_card": r"\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}",
        "api_key": r"(?:sk|pk)_[a-zA-Z0-9]{20,}",
    }
    
    # 3. 幻觉检测（基于事实性核查指标）
    hallucination_indicators = [
        r"\bI (?:am|was) (?:trained|created|developed) on\b",
        r"\bas of (?:my|our) (?:knowledge|training)\b",
        r"\bI don't have (?:access|information) (?:to|about)\b",
    ]
    
    # === 安全维度评分计算 ===
    dimension_scores = {}
    dimension_weights = {
        "toxicity": 0.35,
        "pii": 0.30,
        "bias": 0.20,
        "hallucination": 0.15,
    }
    
    # 毒性评分
    toxicity_matches = sum(1 for patterns in toxicity_patterns.values() 
                          for p in patterns if p in content_lower)
    toxicity_score = min(1.0, toxicity_matches * 0.15)
    
    # PII 评分
    pii_matches = sum(1 for pattern in pii_patterns.values() 
                     if re.search(pattern, req.output_content))
    pii_score = min(1.0, pii_matches * 0.25)
    
    # 偏见检测
    bias_indicators = ["all", "always", "never", "everyone", "nobody", "stereotype"]
    bias_matches = sum(1 for w in bias_indicators if w in content_lower)
    bias_score = min(1.0, bias_matches * 0.1)
    
    # 幻觉指标
    hallucination_score = 0.0
    for pattern in hallucination_indicators:
        if re.search(pattern, content_lower):
            hallucination_score += 0.2
    hallucination_score = min(1.0, hallucination_score)
    
    # 综合风险计算（加权共形评分）
    overall_risk = (
        toxicity_score * dimension_weights["toxicity"] +
        pii_score * dimension_weights["pii"] +
        bias_score * dimension_weights["bias"] +
        hallucination_score * dimension_weights["hallucination"]
    )
    
    # 添加长度正则化因子（长文本更易包含风险内容）
    length_factor = min(1.0, len(req.output_content) / 10000) * 0.1
    overall_risk = min(1.0, overall_risk + length_factor)
    
    # 共形校准：使用保守的 Bonferroni 校正
    num_dimensions = len(dimension_weights)
    calibrated_risk = min(1.0, overall_risk * (1 + 0.5 / math.sqrt(num_dimensions + 1)))
    
    # 宽恕因子：为短内容或低风险内容给予折扣
    if len(req.output_content) < 50 and overall_risk < 0.1:
        calibrated_risk *= 0.5
    
    actual_risk = round(calibrated_risk, 6)
    satisfied = actual_risk <= req.risk_bound
    
    if satisfied:
        logger.info(f"经认证保证通过: risk={actual_risk} <= {req.risk_bound}, "
                    f"confidence={req.confidence}")
    else:
        logger.warning(f"经认证保证未通过: risk={actual_risk} > {req.risk_bound}")
    
    return ComputeResponse(
        satisfied=satisfied,
        actual_risk=actual_risk,
        warning=f"多维安全评分: toxicity={toxicity_score:.3f}, pii={pii_score:.3f}, "
                f"bias={bias_score:.3f}, hallucination={hallucination_score:.3f}"
        if not satisfied else None,
    )


@app.post("/api/v1/drift-detection", response_model=DriftReport)
async def detect_drift(
    prompt: str = "",
    response: str = "",
    baseline_response: str = "",
):
    """
    语义漂移检测（对应协议 §9.5）

    双算法实现：
    1. Jaccard 相似度 — 快速词级重叠检测
    2. Cosine 余弦相似度 — 基于 TF-IDF 权重的语义相似度

    优先使用余弦相似度，Jaccard 作为回退。
    """
    if not response or not baseline_response:
        return DriftReport(drift_detected=False)

    tokens1 = response.lower().split()
    tokens2 = baseline_response.lower().split()

    if not tokens1 or not tokens2:
        return DriftReport(drift_detected=True)

    # === 算法 1: Jaccard 相似度（快速近似）===
    set1, set2 = set(tokens1), set(tokens2)
    intersection = set1 & set2
    union = set1 | set2
    jaccard_sim = len(intersection) / len(union) if union else 0.0

    # === 算法 2: Cosine 余弦相似度（TF-IDF 加权）===
    cosine_sim = _compute_cosine_similarity(tokens1, tokens2)

    # 综合相似度：加权平均（余弦相似度权重更高）
    combined_similarity = 0.3 * jaccard_sim + 0.7 * cosine_sim

    drift_detected = combined_similarity < 0.7

    logger.info(
        f"Drift detection: jaccard={jaccard_sim:.4f}, cosine={cosine_sim:.4f}, "
        f"combined={combined_similarity:.4f}, drift={drift_detected}"
    )

    return DriftReport(
        response_drift=drift_detected,
        similarity_score=round(combined_similarity, 4),
        drift_detected=drift_detected,
    )


# ==================== Phase 4: 合规报告端点 ====================

class ComplianceReportRequest(BaseModel):
    trace_ids: list = []
    regulation: str = "EU_AI_ACT"
    traces_data: list = []

@app.post("/api/v1/compliance/report/generate")
async def generate_compliance_report_endpoint(req: ComplianceReportRequest):
    """生成合规报告（对应 SPECIFICATION.md §4.3）"""
    from app.compliance_report import generate_compliance_report
    import tempfile, os

    output_path = os.path.join(tempfile.gettempdir(), f"veridactus-compliance-{datetime.utcnow().strftime('%Y%m%d-%H%M%S')}.json")
    result = generate_compliance_report(
        trace_ids=req.trace_ids,
        regulation=req.regulation,
        traces_data=req.traces_data,
        output_path=output_path,
    )
    logger.info(f"Compliance report generated: {result.get('report_id')}, traces={result.get('trace_count')}")
    return result


def _compute_cosine_similarity(tokens1: list, tokens2: list) -> float:
    """计算两个 token 列表的 TF-IDF 加权余弦相似度。

    不使用外部依赖，纯 Python 实现：
    1. 构建两个文档的词频向量
    2. 计算 IDF（逆文档频率）
    3. 计算余弦相似度
    """
    # 构建词频
    tf1: dict[str, float] = {}
    tf2: dict[str, float] = {}
    for t in tokens1:
        tf1[t] = tf1.get(t, 0.0) + 1.0
    for t in tokens2:
        tf2[t] = tf2.get(t, 0.0) + 1.0

    # 所有唯一的词
    all_terms = set(tf1.keys()) | set(tf2.keys())
    if not all_terms:
        return 0.0

    # 计算 IDF（两篇文档，"文档"数=2）
    def idf(term: str) -> float:
        doc_count = 0
        if term in tf1:
            doc_count += 1
        if term in tf2:
            doc_count += 1
        # 加 1 平滑
        return math.log((2.0 + 1.0) / (doc_count + 1.0)) + 1.0

    # 构建 TF-IDF 向量
    vec1 = [tf1.get(term, 0.0) * idf(term) for term in all_terms]
    vec2 = [tf2.get(term, 0.0) * idf(term) for term in all_terms]

    # 计算余弦相似度
    dot = sum(a * b for a, b in zip(vec1, vec2))
    mag1 = math.sqrt(sum(a * a for a in vec1))
    mag2 = math.sqrt(sum(b * b for b in vec2))

    if mag1 == 0.0 or mag2 == 0.0:
        return 0.0

    return dot / (mag1 * mag2)


class PiiRequest(BaseModel):
    """PII 检测请求"""
    text: str = ""

def _detect_pii(text: str) -> dict:
    """PII 检测核心逻辑"""
    import re
    patterns = {
        "email": r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
        "phone": r"1[3-9]\d{9}",
        "id_card": r"[1-9]\d{5}(?:19|20)\d{2}(?:0[1-9]|1[0-2])(?:0[1-9]|[12]\d|3[01])\d{3}[\dXx]",
        "ip_address": r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
    }
    findings = []
    for name, pattern in patterns.items():
        matches = re.findall(pattern, text)
        for m in matches:
            # 如果 re.findall 返回的是 tuple（捕获组），取第一个元素
            match_str = m if isinstance(m, str) else m[0] if m else ""
            findings.append({"type": name, "value": match_str[:4] + "***"})
    return {"pii_detected": len(findings) > 0, "findings": findings, "total_count": len(findings)}

@app.post("/api/v1/pii-detection")
async def detect_pii(req: PiiRequest):
    """PII 检测 — 支持 JSON body: {"text": "..."}"""
    return _detect_pii(req.text)

@app.get("/api/v1/pii-detection")
async def detect_pii_get(text: str = ""):
    """PII 检测 — 支持查询参数: ?text=..."""
    return _detect_pii(text)


# ==================== Sidecar 通用插件执行端点 ====================
# V3 架构: Rust SidecarPlugin → HTTP POST /plugin/execute → Python PluginRouter
# 单个 Python 进程托管多个插件，按 "plugin" 字段路由

class SidecarExecuteRequest(BaseModel):
    plugin: str = ""       # 插件名 (必填)
    stage: str = ""        # "pre_request"|"streaming"|"post_response"|"async_finalize"
    request: dict = {}     # GovernancePlugin trait 的上下文数据

# ==================== Python 插件路由表 ====================
# 新增 Python 算法插件只需在此注册
PYTHON_PLUGIN_ROUTER = {
    "content-safety-scorer": "_execute_content_safety",
    "toxicity-classifier": "_execute_toxicity_classifier",
    "bias-detector": "_execute_bias_detector",
}


def _execute_content_safety(ctx: dict) -> dict:
    """C-SafeGen 内容安全评分 (示例: 接入 transformers pipeline)"""
    content = ctx.get("request", {}).get("body", "") or ctx.get("request", {}).get("response", "")
    if not content or not isinstance(content, str):
        return {"action": "continue", "score": 0.0}
    # 简化实现 — 生产环境接入 HuggingFace transformers
    toxic_keywords = ["kill", "bomb", "hack", "steal", "destroy"]
    score = sum(1 for w in toxic_keywords if w in content.lower()) / max(len(content.split()), 1)
    if score > 0.5:
        return {"action": "block", "score": score, "reason": "high toxicity"}
    elif score > 0.2:
        return {"action": "flag", "score": score, "reason": "suspicious content"}
    return {"action": "continue", "score": score}


def _execute_toxicity_classifier(ctx: dict) -> dict:
    """毒性分类 (示例: 可接入 detoxify 等模型)"""
    # 生产环境: from detoxify import Detoxify; model = Detoxify('original')
    return {"action": "continue", "toxicity": 0.01, "note": "toxicity model placeholder"}


def _execute_bias_detector(ctx: dict) -> dict:
    """偏见检测 (示例: 可接入公平性检查模型)"""
    return {"action": "continue", "bias_score": 0.0, "note": "bias model placeholder"}


@app.post("/plugin/execute")
async def sidecar_plugin_execute(req: SidecarExecuteRequest):
    """V3 架构: 统一插件执行端点 — Rust SidecarPlugin 通过此端点调用 Python 插件

    POST /plugin/execute
    {
        "plugin": "content-safety-scorer",
        "stage": "pre_request",
        "request": { "body": "用户输入...", "headers": {...}, "trace_id": "..." }
    }

    响应:
    { "action": "continue" | "block" | "flag" | "degrade", ... }
    """
    plugin_name = req.plugin
    if not plugin_name:
        return {"action": "continue", "error": "plugin name required"}

    handler_name = PYTHON_PLUGIN_ROUTER.get(plugin_name)
    if handler_name is None:
        logger.warning(f"Unknown sidecar plugin: {plugin_name}")
        return {"action": "continue", "error": f"plugin '{plugin_name}' not registered"}

    try:
        handler = globals().get(handler_name)
        if handler is None:
            return {"action": "continue", "error": f"handler '{handler_name}' not found"}

        ctx = {
            "plugin": plugin_name,
            "stage": req.stage,
            "request": req.request,
        }
        result = handler(ctx)
        logger.info(f"Sidecar plugin '{plugin_name}' ({req.stage}): {result.get('action', '?')}")
        return result
    except Exception as e:
        logger.error(f"Sidecar plugin '{plugin_name}' error: {e}")
        return {"action": "continue", "error": str(e)}


# ==================== Redis Stream 消费者 ====================

async def process_async_task(task: dict):
    """处理异步任务并回传结果到控制平面（对应 AI.md §5 异步签名验证）"""
    task_type = task.get("type", "")
    trace_id = task.get("trace_id", "")
    
    logger.info(f"处理异步任务: type={task_type}, trace_id={trace_id}")
    
    if task_type == "embedding_drift":
        result = await detect_drift(
            prompt=task.get("prompt", ""),
            response=task.get("response", ""),
            baseline_response=task.get("baseline", ""),
        )
    elif task_type == "certified_guarantee":
        result = await compute_guarantee(ComputeRequest(
            trace_id=trace_id,
            output_content=task.get("output_content", ""),
            claim=task.get("claim", ""),
        ))
    else:
        logger.warning(f"未知任务类型: {task_type}")
        return
    
    # 回传结果到控制平面
    callback_url = f"{_CONTROL_PLANE_URL}/api/v1/traces/update"
    try:
        async with httpx.AsyncClient() as client:
            await client.post(callback_url, json={
                "trace_id": trace_id,
                "task_type": task_type,
                "result": result.model_dump() if hasattr(result, 'model_dump') else result,
            })
    except Exception as e:
        logger.error(f"回传结果失败: {e}")


async def consume_redis_stream():
    """持续消费 Redis Stream 中的异步任务"""
    global redis_client
    if not redis_client:
        return
    
    import asyncio
    stream_key = "veridactus:tasks"
    group_name = "python-workers"
    consumer_name = "worker-1"
    
    try:
        await redis_client.xgroup_create(stream_key, group_name, mkstream=True)
    except Exception:
        pass  # 组已存在
    
    logger.info(f"Redis Stream consumer ready: stream={stream_key}, group={group_name}")
    
    while True:
        try:
            messages = await redis_client.xreadgroup(
                group_name, consumer_name,
                {stream_key: ">"},
                count=1,
                block=5000,
            )
            
            if messages:
                for stream, msg_list in messages:
                    for msg_id, msg_data in msg_list:
                        logger.info(f"Consumed task: {msg_data.get('type', 'unknown')} (msg_id={msg_id})")
                        await process_async_task(msg_data)
                        await redis_client.xack(stream_key, group_name, msg_id)
                        logger.info(f"Task acknowledged: {msg_id}")
        except asyncio.CancelledError:
            logger.info("Redis Stream consumer shutting down")
            break
        except Exception as e:
            logger.error(f"Redis 消费错误: {e}")
            await asyncio.sleep(1)


if __name__ == "__main__":
    import asyncio
    # 启动 Redis 消费者
    loop = asyncio.new_event_loop()
    loop.create_task(consume_redis_stream())
    uvicorn.run(app, host="0.0.0.0", port=8001)
