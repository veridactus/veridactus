// VERIDACTUS KMS 接口 — 密钥管理服务抽象
// 支持环境变量 / 阿里云KMS / HashiCorp Vault 多后端
// 严禁在任何代码路径中硬编码主密钥
package crypto

import (
	"context"
	"crypto/rand"
	"encoding/base64"
	"fmt"
	"log"
	"os"
	"sync"
)

// KMSProvider KMS 提供者接口
type KMSProvider interface {
	// Name 返回提供者名称（用于审计日志）
	Name() string
	// GetMasterKey 获取主密钥（用于信封加密的 KEK）
	// 返回 base64 编码的 256-bit AES 密钥
	GetMasterKey(ctx context.Context) (string, error)
	// IsAvailable 检查 KMS 是否可用
	IsAvailable(ctx context.Context) bool
}

// ==================== 环境变量 KMS Provider (开发/单机部署) ====================

// EnvKMSProvider 从环境变量获取主密钥
// 适用于云主机/容器部署场景，通过 K8s Secret 或云环境变量注入密钥
// 国内生产环境推荐配合阿里云 KMS 或 HashiCorp Vault 使用
type EnvKMSProvider struct {
	keyName string
	cached  string
	mu      sync.RWMutex
}

// NewEnvKMSProvider 创建环境变量 KMS Provider
func NewEnvKMSProvider(keyName string) *EnvKMSProvider {
	return &EnvKMSProvider{keyName: keyName}
}

func (p *EnvKMSProvider) Name() string { return "env" }

func (p *EnvKMSProvider) IsAvailable(ctx context.Context) bool {
	_, err := p.GetMasterKey(ctx)
	return err == nil
}

func (p *EnvKMSProvider) GetMasterKey(ctx context.Context) (string, error) {
	p.mu.RLock()
	if p.cached != "" {
		defer p.mu.RUnlock()
		return p.cached, nil
	}
	p.mu.RUnlock()

	p.mu.Lock()
	defer p.mu.Unlock()

	key := os.Getenv(p.keyName)
	if key == "" {
		// 生产/正式环境：必须配置主密钥
		env := os.Getenv("VERIDACTUS_ENV")
		if env == "production" || env == "staging" {
			return "", fmt.Errorf("KMS: %s not set in %s environment — cannot start without master key", p.keyName, env)
		}
		// 开发环境：自动生成临时密钥（每次重启密钥不同，之前加密的数据将无法解密）
		tmpKey := generateDevKey()
		log.Printf("⚠️  WARNING: VERIDACTUS_MASTER_KEY not set. Auto-generated temporary key for dev (NOT persistent).")
		log.Printf("   Set VERIDACTUS_MASTER_KEY for production. Generate with: openssl rand -base64 32")
		p.cached = tmpKey
		return tmpKey, nil
	}

	// 验证密钥长度
	decoded, err := base64.StdEncoding.DecodeString(key)
	if err != nil || len(decoded) != 32 {
		return "", fmt.Errorf("KMS: %s must be a base64-encoded 32-byte value", p.keyName)
	}

	p.cached = key
	return key, nil
}

// ==================== 全局 KMS 实例 ====================

var (
	globalKMS     KMSProvider
	globalKMSOnce sync.Once
)

// GetKMS 获取全局 KMS Provider（懒初始化）
func GetKMS() KMSProvider {
	globalKMSOnce.Do(func() {
		kmsType := os.Getenv("VERIDACTUS_KMS_TYPE")
		switch kmsType {
		case "aliyun":
			// 阿里云 KMS — 国内生产环境推荐
			// 使用阿里云 KMS SDK 解密 VERIDACTUS_MASTER_KEY_ENCRYPTED 环境变量
			// 接入方式：go get github.com/aliyun/alibabacloud-kms-go-sdk
			log.Println("KMS: aliyun KMS provider selected (placeholder — using env fallback)")
			globalKMS = NewEnvKMSProvider("VERIDACTUS_MASTER_KEY")
		case "vault":
			// HashiCorp Vault — 私有化部署推荐
			// 接入方式：go get github.com/hashicorp/vault/api
			log.Println("KMS: vault provider selected (placeholder — using env fallback)")
			globalKMS = NewEnvKMSProvider("VERIDACTUS_MASTER_KEY")
		default:
			// 默认：环境变量模式（云主机/K8s Secret 注入）
			globalKMS = NewEnvKMSProvider("VERIDACTUS_MASTER_KEY")
		}
	})
	return globalKMS
}

// SetKMS 设置自定义 KMS Provider（用于测试或自定义部署）
func SetKMS(provider KMSProvider) {
	globalKMS = provider
}

// generateDevKey 生成开发环境临时密钥（仅用于本地调试）
func generateDevKey() string {
	buf := make([]byte, 32)
	rand.Read(buf)
	return base64.StdEncoding.EncodeToString(buf)
}
