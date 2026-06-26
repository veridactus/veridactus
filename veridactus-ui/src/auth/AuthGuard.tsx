// Auth Guard — JWT 验证 + Plan 检测 + 路由保护
import { useEffect, useState } from 'react';
import { Navigate, useLocation } from 'react-router-dom';

const TOKEN_KEY = 'veridactus_token';

export function getToken(): string | null { return localStorage.getItem(TOKEN_KEY); }
export function setToken(token: string) { localStorage.setItem(TOKEN_KEY, token); }
export function clearToken() { localStorage.removeItem(TOKEN_KEY); localStorage.removeItem('veridactus_user'); }

export function isAuthenticated(): boolean {
  const token = getToken();
  if (!token) return false;
  const parts = token.split('.');
  if (parts.length !== 3) return false;
  try {
    const payload = JSON.parse(atob(parts[1]));
    if (payload.exp && payload.exp * 1000 < Date.now()) { clearToken(); return false; }
    return true;
  } catch { return false; }
}

// 获取用户 Plan (personal | enterprise)
export function getUserPlan(): string {
  try {
    const token = getToken();
    if (!token) return 'personal';
    return JSON.parse(atob(token.split('.')[1])).plan || 'personal';
  } catch { return 'personal'; }
}

export function getUserId(): string | null {
  try {
    const token = getToken();
    if (!token) return null;
    return JSON.parse(atob(token.split('.')[1])).sub || null;
  } catch { return null; }
}

interface Props { children: React.ReactNode; }

export default function AuthGuard({ children }: Props) {
  const location = useLocation();
  const [checking, setChecking] = useState(true);

  useEffect(() => { setChecking(false); }, []);

  if (checking) {
    return (
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: '#0B0F19' }}>
        <div style={{ width: 32, height: 32, border: '3px solid rgba(108,92,231,0.3)', borderTopColor: '#6c5ce7', borderRadius: '50%', animation: 'spin 0.8s linear infinite' }} />
        <style>{'@keyframes spin{to{transform:rotate(360deg)}}'}</style>
      </div>
    );
  }

  if (!isAuthenticated()) {
    return <Navigate to="/login" state={{ from: location.pathname }} replace />;
  }

  return <>{children}</>;
}
