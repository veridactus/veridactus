// Auth Hook: JWT 管理 + 自动刷新 + 活动检测 + 不活跃退出
import { useState, useCallback, useEffect, useRef } from 'react';

const TOKEN_KEY = 'veridactus_token';
const REFRESH_KEY = 'veridactus_refresh';
const USER_KEY = 'veridactus_user';
const LAST_ACTIVE_KEY = 'veridactus_last_active';

// 配置
const REFRESH_INTERVAL_MS = 10 * 60 * 1000;  // 每 10 分钟刷新 token（到期前刷新，避免中断）
const INACTIVE_TIMEOUT_MS = 30 * 60 * 1000;   // 30 分钟无操作后自动退出
const ACTIVITY_EVENTS = ['mousedown', 'keydown', 'scroll', 'touchstart', 'mousemove'];

export interface AuthUser {
  id: string;
  email: string;
  display_name: string;
  avatar_url: string;
  org_id: string;
  workspace_id: string;
  role: string;
}

export interface AuthState {
  user: AuthUser | null;
  token: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  login: (provider: string) => Promise<void>;
  logout: () => void;
  refreshToken: () => Promise<boolean>;
  // 手动更新最后活跃时间（API 调用成功时调用）
  touchActivity: () => void;
}

function parseJWT(token: string): AuthUser | null {
  try {
    const base64 = token.split('.')[1];
    const decoded = JSON.parse(atob(base64));
    return {
      id: decoded.sub || decoded.user_id,
      email: decoded.email,
      display_name: decoded.name || decoded.email,
      avatar_url: '',
      org_id: decoded.org_id,
      workspace_id: decoded.workspace_id,
      role: decoded.role || 'developer',
    };
  } catch { return null; }
}

function saveAuth(token: string, refreshToken: string, user: AuthUser) {
  localStorage.setItem(TOKEN_KEY, token);
  localStorage.setItem(REFRESH_KEY, refreshToken);
  localStorage.setItem(USER_KEY, JSON.stringify(user));
  localStorage.setItem(LAST_ACTIVE_KEY, String(Date.now()));
}

export function getStoredToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

export function getStoredUser(): AuthUser | null {
  try {
    const raw = localStorage.getItem(USER_KEY);
    return raw ? JSON.parse(raw) : null;
  } catch { return null; }
}

/** 更新最后活跃时间（不触发重渲染） */
function updateLastActive() {
  localStorage.setItem(LAST_ACTIVE_KEY, String(Date.now()));
}

export function useAuth(): AuthState {
  const [user, setUser] = useState<AuthUser | null>(getStoredUser());
  const [token, setToken] = useState<string | null>(getStoredToken());
  const [isLoading, setLoading] = useState(false);
  const inactiveTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const refreshTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const logoutRef = useRef<() => void>(() => {});

  // 不活跃检测 + 自动退出
  const checkInactive = useCallback(() => {
    const lastStr = localStorage.getItem(LAST_ACTIVE_KEY);
    if (lastStr) {
      const elapsed = Date.now() - Number(lastStr);
      if (elapsed > INACTIVE_TIMEOUT_MS) {
        logoutRef.current();
      }
    }
  }, []);

  // 退出
  const logout = useCallback(() => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(REFRESH_KEY);
    localStorage.removeItem(USER_KEY);
    localStorage.removeItem(LAST_ACTIVE_KEY);
    // 清理 Chat 缓存（防止多用户共享缓存）
    localStorage.removeItem('v_msgs');
    localStorage.removeItem('v_conv');
    localStorage.removeItem('v_stream');
    if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
    if (inactiveTimerRef.current) clearInterval(inactiveTimerRef.current);
    // 移除活动监听
    ACTIVITY_EVENTS.forEach(ev => document.removeEventListener(ev, updateLastActive));
    setUser(null);
    setToken(null);
    window.location.href = '/login';
  }, []);

  // 保持 logoutRef 最新
  logoutRef.current = logout;

  // 自动刷新 token（每 10 分钟）
  const refreshToken = useCallback(async (): Promise<boolean> => {
    const refresh = localStorage.getItem(REFRESH_KEY);
    if (!refresh) return false;
    try {
      const res = await fetch('/api/v1/auth/refresh', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ refresh_token: refresh }),
      });
      if (!res.ok) { logout(); return false; }
      const data = await res.json();
      const u = parseJWT(data.access_token);
      if (u) { saveAuth(data.access_token, data.refresh_token, u); setToken(data.access_token); setUser(u); }
      updateLastActive();
      return true;
    } catch { return false; }
  }, [logout]);

  // 初始化：启动自动刷新 + 不活跃检测 + 活动监听
  useEffect(() => {
    const stored = getStoredToken();
    if (!stored) return;

    const u = parseJWT(stored);
    if (u) setUser(u);

    updateLastActive();

    // 自动刷新定时器（仅已登录时启动）
    refreshTimerRef.current = setInterval(() => {
      refreshToken().catch(() => {});
    }, REFRESH_INTERVAL_MS);

    // 不活跃检测定时器（每 60 秒检查一次）
    inactiveTimerRef.current = setInterval(checkInactive, 60_000);

    // 用户活动监听 — 任何操作都重置不活跃计时
    ACTIVITY_EVENTS.forEach(ev =>
      document.addEventListener(ev, updateLastActive, { passive: true })
    );

    return () => {
      if (refreshTimerRef.current) clearInterval(refreshTimerRef.current);
      if (inactiveTimerRef.current) clearInterval(inactiveTimerRef.current);
      ACTIVITY_EVENTS.forEach(ev => document.removeEventListener(ev, updateLastActive));
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const login = useCallback(async (provider: string) => {
    setLoading(true);
    try {
      const authResp = await fetch(`/api/v1/auth/login/${provider}`);
      const { auth_url } = await authResp.json();
      window.location.href = auth_url;
    } catch (err) {
      console.error('Login failed:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  const touchActivity = useCallback(() => {
    updateLastActive();
  }, []);

  return {
    user, token, isLoading,
    isAuthenticated: !!token && !!user,
    login, logout, refreshToken, touchActivity,
  };
}
