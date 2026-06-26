// 微信 OAuth 回调 — 生产级实现
// 返回 HTML 页面，自动存储 JWT + 关闭窗口
package main

import (
	"encoding/json"
	"fmt"
	"html/template"
	"net/http"

	"github.com/veridactus/control-plane/internal/auth"
)

// wechatCallbackHTML 微信回调成功页面模板
const wechatCallbackHTML = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>VERIDACTUS — 微信登录</title>
<style>
  * { margin:0; padding:0; box-sizing:border-box; }
  body { font-family: system-ui, -apple-system, sans-serif; background: #0B0F19; color: #e0e6f0; display: flex; align-items: center; justify-content: center; min-height: 100vh; }
  .card { background: rgba(19,22,51,0.95); border-radius: 20px; padding: 48px 36px; max-width: 420px; width: 100%; text-align: center; border: 1px solid rgba(7,193,96,0.2); box-shadow: 0 0 60px rgba(7,193,96,0.1); }
  .icon { width: 64px; height: 64px; border-radius: 16px; background: linear-gradient(135deg, #07c160, #06ad56); display: flex; align-items: center; justify-content: center; margin: 0 auto 20px; font-size: 32px; animation: pulse 2s infinite; }
  @keyframes pulse { 0%,100% { transform: scale(1); } 50% { transform: scale(1.05); } }
  h1 { font-size: 24px; font-weight: 700; margin-bottom: 8px; }
  .sub { font-size: 14px; color: #8892b0; margin-bottom: 24px; }
  .user-info { background: rgba(255,255,255,0.04); border-radius: 12px; padding: 16px; margin-bottom: 24px; text-align: left; }
  .user-info .row { display: flex; justify-content: space-between; padding: 6px 0; font-size: 13px; }
  .user-info .row .label { color: #8892b0; }
  .check { color: #00d4aa; font-size: 48px; margin-bottom: 12px; }
  .btn { display: inline-block; padding: 12px 32px; border-radius: 12px; background: linear-gradient(135deg, #6c5ce7, #00d4aa); color: #000; font-weight: 700; font-size: 15px; text-decoration: none; cursor: pointer; border: none; }
  .btn-secondary { background: rgba(255,255,255,0.06); color: #8892b0; border: 1px solid rgba(255,255,255,0.08); margin-left: 12px; }
  .actions { display: flex; justify-content: center; gap: 12px; margin-top: 8px; }
  .error { color: #ff7675; font-size: 14px; }
</style>
</head>
<body>
<div class="card">
  {{if .Success}}
    <div class="check">✅</div>
    <div class="icon">💬</div>
    <h1>微信登录成功</h1>
    <p class="sub">欢迎回来，{{.DisplayName}}</p>
    <div class="user-info">
      <div class="row"><span class="label">账户</span><span>{{.Email}}</span></div>
      <div class="row"><span class="label">方案</span><span>{{.Plan}}</span></div>
      <div class="row"><span class="label">组织</span><span>{{.OrgName}}</span></div>
    </div>
    <div class="actions">
      <a href="{{.RedirectURL}}" class="btn">进入 VERIDACTUS</a>
      {{if .NeedBindPhone}}
      <a href="{{.BindPhoneURL}}" class="btn btn-secondary">绑定手机号</a>
      {{end}}
    </div>
  {{else}}
    <div class="icon" style="background: linear-gradient(135deg, #ff7675, #d63031);">⚠️</div>
    <h1>登录失败</h1>
    <p class="sub error">{{.Error}}</p>
    <a href="{{.LoginURL}}" class="btn">返回登录页</a>
  {{end}}
</div>
<script>
  // 存储 JWT 到 localStorage
  {{if .Success}}
  localStorage.setItem('veridactus_token', '{{.Token}}');
  localStorage.setItem('veridactus_user', JSON.stringify({{.UserJSON}}));
  // 如果是从弹窗打开的，发送消息给父窗口
  if (window.opener) {
    window.opener.postMessage({ type: 'wechat_login_success', token: '{{.Token}}', needBindPhone: {{.NeedBindPhone}} }, '*');
    setTimeout(function() { window.close(); }, 1500);
  }
  {{end}}
</script>
</body>
</html>`

// handleWeChatCallbackPage 微信回调 HTML 页面
// 用户从微信扫码后跳转到此页面，自动存储 JWT 并显示欢迎信息
func (srv *Server) handleWeChatCallbackPage() http.HandlerFunc {
	tmpl := template.Must(template.New("wechat").Parse(wechatCallbackHTML))

	type pageData struct {
		Success       bool
		Token         string
		DisplayName   string
		Email         string
		Plan          string
		OrgName       string
		NeedBindPhone bool
		RedirectURL   string
		BindPhoneURL  string
		LoginURL      string
		UserJSON      template.JS
		Error         string
	}

	return func(w http.ResponseWriter, r *http.Request) {
		code := r.URL.Query().Get("code")
		state := r.URL.Query().Get("state")
		loginURL := "/login"
		redirectURL := "/chat"
		bindPhoneURL := "/bind-phone"

		if code == "" {
			tmpl.Execute(w, pageData{Success: false, Error: "缺少授权码", LoginURL: loginURL})
			return
		}

		// 调用微信 OAuth
		wx := auth.NewWeChatProvider()
		info, err := wx.ExchangeCode(r.Context(), code)
		if err != nil {
			tmpl.Execute(w, pageData{Success: false, Error: "微信授权失败: " + err.Error(), LoginURL: loginURL})
			return
		}

		// 登录或创建用户
		svc := auth.NewWeChatLoginService(srv.store, srv.jwtSecret)
		result, err := svc.LoginOrCreateByWeChat(r.Context(), info)
		if err != nil {
			tmpl.Execute(w, pageData{Success: false, Error: "登录失败: " + err.Error(), LoginURL: loginURL})
			return
		}

		if result.Token == "" {
			tmpl.Execute(w, pageData{Success: false, Error: "认证失败", LoginURL: loginURL})
			return
		}

		plan := "personal"
		if result.Org != nil {
			plan = result.Org.Plan
		}
		orgName := ""
		if result.Org != nil {
			orgName = result.Org.Name
		}

		userJSON, _ := json.Marshal(map[string]interface{}{
			"id":           result.User.ID,
			"email":        result.User.Email,
			"display_name": result.User.DisplayName,
			"plan":         plan,
		})

		if state != "" {
			auth.CompleteWeChatState(state, result.Token, result.NeedBindPhone)
			redirectURL = fmt.Sprintf("/login?wechat_token=%s&state=%s", result.Token, state)
		}

		tmpl.Execute(w, pageData{
			Success:       true,
			Token:         result.Token,
			DisplayName:   result.User.DisplayName,
			Email:         result.User.Email,
			Plan:          plan,
			OrgName:       orgName,
			NeedBindPhone: result.NeedBindPhone,
			RedirectURL:   redirectURL,
			BindPhoneURL:  bindPhoneURL,
			UserJSON:      template.JS(userJSON),
		})
	}
}
