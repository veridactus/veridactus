// VERIDACTUS 控制平面 — 结构化日志（JSON 格式）
// 支持 INFO/WARN/ERROR 级别，输出为 JSON 行格式

package main

import (
	"encoding/json"
	"fmt"
	"log"
	"os"
	"time"
)

// LogLevel 日志级别
type LogLevel int

const (
	LevelDebug LogLevel = iota
	LevelInfo
	LevelWarn
	LevelError
)

var currentLevel = LevelInfo

// init 根据环境变量设置日志级别
func init() {
	level := os.Getenv("LOG_LEVEL")
	switch level {
	case "debug":
		currentLevel = LevelDebug
	case "warn":
		currentLevel = LevelWarn
	case "error":
		currentLevel = LevelError
	default:
		currentLevel = LevelInfo
	}
}

// structuredLog 输出 JSON 格式的结构化日志
func structuredLog(level LogLevel, msg string, fields map[string]interface{}) {
	if level < currentLevel {
		return
	}

	levelStr := "info"
	switch level {
	case LevelDebug:
		levelStr = "debug"
	case LevelWarn:
		levelStr = "warn"
	case LevelError:
		levelStr = "error"
	}

	entry := map[string]interface{}{
		"timestamp": time.Now().UTC().Format(time.RFC3339),
		"level":     levelStr,
		"message":   msg,
		"component": "control-plane",
	}
	for k, v := range fields {
		entry[k] = v
	}

	jsonBytes, err := json.Marshal(entry)
	if err != nil {
		log.Printf("log serialization failed: %v", err)
		return
	}
	fmt.Fprintln(os.Stdout, string(jsonBytes))
}

// logFatal 输出 FATAL 级别日志并退出
func logFatal(msg string, fields ...interface{}) {
	m := make(map[string]interface{})
	for i := 0; i+1 < len(fields); i += 2 {
		if k, ok := fields[i].(string); ok {
			m[k] = fields[i+1]
		}
	}
	structuredLog(LevelError, msg, m)
	os.Exit(1)
}

// 便捷方法
func logInfo(msg string, fields ...interface{}) {
	m := make(map[string]interface{})
	for i := 0; i+1 < len(fields); i += 2 {
		if k, ok := fields[i].(string); ok {
			m[k] = fields[i+1]
		}
	}
	structuredLog(LevelInfo, msg, m)
}

func logWarn(msg string, fields ...interface{}) {
	m := make(map[string]interface{})
	for i := 0; i+1 < len(fields); i += 2 {
		if k, ok := fields[i].(string); ok {
			m[k] = fields[i+1]
		}
	}
	structuredLog(LevelWarn, msg, m)
}

func logError(msg string, fields ...interface{}) {
	m := make(map[string]interface{})
	for i := 0; i+1 < len(fields); i += 2 {
		if k, ok := fields[i].(string); ok {
			m[k] = fields[i+1]
		}
	}
	structuredLog(LevelError, msg, m)
}
