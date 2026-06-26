// 微信登录状态管理 — 生产级 QR 扫码轮询
package auth

import (
	"sync"
	"time"
)

// WeChatLoginState 微信扫码登录状态
type WeChatLoginState struct {
	State     string    `json:"state"`
	Status    string    `json:"status"` // "pending" | "completed" | "expired"
	Token     string    `json:"token,omitempty"`
	NeedBind  bool      `json:"need_bind_phone,omitempty"`
	CreatedAt time.Time `json:"created_at"`
}

var (
	weChatStates   = sync.Map{} // state -> *WeChatLoginState
	stateTimeouts  = make(map[string]*time.Timer)
	stateMutex     sync.Mutex
)

// RegisterWeChatState 注册一个新的微信登录状态 (state 创建时调用)
func RegisterWeChatState(state string) {
	ws := &WeChatLoginState{
		State:     state,
		Status:    "pending",
		CreatedAt: time.Now(),
	}
	weChatStates.Store(state, ws)

	// 5分钟超时
	stateMutex.Lock()
	stateTimeouts[state] = time.AfterFunc(5*time.Minute, func() {
		if v, ok := weChatStates.Load(state); ok {
			s := v.(*WeChatLoginState)
			if s.Status == "pending" {
				s.Status = "expired"
				weChatStates.Store(state, s)
			}
		}
	})
	stateMutex.Unlock()
}

// CompleteWeChatState 标记微信登录完成 (回调成功后调用)
func CompleteWeChatState(state, token string, needBind bool) {
	if v, ok := weChatStates.Load(state); ok {
		s := v.(*WeChatLoginState)
		s.Status = "completed"
		s.Token = token
		s.NeedBind = needBind
		weChatStates.Store(state, s)
	}
}

// GetWeChatState 获取微信登录状态 (前端轮询)
func GetWeChatState(state string) *WeChatLoginState {
	if v, ok := weChatStates.Load(state); ok {
		return v.(*WeChatLoginState)
	}
	return nil
}
