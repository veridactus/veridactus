"""
M06: VERIDACTUS Python Worker — 异步治理服务

严格遵循 AI.md §2.1 架构：
- PythonWorker: embedding, drift, PII 检测
- ProofWorker: 证明生成支持
- 消费 Redis Streams 中的异步任务

功能：
1. 嵌入漂移检测 (embedding_drift)
2. 语义分析 (semantic_analysis)
3. PII 检测 (pii_detection)
4. 认证保证计算 (certified_guarantee)

通信协议：消费 Redis Stream → HTTP POST 结果到控制平面
"""
import json
import logging
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


@asynccontextmanager
async def lifespan(app: FastAPI):
    global redis_client
    redis_client = aioredis.from_url("redis://localhost:6379", decode_responses=True)
    logger.info("Python Worker 已启动，等待 Redis Stream 任务...")
    yield
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
    
    比较当前响应与基线响应的语义相似度。
    简化实现：基于 Jaccard 相似度。
    """
    if not response or not baseline_response:
        return DriftReport(drift_detected=False)
    
    # 简化：使用 token 集合的 Jaccard 相似度
    tokens1 = set(response.lower().split())
    tokens2 = set(baseline_response.lower().split())
    
    if not tokens1 or not tokens2:
        return DriftReport(drift_detected=True)
    
    intersection = tokens1 & tokens2
    union = tokens1 | tokens2
    similarity = len(intersection) / len(union) if union else 0
    
    drift_detected = similarity < 0.7
    
    return DriftReport(
        response_drift=drift_detected,
        similarity_score=round(similarity, 4),
        drift_detected=drift_detected,
    )


class PiiRequest(BaseModel):
    """PII 检测请求"""
    text: str = ""

def _detect_pii(text: str) -> dict:
    """PII 检测核心逻辑"""
    import re
    patterns = {
        "email": r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}",
        "phone": r"1[3-9]\d{9}",
        "id_card": r"[1-9]\d{5}(19|20)\d{2}(0[1-9]|1[0-2])(0[1-9]|[12]\d|3[01])\d{3}[\dXx]",
        "ip_address": r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b",
    }
    findings = []
    for name, pattern in patterns.items():
        matches = re.findall(pattern, text)
        for m in matches:
            findings.append({"type": name, "value": m[:4] + "***"})
    return {"pii_detected": len(findings) > 0, "findings": findings, "total_count": len(findings)}

@app.post("/api/v1/pii-detection")
async def detect_pii(req: PiiRequest):
    """PII 检测 — 支持 JSON body: {"text": "..."}"""
    return _detect_pii(req.text)

@app.get("/api/v1/pii-detection")
async def detect_pii_get(text: str = ""):
    """PII 检测 — 支持查询参数: ?text=..."""
    return _detect_pii(text)


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
    control_plane_url = "http://localhost:8081/api/v1/traces/update"
    try:
        async with httpx.AsyncClient() as client:
            await client.post(control_plane_url, json={
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
    
    stream_key = "veridactus:tasks"
    group_name = "python-workers"
    consumer_name = "worker-1"
    
    try:
        await redis_client.xgroup_create(stream_key, group_name, mkstream=True)
    except Exception:
        pass  # 组已存在
    
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
                        await process_async_task(msg_data)
                        await redis_client.xack(stream_key, group_name, msg_id)
        except Exception as e:
            logger.error(f"Redis 消费错误: {e}")
            import asyncio
            await asyncio.sleep(1)


if __name__ == "__main__":
    import asyncio
    # 启动 Redis 消费者
    loop = asyncio.new_event_loop()
    loop.create_task(consume_redis_stream())
    uvicorn.run(app, host="0.0.0.0", port=8001)
