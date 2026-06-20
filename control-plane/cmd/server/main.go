// VERIDACTUS 控制平面 — 程序入口
// M08: REST API Gateway (AI.md §2.1)
package main

import (
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

// getEnv 读取环境变量，若未设置则返回默认值。
func getEnv(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

// main 初始化 SQLite 存储、注册 REST 路由并启动 HTTP 服务。
// 启动后异步Waiting for data plane并推送当前配置。
func main() {
	dbPath := getEnv("DB_PATH", "./veridactus.db")
	store, err := NewStore(dbPath)
	if err != nil {
		logError("Storage init failed: %v", err); os.Exit(1)
	}
	logInfo("SQLite database connected: %s", dbPath)

	if err := store.InitStorageTables(); err != nil {
		logError("Storage config table init failed: %v", err); os.Exit(1)
	}
	logInfo("Storage config tables initialized")

	mux := http.NewServeMux()

	mux.HandleFunc("/api/v1/health", handleHealth(store))

	mux.HandleFunc("/api/v1/traces", handleTraces(store))
	mux.HandleFunc("/api/v1/traces/", handleTraces(store))
	mux.HandleFunc("/api/v1/pipelines", handlePipelines(store))
	mux.HandleFunc("/api/v1/pipelines/", handlePipelines(store))
	mux.HandleFunc("/api/v1/plugins", handlePlugins(store))
	mux.HandleFunc("/api/v1/plugins/", handlePlugins(store))
	mux.HandleFunc("/api/v1/policies", handlePolicies(store))
	mux.HandleFunc("/api/v1/policies/", handlePolicies(store))
	mux.HandleFunc("/api/v1/apikeys", handleApiKeys(store))
	mux.HandleFunc("/api/v1/apikeys/", handleApiKeyByID(store))
	mux.HandleFunc("/api/v1/models", handleModels(store))
	mux.HandleFunc("/api/v1/models/", handleModelByID(store))
	mux.HandleFunc("/api/v1/dataplane-configs", handleDataPlaneConfigs(store))
	mux.HandleFunc("/api/v1/dataplane-configs/", handleDataPlaneConfigByID(store))
	mux.HandleFunc("/api/v1/config/poll", handleConfigPoll(store))

	handler := adminAuthMiddleware(corsMiddleware(mux))

	port := getEnv("PORT", "8081")
	srv := &http.Server{Addr: fmt.Sprintf(":%s", port), Handler: handler}

	// 等待数据平面启动后，推送当前配置（带重试）
	go func() {
		time.Sleep(3 * time.Second)
		for i := 0; i < 30; i++ {
			c := &http.Client{Timeout: 5 * time.Second}
			resp, err := c.Get(dataPlaneURL + "/health")
			if err == nil && resp.StatusCode == 200 {
				resp.Body.Close()
				logInfo("Data plane ready, pushing config...")
				pushModelsToDataPlane(store)
				time.Sleep(1 * time.Second)
				pushPipelinesToDataPlane(store)
				return
			}
			if resp != nil {
				resp.Body.Close()
			}
			logInfo("Waiting for data plane (%d/30)...", i+1)
			time.Sleep(2 * time.Second)
		}
		logWarn("Data plane not ready, skip startup config push")
	}()

	go func() {
		logInfo("VERIDACTUS Control Plane started on :%s (SQLite persistence)", port)
		if err := srv.ListenAndServe(); err != http.ErrServerClosed {
			logError("server fatal error: %v", err); os.Exit(1)
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit
	logInfo("Control Plane shutting down...")
}
