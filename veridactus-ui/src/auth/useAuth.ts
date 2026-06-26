// Auth Hook: JWT 管理 + 自动刷新 + 角色判断
import { useState, useCallback, useEffect } from 'react';

const TOKEN_KEY = 'veridactus_token';
const REFRESH_KEY = 'veridactus_refresh';
const USER_KEY = 'veridactus_user';

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

export function useAuth(): AuthState {
  const [user, setUser] = useState<AuthUser | null>(getStoredUser());
  const [token, setToken] = useState<string | null>(getStoredToken());
  const [isLoading, setLoading] = useState(false);

  useEffect(() => {
    // 启动时尝试自动恢复
    const stored = getStoredToken();
    if (stored) {
      const u = parseJWT(stored);
      if (u) setUser(u);
    }
  }, []);

  const login = useCallback(async (provider: string) => {
    setLoading(true);
    try {
      const authResp = await fetch(`/api/v1/auth/login/${provider}`);
      const { auth_url } = await authResp.json();
      // 重定向到 OAuth provider
      window.location.href = auth_url;
    } catch (err) {
      console.error('Login failed:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  const logout = useCallback(() => {
    localStorage.removeItem(TOKEN_KEY);
    localStorage.removeItem(REFRESH_KEY);
    localStorage.removeItem(USER_KEY);
    setUser(null);
    setToken(null);
    window.location.href = '/login';
  }, []);

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
      return true;
    } catch { return false; }
  }, [logout]);

  return {
    user, token, isLoading,
    isAuthenticated: !!token && !!user,
    login, logout, refreshToken,
  };
}
