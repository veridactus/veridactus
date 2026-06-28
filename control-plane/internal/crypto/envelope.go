// VERIDACTUS 控制平面 — 信封加密模块
// AES-256-GCM 加密 + 可选 KMS 主密钥
package crypto

import (
	"context"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"log"
)

// EnvelopeEncrypted 信封加密后的数据
type EnvelopeEncrypted struct {
	Ciphertext string `json:"ciphertext"` // Base64: AES-256-GCM(iv+ciphertext+tag)
	DataKey    string `json:"data_key"`   // Base64: 加密后的数据密钥
	Version    int    `json:"version"`    // 加密方案版本
}

// EncryptProviderKey 加密 LLM Provider Key（信封加密）
// 1. 生成随机 256-bit 数据密钥 (DEK)
// 2. 使用 DEK 对明文做 AES-256-GCM 加密
// 3. 使用主密钥加密 DEK
// 返回: EnvelopeEncrypted{密文, 加密的DEK, 版本}
func EncryptProviderKey(plaintext, masterKey string) (*EnvelopeEncrypted, error) {
	if plaintext == "" {
		return nil, errors.New("plaintext is empty")
	}
	if masterKey == "" {
		masterKey = generateDefaultMasterKey()
	}

	// 1. 生成随机 DEK
	dek := make([]byte, 32) // 256-bit
	if _, err := io.ReadFull(rand.Reader, dek); err != nil {
		return nil, fmt.Errorf("generate DEK: %w", err)
	}

	// 2. AES-256-GCM 加密明文
	block, err := aes.NewCipher(dek)
	if err != nil {
		return nil, fmt.Errorf("create cipher: %w", err)
	}
	aesGCM, err := cipher.NewGCM(block)
	if err != nil {
		return nil, fmt.Errorf("create GCM: %w", err)
	}
	nonce := make([]byte, aesGCM.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, fmt.Errorf("generate nonce: %w", err)
	}
	ciphertext := aesGCM.Seal(nonce, nonce, []byte(plaintext), nil)

	// 3. 使用主密钥加密 DEK
	encryptedDEK, err := encryptDEKWithMasterKey(dek, masterKey)
	if err != nil {
		return nil, fmt.Errorf("encrypt DEK: %w", err)
	}

	return &EnvelopeEncrypted{
		Ciphertext: base64.StdEncoding.EncodeToString(ciphertext),
		DataKey:    base64.StdEncoding.EncodeToString(encryptedDEK),
		Version:    1,
	}, nil
}

// DecryptProviderKey 解密 LLM Provider Key
func DecryptProviderKey(encrypted *EnvelopeEncrypted, masterKey string) (string, error) {
	if encrypted == nil {
		return "", errors.New("encrypted is nil")
	}
	if masterKey == "" {
		masterKey = generateDefaultMasterKey()
	}

	// 1. 解密 DEK
	ciphertext, err := base64.StdEncoding.DecodeString(encrypted.Ciphertext)
	if err != nil {
		return "", fmt.Errorf("decode ciphertext: %w", err)
	}
	encryptedDEK, err := base64.StdEncoding.DecodeString(encrypted.DataKey)
	if err != nil {
		return "", fmt.Errorf("decode data key: %w", err)
	}

	dek, err := decryptDEKWithMasterKey(encryptedDEK, masterKey)
	if err != nil {
		return "", fmt.Errorf("decrypt DEK: %w", err)
	}

	// 2. AES-256-GCM 解密
	block, err := aes.NewCipher(dek)
	if err != nil {
		return "", fmt.Errorf("create cipher: %w", err)
	}
	aesGCM, err := cipher.NewGCM(block)
	if err != nil {
		return "", fmt.Errorf("create GCM: %w", err)
	}
	nonceSize := aesGCM.NonceSize()
	if len(ciphertext) < nonceSize {
		return "", errors.New("ciphertext too short")
	}
	nonce, ct := ciphertext[:nonceSize], ciphertext[nonceSize:]
	plaintext, err := aesGCM.Open(nil, nonce, ct, nil)
	if err != nil {
		return "", fmt.Errorf("decrypt: %w", err)
	}

	return string(plaintext), nil
}

