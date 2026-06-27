// VERIDACTUS — 短信验证码发送服务 (纯生产实现)
//
// 支持 SMS 服务商:
//   阿里云短信: ALIYUN_SMS_ACCESS_KEY_ID / ALIYUN_SMS_ACCESS_KEY_SECRET
//              ALIYUN_SMS_SIGN_NAME / ALIYUN_SMS_TEMPLATE_CODE
//   Twilio:     TWILIO_ACCOUNT_SID / TWILIO_AUTH_TOKEN / TWILIO_PHONE_NUMBER
//
// 未配置任何 SMS 服务商时，SendPhoneCode 返回明确错误。
// 不提供任何"开发模式"回退 —— 生产代码不应有安全降级。
package auth

import (
	"context"
	"crypto/hmac"
	"crypto/sha1"
	"crypto/sha256"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/veridactus/control-plane/internal/store"
)

// ==================== SMS Provider 接口 ====================

type SmsProvider interface {
	SendVerificationCode(phone, code string) error
	Name() string
}

// NewSmsProvider 检测并返回配置的短信服务商
// 返回值 nil 表示未配置任何 SMS 服务，此时应提示用户配置
func NewSmsProvider() SmsProvider {
	if os.Getenv("ALIYUN_SMS_ACCESS_KEY_ID") != "" &&
		os.Getenv("ALIYUN_SMS_ACCESS_KEY_SECRET") != "" &&
		os.Getenv("ALIYUN_SMS_TEMPLATE_CODE") != "" {
		return NewAliyunSmsProvider()
	}
	if os.Getenv("TWILIO_ACCOUNT_SID") != "" &&
		os.Getenv("TWILIO_AUTH_TOKEN") != "" &&
		os.Getenv("TWILIO_PHONE_NUMBER") != "" {
		return NewTwilioSmsProvider()
	}
	return nil
}

// ==================== 阿里云短信 ====================
// 文档: https://help.aliyun.com/document_detail/101414.html
// 所需环境变量:
//   ALIYUN_SMS_ACCESS_KEY_ID      — 阿里云 AccessKey ID
//   ALIYUN_SMS_ACCESS_KEY_SECRET  — 阿里云 AccessKey Secret
//   ALIYUN_SMS_SIGN_NAME          — 短信签名 (默认 "VERIDACTUS")
//   ALIYUN_SMS_TEMPLATE_CODE      — 短信模板代码 (如 SMS_123456789)
// 模板变量: ${code} — 验证码

type AliyunSmsProvider struct {
	accessKeyID     string
	accessKeySecret string
	signName        string
	templateCode    string
	client          *http.Client
}

func NewAliyunSmsProvider() *AliyunSmsProvider {
	return &AliyunSmsProvider{
		accessKeyID:     os.Getenv("ALIYUN_SMS_ACCESS_KEY_ID"),
		accessKeySecret: os.Getenv("ALIYUN_SMS_ACCESS_KEY_SECRET"),
		signName:        getEnvDefault("ALIYUN_SMS_SIGN_NAME", "VERIDACTUS"),
		templateCode:    os.Getenv("ALIYUN_SMS_TEMPLATE_CODE"),
		client:          &http.Client{Timeout: 10 * time.Second},
	}
}

func (a *AliyunSmsProvider) Name() string { return "aliyun" }

