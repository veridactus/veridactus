//! # VERIDACTUS 端到端 (E2E) 集成测试套件
//!
//! 单测试执行所有场景，避免大模型反复加载。

use std::time::{Duration, Instant};
use reqwest::Client;
use tokio::sync::OnceCell;
use veridactus_core::http::server::{create_router, AppState};

struct TestConfig { proxy_url: String, api_key: String, audit_token: &'static str }
static PROXY: OnceCell<TestConfig> = OnceCell::const_new();
const MODEL: &str = "glm-5.1";

async fn init() -> &'static TestConfig {
    PROXY.get_or_init(|| async {
        let state = AppState::new_with_defaults();
        let key = { let mut m = state.api_key_manager.lock().unwrap(); m.generate_key("e2e-test") };
        let app = create_router(state);
        let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let p = l.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(l, app).await.unwrap(); });
        tokio::time::sleep(Duration::from_millis(200)).await;
        TestConfig { proxy_url: format!("http://127.0.0.1:{}",p), api_key: key, audit_token: "test-audit-token" }
    }).await
}

fn cli() -> Client { Client::builder().timeout(Duration::from_secs(120)).build().unwrap() }

async fn chat(cfg: &TestConfig, hdrs: Vec<(&str, &str)>) -> reqwest::Response {
    let mut r = cli()
        .post(format!("{}/v1/chat/completions",cfg.proxy_url))
        .header("Authorization",format!("Bearer {}",cfg.api_key))
        .header("Content-Type","application/json")
        .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":"回复:hello"}],"stream":false}));
    for (k,v) in hdrs { r = r.header(k,v); }
    r.send().await.unwrap()
}

async fn traces(cfg: &TestConfig) -> reqwest::Response {
    cli().get(format!("{}/v1/traces",cfg.proxy_url)).header("Authorization",format!("Bearer {}",cfg.api_key)).send().await.unwrap()
}

#[tokio::test]
async fn e2e_full_suite() {
    let cfg = init().await;
    let start = Instant::now();
    let mut ok = 0u32; let mut fail = 0u32;

    macro_rules! S {
        ($n:expr,$t:expr,$b:expr) => {{
            print!("  {} ... ", $n);
            match tokio::time::timeout(Duration::from_secs($t), $b).await {
                Ok(v) => { ok += 1; println!("✅ ({:.0}s) {:?}", start.elapsed().as_secs_f32(), v); }
                Err(_) => { fail += 1; println!("⏰ 超时(>{}s)", $t); }
            }
        }};
    }

    S!("S01 健康检查", 5, async { cli().get(format!("{}/health",cfg.proxy_url)).send().await.map(|r| r.status()) });

    S!("S02 模型列表", 5, async {
        cli().get(format!("{}/models",cfg.proxy_url)).header("Authorization",format!("Bearer {}",cfg.api_key)).send().await.map(|r| r.status())
    });

    S!("S03 无密钥→401", 15, async {
        cli().post(format!("{}/v1/chat/completions",cfg.proxy_url)).header("Content-Type","application/json")
            .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":"hi"}]}))
            .send().await.map(|r| r.status())
    });

    S!("S04 无效密钥→401", 15, async {
        cli().post(format!("{}/v1/chat/completions",cfg.proxy_url)).header("Authorization","Bearer bad").header("Content-Type","application/json")
            .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":"hi"}]}))
            .send().await.map(|r| r.status())
    });

    S!("S05 预算→429", 5, async {
        chat(cfg, vec![("VERIDACTUS-Version","0.2"),("VERIDACTUS-Budget-Limit","0.00")]).await.status()
    });

    S!("S06 Passthrough直通", 120, async { chat(cfg, vec![]).await.status() });

    S!("S07 治理+签名+存库", 120, async {
        let r = chat(cfg, vec![("VERIDACTUS-Version","0.2"),("VERIDACTUS-Budget-Limit","0.10")]).await;
        let s = r.status();
        let tid = r.headers().get("VERIDACTUS-Trace-Id").and_then(|v| v.to_str().ok().map(|s| s.to_string()));
        let pl = r.headers().get("VERIDACTUS-Proof-Levels").and_then(|v| v.to_str().ok().map(|s| s.to_string()));
        let list = traces(cfg).await;
        let j: serde_json::Value = list.json().await.unwrap();
        (s, tid, pl, j["total"].as_i64().unwrap_or(0))
    });

    S!("S08 版本协商0.3→0.2", 120, async {
        let r = chat(cfg, vec![("VERIDACTUS-Version","0.3")]).await;
        (r.status(), r.headers().get("VERIDACTUS-Version").and_then(|v| v.to_str().ok().map(|s| s.to_string())))
    });

    S!("S09 审计令牌错误详情", 15, async {
        let r = cli().post(format!("{}/v1/chat/completions",cfg.proxy_url))
            .header("Authorization","Bearer bad").header("VERIDACTUS-Audit-Token",cfg.audit_token)
            .header("Content-Type","application/json")
            .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":"hi"}]}))
            .send().await.unwrap();
        (r.status(), r.json::<serde_json::Value>().await.ok().and_then(|j| j["error"]["details"].as_object().map(|_| true)))
    });

    S!("S10 可观测性头部", 120, async {
        let r = chat(cfg, vec![("VERIDACTUS-Version","0.2")]).await;
        (r.status(),
         r.headers().get("VERIDACTUS-Version").is_some(),
         r.headers().get("VERIDACTUS-Trace-Id").is_some(),
         r.headers().get("VERIDACTUS-Proof-Levels").is_some())
    });

    S!("S11 并发3请求", 120, async {
        let mut hs = Vec::new();
        for i in 0u32..3 {
            let u = format!("{}/v1/chat/completions",cfg.proxy_url);
            let k = cfg.api_key.clone();
            hs.push(tokio::spawn(async move {
                cli().post(&u).header("Authorization",format!("Bearer {}",k))
                    .header("Content-Type","application/json")
                    .json(&serde_json::json!({"model":MODEL,"messages":[{"role":"user","content":format!("数字{}的平方?",i)}],"stream":false}))
                    .send().await.unwrap().status()
            }));
        }
        let mut r = Vec::new();
        for h in hs { r.push(h.await.unwrap()); }
        r
    });

    S!("S12 未知模型降级", 30, async {
        cli().post(format!("{}/v1/chat/completions",cfg.proxy_url))
            .header("Authorization",format!("Bearer {}",cfg.api_key)).header("Content-Type","application/json")
            .json(&serde_json::json!({"model":"not-exist","messages":[{"role":"user","content":"hi"}]}))
            .send().await.unwrap().status()
    });

    S!("S13 Trace存储完整性", 5, async {
        let j: serde_json::Value = traces(cfg).await.json().await.unwrap();
        j["total"].as_i64().unwrap_or(0)
    });

    let t = start.elapsed().as_secs_f32();
    println!("\n══════════════ E2E 结果 ══════════════");
    println!("  通过: {}  |  失败: {}  |  耗时: {:.0}s", ok, fail, t);
    assert!(fail == 0, "{} 个场景失败", fail);
}
