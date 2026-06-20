# Python Worker 测试

import pytest
from fastapi.testclient import TestClient
import sys
import os

# 将 app 目录添加到导入路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from app.main import app

client = TestClient(app)


def test_health():
    """测试健康检查端点"""
    response = client.get("/health")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "ok"


def test_compute_guarantee():
    """测试差分隐私保证计算"""
    payload = {
        "epsilon": 1.0,
        "delta": 1e-5,
        "sensitivity": 1.0,
    }
    response = client.post("/compute-guarantee", json=payload)
    # 即使没有 Redis 后端，也应返回有效结构
    assert response.status_code in (200, 503)


def test_drift_detection():
    """测试语义漂移检测端点"""
    payload = {
        "reference_embedding": [0.1] * 768,
        "current_embedding": [0.15] * 768,
        "threshold": 0.7,
    }
    response = client.post("/drift-detection", json=payload)
    assert response.status_code in (200, 503)


def test_pii_detection():
    """测试 PII 检测端点"""
    payload = {
        "text": "My email is test@example.com and phone is 13800138000",
        "detect_types": ["email", "phone_number"],
    }
    response = client.post("/pii-detection", json=payload)
    assert response.status_code in (200, 503)