// encryptDEKWithMasterKey 使用主密钥加密数据密钥
func encryptDEKWithMasterKey(dek []byte, masterKey string) ([]byte, error) {
	key := deriveMasterKey(masterKey)
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	aesGCM, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonce := make([]byte, aesGCM.NonceSize())
	if _, err := io.ReadFull(rand.Reader, nonce); err != nil {
		return nil, err
	}
	return aesGCM.Seal(nonce, nonce, dek, nil), nil
}

// decryptDEKWithMasterKey 使用主密钥解密数据密钥
func decryptDEKWithMasterKey(encryptedDEK []byte, masterKey string) ([]byte, error) {
	key := deriveMasterKey(masterKey)
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, err
	}
	aesGCM, err := cipher.NewGCM(block)
	if err != nil {
		return nil, err
	}
	nonceSize := aesGCM.NonceSize()
	if len(encryptedDEK) < nonceSize {
		return nil, errors.New("encrypted DEK too short")
	}
	nonce, ct := encryptedDEK[:nonceSize], encryptedDEK[nonceSize:]
	return aesGCM.Open(nil, nonce, ct, nil)
}

// deriveMasterKey 从字符串主密钥派生 256-bit AES 密钥
func deriveMasterKey(masterKey string) []byte {
	h := sha256.Sum256([]byte(masterKey))
	return h[:]
}

// HashKey 对密钥做 SHA-256 哈希（用于存储和比对）
func HashKey(key string) string {
	h := sha256.Sum256([]byte(key))
	return hex.EncodeToString(h[:])
}

// GenerateRandomKey 生成随机密钥
func GenerateRandomKey(prefix string, length int) (string, error) {
	b := make([]byte, length)
	if _, err := io.ReadFull(rand.Reader, b); err != nil {
		return "", err
	}
	return prefix + hex.EncodeToString(b)[:length], nil
}

// GenerateAPIKey 生成 VERIDACTUS API Key (vd-xxx 格式)
func GenerateAPIKey() (string, error) {
	return GenerateRandomKey("vd-", 32)
}

// GenerateRefreshToken 生成刷新令牌
func GenerateRefreshToken() (string, error) {
	b := make([]byte, 48)
	if _, err := io.ReadFull(rand.Reader, b); err != nil {
		return "", err
	}
	return "rt_" + hex.EncodeToString(b)[:64], nil
}

// ErrMasterKeyUnavailable 主密钥不可用错误
var ErrMasterKeyUnavailable = errors.New("master key unavailable: set VERIDACTUS_MASTER_KEY environment variable")

// cachedMasterKey 启动时缓存的主密钥（避免重复调用 KMS）
var cachedMasterKey string

// InitMasterKey 在服务启动时初始化主密钥（由 main.go 调用）
// 如果主密钥不可用且非开发环境，返回错误阻止启动
func InitMasterKey() error {
	kms := GetKMS()
	ctx := context.TODO()
	key, err := kms.GetMasterKey(ctx)
	if err != nil {
		return fmt.Errorf("init master key: %w", err)
	}
	if key == "" {
		return ErrMasterKeyUnavailable
	}
	cachedMasterKey = key
	log.Printf("INFO: Master key initialized via KMS provider: %s", kms.Name())
	return nil
}

// getMasterKey 获取启动时初始化的主密钥
func getMasterKey() string {
	if cachedMasterKey != "" {
		return cachedMasterKey
	}
	// 回退：尝试初始化（仅在测试等未显式调用 InitMasterKey 的场景）
	_ = InitMasterKey()
	if cachedMasterKey == "" {
		panic("VERIDACTUS: master key not initialized. Call crypto.InitMasterKey() at startup.")
	}
	return cachedMasterKey
}

func generateDefaultMasterKey() string {
	return getMasterKey()
}
