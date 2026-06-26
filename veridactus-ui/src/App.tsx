import { useEffect } from 'react';
import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom';
import { lazy, Suspense } from 'react';
import { I18nProvider } from './i18n';
import { useThemeStore } from './store';
import AuthGuard from './auth/AuthGuard';
import Sidebar from './components/layout/Sidebar';
import UserHeader from './components/layout/UserHeader';
import BottomStatusBar from './components/layout/BottomStatusBar';
import DataFlowBackground from './components/viz/DataFlowBackground';
import { useMetricsStream } from './hooks/useMetricsStream';
import { ToastContainer, useToastMount } from './components/ui/Toast';

const Dashboard = lazy(() => import('./pages/Dashboard'));
const Pipelines = lazy(() => import('./pages/Pipelines'));
const PipelineEdit = lazy(() => import('./pages/PipelineEdit'));
const PipelineDesigner = lazy(() => import('./pages/PipelineDesigner'));
const AuditCenter = lazy(() => import('./pages/AuditCenter'));
const Plugins = lazy(() => import('./pages/Plugins'));
const ApiKeys = lazy(() => import('./pages/ApiKeys'));
const Models = lazy(() => import('./pages/Models'));
const Settings = lazy(() => import('./pages/Settings'));

// Phase 3: 新引擎
const LoginPage = lazy(() => import('./auth/LoginPage'));
const PhoneBind = lazy(() => import('./auth/PhoneBind'));
const ChatPage = lazy(() => import('./engines/chat/ChatPage'));
const VaultPage = lazy(() => import('./engines/vault/VaultPage'));
const VaultDetail = lazy(() => import('./engines/vault/VaultDetail'));

// Phase 4: 企业级
const AuditorCommandCenter = lazy(() => import('./engines/vault/AuditCenter'));
const BrandSettings = lazy(() => import('./admin/BrandSettings'));
const ComplianceReport = lazy(() => import('./pages/ComplianceReport'));
const PluginMarket = lazy(() => import('./pages/PluginMarket'));
const OnboardingPage = lazy(() => import('./auth/OnboardingPage'));
const PlaygroundPage = lazy(() => import('./engines/devhub/PlaygroundPage'));

function LoadingScreen() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', height: '100vh', background: 'var(--bg-primary)' }}>
      <div style={{ textAlign: 'center' }}>
        <div style={{ width: 40, height: 40, border: '3px solid var(--border-default)', borderTopColor: '#00d4aa', borderRadius: '50%', animation: 'spin 0.8s linear infinite', margin: '0 auto 16px' }} />
        <p style={{ color: 'var(--text-secondary)', fontSize: 14, fontWeight: 500 }}>Loading VERIDACTUS...</p>
      </div>
      <style>{'@keyframes spin { to { transform: rotate(360deg); } }'}</style>
    </div>
  );
}

function AppLayout() {
  const theme = useThemeStore(s => s.theme);
  useMetricsStream();
  useToastMount();

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  return (
    <AuthGuard>
    <div style={{ display: 'flex', height: '100vh', background: 'var(--bg-primary)', transition: 'background 0.3s ease', overflow: 'hidden' }}>
      <DataFlowBackground />
      <Sidebar />
      <UserHeader />
      <main className="main-content">
        <div className="content-area">
          <Suspense fallback={<LoadingScreen />}>
            <Routes>
              <Route path="/dashboard" element={<Dashboard />} />
              <Route path="/pipelines" element={<Pipelines />} />
              <Route path="/pipelines/new" element={<PipelineDesigner />} />
              <Route path="/pipelines/design/:id?" element={<PipelineDesigner />} />
              <Route path="/pipelines/edit/:id" element={<PipelineEdit />} />
              <Route path="/audit" element={<AuditCenter />} />
              <Route path="/plugins" element={<Plugins />} />
              <Route path="/api-keys" element={<ApiKeys />} />
              <Route path="/models" element={<Models />} />
              <Route path="/settings" element={<Settings />} />
              {/* Phase 3: 新引擎 */}
              <Route path="/chat" element={<ChatPage />} />
              <Route path="/vault" element={<VaultPage />} />
              <Route path="/vault/:traceId" element={<VaultDetail />} />
              {/* Phase 4: 企业级 */}
              <Route path="/playground" element={<PlaygroundPage />} />
              <Route path="/audit-center" element={<AuditorCommandCenter />} />
              <Route path="/brand" element={<BrandSettings />} />
              <Route path="/compliance" element={<ComplianceReport />} />
              <Route path="/plugins/market" element={<PluginMarket />} />
              <Route path="*" element={<Navigate to="/chat" replace />} />
            </Routes>
          </Suspense>
        </div>
      </main>
      <BottomStatusBar />
    </div>
    </AuthGuard>
  );
}

export default function App() {
  return (
    <BrowserRouter>
      <I18nProvider>
        <ToastContainer />
        <Routes>
          <Route path="/login" element={<LoginPage />} />
          <Route path="/bind-phone" element={<PhoneBind />} />
          <Route path="/onboarding" element={<OnboardingPage />} />
          <Route path="/*" element={<AppLayout />} />
        </Routes>
      </I18nProvider>
    </BrowserRouter>
  );
}
