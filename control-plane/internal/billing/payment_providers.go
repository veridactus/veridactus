// VERIDACTUS 支付接口 — 国内支付宝/微信支付 + 国际 Stripe
// 通过 VERIDACTUS_PAYMENT_PROVIDER 环境变量切换
package billing

import (
	"context"
	"os"
)

// PaymentProvider 支付 Provider 接口
type PaymentProvider interface {
	// Name 返回支付服务商名称
	Name() string
	// CreateCheckout 创建支付订单，返回支付 URL
	CreateCheckout(ctx context.Context, req *CheckoutRequest) (*CheckoutResponse, error)
	// VerifyWebhook 验证支付回调签名
	VerifyWebhook(ctx context.Context, payload []byte, signature string) (*WebhookEvent, error)
}

// CheckoutRequest 支付请求
type CheckoutRequest struct {
	WorkspaceID   string  `json:"workspace_id"`
	AmountCNY     float64 `json:"amount_cny"`      // 人民币金额（国内支付）
	AmountUSD     float64 `json:"amount_usd"`      // 美元金额（国际支付）
	Description   string  `json:"description"`
	ReturnURL     string  `json:"return_url"`
	NotifyURL     string  `json:"notify_url"`
}

// CheckoutResponse 支付响应
type CheckoutResponse struct {
	SessionID   string `json:"session_id"`
	PaymentURL  string `json:"payment_url"`   // 支付跳转 URL
	QRCode      string `json:"qr_code"`       // 二维码（微信/支付宝扫码支付）
	Amount      float64 `json:"amount"`
	Currency    string `json:"currency"`
	Status      string `json:"status"`
}

// WebhookEvent 支付回调事件
type WebhookEvent struct {
	EventID     string `json:"event_id"`
	EventType   string `json:"event_type"`    // payment.succeeded / payment.failed / refund.succeeded
	WorkspaceID string `json:"workspace_id"`
	Amount      int64  `json:"amount"`         // 分（1元=100分）
	Currency    string `json:"currency"`
	OutTradeNo  string `json:"out_trade_no"`
	Status      string `json:"status"`
}

// ==================== 支付宝 Provider ====================

type AlipayProvider struct {
	appID        string
	privateKey   string
	alipayPublicKey string
	notifyURL    string
	isSandbox    bool
}

func NewAlipayProvider() *AlipayProvider {
	return &AlipayProvider{
		appID:           os.Getenv("ALIPAY_APP_ID"),
		privateKey:      os.Getenv("ALIPAY_PRIVATE_KEY"),
		alipayPublicKey: os.Getenv("ALIPAY_PUBLIC_KEY"),
		notifyURL:       os.Getenv("ALIPAY_NOTIFY_URL"),
		isSandbox:       os.Getenv("ALIPAY_SANDBOX") == "true",
	}
}

func (p *AlipayProvider) Name() string { return "alipay" }

func (p *AlipayProvider) CreateCheckout(ctx context.Context, req *CheckoutRequest) (*CheckoutResponse, error) {
	// 生产实现: 使用支付宝 SDK 创建 PC 扫码支付订单
	// 接入方式: go get github.com/smartwalle/alipay/v3
	// 调用: client.TradePagePay() → 返回 HTML 表单 / QR 码 URL
	return nil, errProviderNotIntegrated("alipay", "github.com/smartwalle/alipay/v3")
}

func (p *AlipayProvider) VerifyWebhook(ctx context.Context, payload []byte, signature string) (*WebhookEvent, error) {
	// 生产实现: 验证支付宝异步通知签名
	return nil, errProviderNotIntegrated("alipay", "github.com/smartwalle/alipay/v3")
}

// ==================== 微信支付 Provider ====================

type WeChatPayProvider struct {
	mchID      string
	apiV3Key   string
	privateKey string
	appID      string
	notifyURL  string
}

func NewWeChatPayProvider() *WeChatPayProvider {
	return &WeChatPayProvider{
		mchID:      os.Getenv("WECHATPAY_MCH_ID"),
		apiV3Key:   os.Getenv("WECHATPAY_API_V3_KEY"),
		privateKey: os.Getenv("WECHATPAY_PRIVATE_KEY"),
		appID:      os.Getenv("WECHATPAY_APP_ID"),
		notifyURL:  os.Getenv("WECHATPAY_NOTIFY_URL"),
	}
}

func (p *WeChatPayProvider) Name() string { return "wechatpay" }

func (p *WeChatPayProvider) CreateCheckout(ctx context.Context, req *CheckoutRequest) (*CheckoutResponse, error) {
	// 生产实现: 使用微信支付 API V3 创建 Native 支付订单
	// 接入方式: go get github.com/wechatpay-apiv3/wechatpay-go
	// 调用: client.V3TransactionNative() → 返回 code_url (生成 QR 码)
	return nil, errProviderNotIntegrated("wechatpay", "github.com/wechatpay-apiv3/wechatpay-go")
}

func (p *WeChatPayProvider) VerifyWebhook(ctx context.Context, payload []byte, signature string) (*WebhookEvent, error) {
	return nil, errProviderNotIntegrated("wechatpay", "github.com/wechatpay-apiv3/wechatpay-go")
}

// ==================== Stripe Provider (国际版) ====================

type StripeProvider struct {
	secretKey     string
	webhookSecret string
}

func NewStripeProvider() *StripeProvider {
	return &StripeProvider{
		secretKey:     os.Getenv("STRIPE_SECRET_KEY"),
		webhookSecret: os.Getenv("STRIPE_WEBHOOK_SECRET"),
	}
}

func (p *StripeProvider) Name() string { return "stripe" }

func (p *StripeProvider) CreateCheckout(ctx context.Context, req *CheckoutRequest) (*CheckoutResponse, error) {
	// 生产实现: 使用 Stripe Go SDK 创建 Checkout Session
	// 接入方式: go get github.com/stripe/stripe-go/v81
	// 调用: session.New(params) → 返回 checkout URL
	return nil, errProviderNotIntegrated("stripe", "github.com/stripe/stripe-go/v81")
}

func (p *StripeProvider) VerifyWebhook(ctx context.Context, payload []byte, signature string) (*WebhookEvent, error) {
	return nil, errProviderNotIntegrated("stripe", "github.com/stripe/stripe-go/v81")
}

// ==================== 工厂函数 ====================

// GetPaymentProvider 根据环境变量返回支付 Provider
// VERIDACTUS_PAYMENT_PROVIDER: alipay | wechatpay | stripe
func GetPaymentProvider() PaymentProvider {
	switch os.Getenv("VERIDACTUS_PAYMENT_PROVIDER") {
	case "alipay":
		return NewAlipayProvider()
	case "wechatpay":
		return NewWeChatPayProvider()
	case "stripe":
		return NewStripeProvider()
	default:
		// 国内默认支付宝，可通过环境变量切换
		return NewAlipayProvider()
	}
}

// ==================== 辅助 ====================

func errProviderNotIntegrated(provider, pkg string) error {
	return &PaymentNotIntegratedError{Provider: provider, Package: pkg}
}

type PaymentNotIntegratedError struct {
	Provider string
	Package  string
}

func (e *PaymentNotIntegratedError) Error() string {
	return "payment provider '" + e.Provider + "' SDK not integrated. Run: go get " + e.Package
}