func (a *AliyunSmsProvider) SendVerificationCode(phone, code string) error {
	params := map[string]string{
		"AccessKeyId":      a.accessKeyID,
		"Action":           "SendSms",
		"Format":           "JSON",
		"PhoneNumbers":     phone,
		"SignName":         a.signName,
		"TemplateCode":     a.templateCode,
		"TemplateParam":    fmt.Sprintf(`{"code":"%s"}`, code),
		"SignatureMethod":  "HMAC-SHA1",
		"SignatureVersion": "1.0",
		"SignatureNonce":   generateNonce(),
		"Timestamp":        time.Now().UTC().Format("2006-01-02T15:04:05Z"),
		"Version":          "2017-05-25",
		"RegionId":         "cn-hangzhou",
	}

	signature := aliyunHmacSign(params, a.accessKeySecret)

	queryParams := url.Values{}
	for k, v := range params {
		queryParams.Set(k, v)
	}
	queryParams.Set("Signature", signature)

	reqURL := "https://dysmsapi.aliyuncs.com/?" + queryParams.Encode()
	resp, err := a.client.Get(reqURL)
	if err != nil {
		return fmt.Errorf("阿里云短信请求失败: %w", err)
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	var result struct {
		Code    string `json:"Code"`
		Message string `json:"Message"`
	}
	json.Unmarshal(body, &result)

	if result.Code != "OK" {
		return fmt.Errorf("阿里云短信发送失败: %s (%s)", result.Message, result.Code)
	}
	log.Printf("[SMS] 阿里云验证码已发送: phone=%s", maskPhone(phone))
	return nil
}

func aliyunHmacSign(params map[string]string, secret string) string {
	var keys []string
	for k := range params { keys = append(keys, k) }
	for i := 0; i < len(keys); i++ {
		for j := i + 1; j < len(keys); j++ {
			if keys[i] > keys[j] { keys[i], keys[j] = keys[j], keys[i] }
		}
	}
	var parts []string
	for _, k := range keys {
		parts = append(parts, fmt.Sprintf("%s=%s", url.QueryEscape(k), url.QueryEscape(params[k])))
	}
	canonical := strings.Join(parts, "&")
	stringToSign := fmt.Sprintf("GET&%s&%s", url.QueryEscape("/"), url.QueryEscape(canonical))
	mac := hmac.New(sha1.New, []byte(secret+"&"))
	mac.Write([]byte(stringToSign))
	return base64.StdEncoding.EncodeToString(mac.Sum(nil))
}

// ==================== Twilio ====================
// 所需环境变量:
//   TWILIO_ACCOUNT_SID       — Twilio Account SID
//   TWILIO_AUTH_TOKEN        — Twilio Auth Token
//   TWILIO_PHONE_NUMBER      — Twilio 购买的电话号码 (如 +1234567890)

type TwilioSmsProvider struct {
	accountSID  string
	authToken   string
	phoneNumber string
	client      *http.Client
}

func NewTwilioSmsProvider() *TwilioSmsProvider {
	return &TwilioSmsProvider{
		accountSID:  os.Getenv("TWILIO_ACCOUNT_SID"),
		authToken:   os.Getenv("TWILIO_AUTH_TOKEN"),
		phoneNumber: os.Getenv("TWILIO_PHONE_NUMBER"),
		client:      &http.Client{Timeout: 10 * time.Second},
	}
}

func (t *TwilioSmsProvider) Name() string { return "twilio" }

func (t *TwilioSmsProvider) SendVerificationCode(phone, code string) error {
	apiURL := fmt.Sprintf("https://api.twilio.com/2010-04-01/Accounts/%s/Messages.json", t.accountSID)
	data := url.Values{}
	data.Set("To", phone)
	data.Set("From", t.phoneNumber)
	data.Set("Body", fmt.Sprintf("【VERIDACTUS】您的验证码是: %s，5分钟内有效。", code))

	req, _ := http.NewRequest("POST", apiURL, strings.NewReader(data.Encode()))
	req.SetBasicAuth(t.accountSID, t.authToken)
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")

	resp, err := t.client.Do(req)
	if err != nil { return fmt.Errorf("Twilio 请求失败: %w", err) }
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		body, _ := io.ReadAll(resp.Body)
		return fmt.Errorf("Twilio 发送失败 (HTTP %d): %s", resp.StatusCode, string(body))
	}
	log.Printf("[SMS] Twilio 验证码已发送: phone=%s", maskPhone(phone))
	return nil
}

// ==================== 核心发送函数 ====================

// SendPhoneCode 发送手机验证码 (纯生产，不模拟)
// 返回错误如果未配置 SMS 服务商
func SendPhoneCode(ctx context.Context, s store.StoreFacade, phone string) (string, string, error) {
	provider := NewSmsProvider()
	if provider == nil {
		return "", "", fmt.Errorf(
			"短信服务未配置。请设置环境变量:\n" +
				"  阿里云: ALIYUN_SMS_ACCESS_KEY_ID, ALIYUN_SMS_ACCESS_KEY_SECRET, ALIYUN_SMS_TEMPLATE_CODE, ALIYUN_SMS_SIGN_NAME\n" +
				"  Twilio: TWILIO_ACCOUNT_SID, TWILIO_AUTH_TOKEN, TWILIO_PHONE_NUMBER")
	}

	code := generateVerificationCode()
	expires := time.Now().Add(5 * time.Minute).UTC().Format(time.RFC3339)
	wsID := "phone-verification"
	s.UpdateSettings(ctx, wsID, map[string]string{
		"phone:" + phone + ":code":    code,
		"phone:" + phone + ":expires": expires,
	})

	if err := provider.SendVerificationCode(phone, code); err != nil {
		return "", "", fmt.Errorf("短信发送失败: %w", err)
	}

	log.Printf("[SMS] 验证码已通过 %s 发送: phone=%s", provider.Name(), maskPhone(phone))
	return code, provider.Name(), nil
}

// VerifyPhoneCode 验证手机验证码
func VerifyPhoneCode(ctx context.Context, s store.StoreFacade, phone, code string) bool {
	wsID := "phone-verification"
	settings, err := s.GetSettings(ctx, wsID)
	if err != nil { return false }
	storedCode := settings["phone:"+phone+":code"]
	expiresStr := settings["phone:"+phone+":expires"]
	if storedCode == "" || expiresStr == "" { return false }
	expires, err := time.Parse(time.RFC3339, expiresStr)
	if err != nil || time.Now().After(expires) { return false }
	return subtle.ConstantTimeCompare([]byte(code), []byte(storedCode)) == 1
}

// ==================== 工具 ====================

func getEnvDefault(key, def string) string {
	if v := os.Getenv(key); v != "" { return v }
	return def
}

func maskPhone(phone string) string {
	if len(phone) < 7 { return "***" }
	return phone[:3] + "****" + phone[len(phone)-4:]
}

func generateNonce() string {
	b := sha256.Sum256([]byte(time.Now().String()))
	return hex.EncodeToString(b[:])[:16]
}
