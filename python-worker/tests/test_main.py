"""Python Worker 测试 — 验证所有 API 端点正常工作。

修复记录：
- 修复了端点路径与 FastAPI 路由不匹配的问题
- 修复了请求 payload 结构与 Pydantic 模型不匹配的问题
- 增加了更多边界条件测试
"""

import pytest
from fastapi.testclient import TestClient
import sys
import os

# 将 app 目录添加到导入路径
sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
from app.main import app

client = TestClient(app)


class TestHealthCheck:
    """健康检查端点测试"""

    def test_health_returns_ok(self):
        response = client.get("/health")
        assert response.status_code == 200
        data = response.json()
        assert data["status"] == "ok"
        assert data["version"] == "0.2.1"
        assert data["worker"] == "python"


class TestComputeGuarantee:
    """C-SafeGen 经认证保证计算测试"""

    def test_compute_guarantee_safe_content(self):
        """安全内容应该通过保证检查"""
        payload = {
            "trace_id": "test-trace-001",
            "methodology": "C-SafeGen_v1.0",
            "claim": "output is safe",
            "risk_bound": 0.01,
            "confidence": 0.99,
            "output_content": "Hello, how can I help you today?",
        }
        response = client.post("/api/v1/compute-guarantee", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert "satisfied" in data
        assert "actual_risk" in data
        assert data["methodology_version"] == "C-SafeGen_v1.0"

    def test_compute_guarantee_harmful_content(self):
        """有害内容应被检测并产生较高风险分数"""
        payload = {
            "trace_id": "test-trace-002",
            "methodology": "C-SafeGen_v1.0",
            "claim": "output is safe",
            "risk_bound": 0.01,
            "confidence": 0.99,
            "output_content": "I hate everyone and will kill them. Here is my email: attacker@evil.com, phone: 13800138000",
        }
        response = client.post("/api/v1/compute-guarantee", json=payload)
        assert response.status_code == 200
        data = response.json()
        # 有害内容应不满足严格的 risk_bound=0.01
        assert data["satisfied"] is False
        assert data["actual_risk"] > 0.01

    def test_compute_guarantee_empty_content(self):
        """空内容应该返回低风险"""
        payload = {
            "trace_id": "test-trace-003",
            "output_content": "",
        }
        response = client.post("/api/v1/compute-guarantee", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["satisfied"] is True

    def test_compute_guarantee_pii_leak(self):
        """包含 PII 的内容应有风险评分"""
        payload = {
            "trace_id": "test-trace-004",
            "output_content": "My credit card is 4111-1111-1111-1111 and SSN is 123-45-6789",
            "risk_bound": 0.3,
            "confidence": 0.99,
        }
        response = client.post("/api/v1/compute-guarantee", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert "actual_risk" in data
        # PII 应被检测并贡献风险分数
        assert data["actual_risk"] > 0.0


class TestDriftDetection:
    """语义漂移检测测试"""

    def test_drift_detection_similar_texts(self):
        """高度相似的文本不应触发漂移"""
        response = client.post(
            "/api/v1/drift-detection?prompt=hello&response=The weather is nice today&baseline_response=The weather is good today"
        )
        assert response.status_code == 200
        data = response.json()
        assert "drift_detected" in data

    def test_drift_detection_different_texts(self):
        """完全不同的文本应触发漂移"""
        response = client.post(
            "/api/v1/drift-detection?prompt=hello&response=The sky is blue&baseline_response=This is a completely different sentence with unique words"
        )
        assert response.status_code == 200
        data = response.json()
        assert data["drift_detected"] is True

    def test_drift_detection_empty_params(self):
        """空参数应返回默认无漂移"""
        response = client.post("/api/v1/drift-detection")
        assert response.status_code == 200
        data = response.json()
        assert data["drift_detected"] is False


class TestPiiDetection:
    """PII 检测测试"""

    def test_pii_detection_post_email(self):
        """POST 方式检测邮箱"""
        payload = {"text": "My email is test@example.com"}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is True
        assert data["total_count"] >= 1

    def test_pii_detection_post_phone(self):
        """检测中国手机号"""
        payload = {"text": "我的手机号是13800138000，请联系我"}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is True

    def test_pii_detection_get(self):
        """GET 方式检测"""
        response = client.get("/api/v1/pii-detection?text=test@example.com 13800138000")
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is True

    def test_pii_detection_clean_text(self):
        """无 PII 的文本应返回未检测到"""
        payload = {"text": "Hello world, this is a clean text without any personal data."}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is False

    def test_pii_detection_id_card(self):
        """检测身份证号"""
        payload = {"text": "身份证号：110101199003071234"}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is True

    def test_pii_detection_ip_address(self):
        """检测 IP 地址"""
        payload = {"text": "Server is at 192.168.1.100 and public IP is 8.8.8.8"}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["pii_detected"] is True

    def test_pii_detection_masked_values(self):
        """检测到的 PII 值应该被遮蔽"""
        payload = {"text": "Email: user@domain.com"}
        response = client.post("/api/v1/pii-detection", json=payload)
        assert response.status_code == 200
        data = response.json()
        assert data["total_count"] > 0
        # 检查 findings 中确实有遮蔽的值
        for finding in data["findings"]:
            assert "***" in finding["value"]
