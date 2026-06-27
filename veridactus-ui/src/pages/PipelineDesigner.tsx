import { useCallback, useEffect, useRef, useState, useMemo } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { useParams, useNavigate } from 'react-router-dom';
import ReactFlow, {
  Background, Controls, MiniMap, useNodesState, useEdgesState,
  addEdge, Connection, Node, Edge, ReactFlowProvider, MarkerType,
  BackgroundVariant,
} from 'reactflow';
import 'reactflow/dist/style.css';
import GlassCard from '../components/ui/GlassCard';
import { useI18n } from '../i18n';
import { createPipeline, updatePipeline, getPipeline } from '../api';
import type { Pipeline } from '../types';
import TemplateSelector from '../engines/pipeline/TemplateSelector';
import {
  ArrowLeft, Save, Send, GripVertical, AlarmClock, CheckCircle, AlertCircle,
  Zap, Shield, Clock, Sparkles, Loader2, Trash2, Copy, Play, Star, ChevronDown, ChevronUp
} from 'lucide-react';

const stages = {
  pre_request: { id: 'pre_request', label: 'Pre-Request', color: '#00d4aa', desc: '请求预处理', emoji: '🛂', glow: 'rgba(0, 212, 170, 0.3)' },
  streaming:  { id: 'streaming',  label: 'Streaming',  color: '#74b9ff', desc: '流式处理',   emoji: '🔁', glow: 'rgba(116, 185, 255, 0.3)' },
  post_response: { id: 'post_response', label: 'Post-Response', color: '#a29bfe', desc: '响应后处理', emoji: '✅', glow: 'rgba(162, 155, 254, 0.3)' },
  async:      { id: 'async',      label: 'Async',      color: '#fdcb6e', desc: '异步任务',   emoji: '⏳', glow: 'rgba(253, 203, 110, 0.3)' },
} as const;

const pluginsByStage: Record<string, any[]> = {
  pre_request: [
    { id: 'budget', name: 'Budget Guard',     category: 'budget',  type: 'native', description: '预算限制与速率控制', icon: '💰' },
    { id: 'auth',   name: 'Auth Validator',   category: 'auth',    type: 'native', description: 'API 密钥与令牌验证', icon: '🔐' },
    { id: 'route',  name: 'Route Selector',   category: 'route',   type: 'native', description: '智能模型路由与故障转移', icon: '🧭' },
  ],
  streaming: [
    { id: 'guard_keyword', name: 'Keyword Guardrail', category: 'guardrail', type: 'wasm', description: '基于 Wasm 的实时内容过滤', icon: '🛡️' },
    { id: 'pii_regex',     name: 'PII Masking',       category: 'pii',      type: 'wasm', description: '正则表达式 PII 检测与脱敏', icon: '🎭' },
  ],
  post_response: [
    { id: 'trace_finalizer', name: 'Trace Finalizer', category: 'proof', type: 'native', description: 'L0 签名计算与追踪终结', icon: '📜' },
  ],
  async: [
    { id: 'drift',     name: 'Drift Detector',  category: 'drift',     type: 'grpc', description: '嵌入漂移语义一致性分析', icon: '📊' },
    { id: 'guarantee', name: 'C-SafeGen',       category: 'guarantee', type: 'grpc', description: '认证保证计算 (Conformal)', icon: '🏆' },
    { id: 'proof_l1',  name: 'TEE Attestation', category: 'proof',     type: 'grpc', description: 'L1 TEE 证明生成与验证', icon: '🔒' },
  ],
};

function pluginNodeStyle(color: string, glow?: string) {
  return {
    background: `linear-gradient(135deg, ${color}15 0%, ${color}08 100%)`,
    border: `1.5px solid ${color}50`,
    borderRadius: 12,
    padding: '10px 16px',
    color: 'var(--text-primary)',
    fontSize: 12,
    fontWeight: 500,
    minWidth: 160,
    boxShadow: glow ? `0 0 20px ${glow}, 0 4px 12px rgba(0,0,0,0.3)` : '0 4px 12px rgba(0,0,0,0.3)',
    backdropFilter: 'blur(8px)',
  };
}

const SPECIAL_NODE_STYLE: Record<string, React.CSSProperties> = {
  client: {
    background: 'linear-gradient(135deg, #6c5ce7 0%, #a29bfe 100%)',
    border: '2px solid rgba(108,92,231,0.8)',
    borderRadius: 16,
    padding: '14px 24px',
    color: '#fff',
    fontSize: 14,
    fontWeight: 700,
    minWidth: 150,
    textAlign: 'center',
    boxShadow: '0 0 30px rgba(108,92,231,0.5), 0 8px 24px rgba(0,0,0,0.4)',
  },
  upstream: {
    background: 'linear-gradient(135deg, #f093fb 0%, #f5576c 100%)',
    border: '2px solid rgba(245,87,108,0.8)',
    borderRadius: 16,
    padding: '14px 24px',
    color: '#fff',
    fontSize: 14,
    fontWeight: 700,
    minWidth: 150,
    textAlign: 'center',
    boxShadow: '0 0 30px rgba(245,87,108,0.5), 0 8px 24px rgba(0,0,0,0.4)',
  },
};

const SYNC_Y = 200;
const ASYNC_Y = 480;
const MAIN_GAP = 220;
const ASYNC_GAP = 180;

function stageX(index: number) { return 100 + index * MAIN_GAP; }

const SYNC_EDGE = { stroke: '#6c5ce7', strokeWidth: 3 };
const ASYNC_EDGE = { stroke: '#fdcb6e', strokeWidth: 2.5, strokeDasharray: '10 6' };

function isAsyncEdge(edge: { source: string; target: string }, nodes: Node[]) {
  const s = nodes.find(n => n.id === edge.source);
  const t = nodes.find(n => n.id === edge.target);
  return s?.data?.type === 'async' || t?.data?.type === 'async' || s?.data?.stage === 'async' || t?.data?.stage === 'async';
}

function ParticleField() {
  return (
    <div style={{ position: 'absolute', inset: 0, overflow: 'hidden', pointerEvents: 'none', zIndex: 0 }}>
      {[...Array(20)].map((_, i) => (
        <div
          key={i}
          style={{
            position: 'absolute',
            width: Math.random() * 4 + 2,
            height: Math.random() * 4 + 2,
            borderRadius: '50%',
            background: ['#6c5ce7', '#00d4aa', '#74b9ff', '#fdcb6e'][i % 4],
            opacity: Math.random() * 0.3 + 0.1,
            left: `${Math.random() * 100}%`,
            top: `${Math.random() * 100}%`,
            animation: `float ${5 + Math.random() * 10}s ease-in-out infinite`,
            animationDelay: `${Math.random() * 5}s`,
          }}
        />
      ))}
      <style>{`
        @keyframes float {
          0%, 100% { transform: translateY(0) translateX(0); }
          25% { transform: translateY(-20px) translateX(10px); }
          50% { transform: translateY(-10px) translateX(-10px); }
          75% { transform: translateY(-30px) translateX(5px); }
        }
      `}</style>
    </div>
  );
}

interface ToastProps {
  message: string;
  type: 'success' | 'error' | 'info';
  visible: boolean;
}
function Toast({ message, type, visible }: ToastProps) {
  const icons = { success: <CheckCircle size={18} />, error: <AlertCircle size={18} />, info: <Sparkles size={18} /> };
  const colors = { success: '#00d4aa', error: '#ff7675', info: '#6c5ce7' };
  return (
    <motion.div
      initial={{ opacity: 0, y: 20, scale: 0.9 }}
      animate={{ opacity: visible ? 1 : 0, y: visible ? 0 : 20, scale: visible ? 1 : 0.9 }}
      style={{
        position: 'fixed', bottom: 32, right: 32, zIndex: 1000,
        background: 'rgba(19, 22, 51, 0.95)',
        backdropFilter: 'blur(16px)',
        border: `1px solid ${colors[type]}40`,
        borderRadius: 12,
        padding: '12px 20px',
        display: 'flex', alignItems: 'center', gap: 12,
        boxShadow: `0 0 30px ${colors[type]}30, 0 8px 32px rgba(0,0,0,0.5)`,
        color: colors[type],
        fontWeight: 600, fontSize: 14,
      }}
    >
      {icons[type]}
      {message}
    </motion.div>
  );
}

function LoadingSpinner({ size = 24 }: { size?: number }) {
  return (
    <motion.div
      animate={{ rotate: 360 }}
      transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
      style={{ display: 'flex', alignItems: 'center', justifyContent: 'center' }}
    >
      <Loader2 size={size} style={{ color: '#6c5ce7' }} />
    </motion.div>
  );
}

function PipelineCanvas({ pipelineId }: { pipelineId: string | null }) {
  const { t, locale } = useI18n();
  const wrapper = useRef<HTMLDivElement>(null);
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selected, setSelected] = useState<Node | null>(null);
  const [rf, setRf] = useState<any>(null);
  const [dragOver, setDragOver] = useState(false);
  const [showLibrary, setShowLibrary] = useState(true);

  useEffect(() => {
    if (!pipelineId) return;
    (async () => {
      try {
        const pipeline = await getPipeline(pipelineId);
        if (!pipeline || !pipeline.stages || pipeline.stages.length === 0) return;

        const ns: Node[] = [];
        const es: any[] = [];

        // 客户端入口节点
        ns.push({
          id: 'client-in', type: 'default', position: { x: -60, y: SYNC_Y - 40 },
          data: { label: '🖥️ Client\nRequest', stage: 'client', name: 'Client', type: 'client' },
          style: SPECIAL_NODE_STYLE.client
        });
        // 上游 LLM 网关节点
        ns.push({
          id: 'llm-upstream', type: 'default', position: { x: stageX(1.5), y: SYNC_Y - 100 },
          data: { label: '☁️ Upstream\nLLM API', stage: 'upstream', name: 'Upstream LLM', type: 'gateway' },
          style: SPECIAL_NODE_STYLE.upstream
        });
        // 客户端出口节点
        ns.push({
          id: 'client-out', type: 'default', position: { x: stageX(3.5), y: SYNC_Y - 40 },
          data: { label: '🖥️ Client\nResponse', stage: 'client', name: 'Client', type: 'client' },
          style: SPECIAL_NODE_STYLE.client
        });

        // 按阶段渲染已保存的插件
        const stageOrder: Record<string, { x: number; y: number; color: string; glow: string }> = {
          pre_request: { x: stageX(0.3), y: SYNC_Y, color: stages.pre_request.color, glow: stages.pre_request.glow },
          streaming: { x: stageX(2), y: SYNC_Y, color: stages.streaming.color, glow: stages.streaming.glow },
          post_response: { x: stageX(2.8), y: SYNC_Y, color: stages.post_response.color, glow: stages.post_response.glow },
          async: { x: stageX(0.8), y: ASYNC_Y, color: stages.async.color, glow: stages.async.glow },
        };

        pipeline.stages.forEach((stage: any) => {
          const placement: string = stage.placement || 'pre_request';
          const layout = stageOrder[placement] || stageOrder.pre_request;
          const isAsync = placement === 'async';
          const gap = isAsync ? ASYNC_GAP : 40;

          (stage.plugins || []).forEach((plugin: any, i: number) => {
            const nodeId = `plugin-${placement}-${i}`;
            ns.push({
              id: nodeId, type: 'default',
              position: { x: layout.x + i * gap, y: layout.y + i * 60 },
              data: {
                label: `${plugin.name}`,
                stage: placement,
                name: plugin.name,
                type: plugin.type || 'native',
                plugin: plugin,
              },
              style: pluginNodeStyle(layout.color, layout.glow)
            });
          });
        });

        // 构建连线
        const preNodes = ns.filter(n => n.data?.stage === 'pre_request');
        const streamNodes = ns.filter(n => n.data?.stage === 'streaming');
        const postNodes = ns.filter(n => n.data?.stage === 'post_response');
        const asyncNodes = ns.filter(n => n.data?.stage === 'async');

        preNodes.forEach(n => {
          es.push({
            id: `e-cin-${n.id}`, source: 'client-in', target: n.id,
            animated: true, style: SYNC_EDGE,
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7', width: 20, height: 20 }
          });
          es.push({
            id: `e-${n.id}-up`, source: n.id, target: 'llm-upstream',
            animated: true, style: SYNC_EDGE,
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
          });
        });

        streamNodes.forEach(n => {
          es.push({
            id: `e-up-${n.id}`, source: 'llm-upstream', target: n.id,
            animated: true, style: SYNC_EDGE,
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
          });
          postNodes.forEach(pn => {
            es.push({
              id: `e-${n.id}-${pn.id}`, source: n.id, target: pn.id,
              animated: true, style: SYNC_EDGE,
              markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
            });
          });
        });

        postNodes.forEach(n => {
          es.push({
            id: `e-${n.id}-cout`, source: n.id, target: 'client-out',
            animated: true, style: SYNC_EDGE,
            markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
          });
          asyncNodes.forEach(an => {
            es.push({
              id: `e-${n.id}-${an.id}`, source: n.id, target: an.id,
              animated: false, style: ASYNC_EDGE,
              markerEnd: { type: MarkerType.ArrowClosed, color: '#fdcb6e' }
            });
          });
        });

        setNodes(ns);
        setEdges(es);
      } catch (err) {
        console.warn('Failed to load pipeline data, using defaults:', err);
      }
    })();
  }, [pipelineId]);

  useEffect(() => {
    const ns: Node[] = [];
    const es: any[] = [];

    ns.push({
      id: 'client-in', type: 'default', position: { x: -60, y: SYNC_Y - 40 },
      data: { label: '🖥️ Client\nRequest', stage: 'client', name: 'Client', type: 'client' },
      style: SPECIAL_NODE_STYLE.client
    });
    ns.push({
      id: 'llm-upstream', type: 'default', position: { x: stageX(1.5), y: SYNC_Y - 100 },
      data: { label: '☁️ Upstream\nLLM API', stage: 'upstream', name: 'Upstream LLM', type: 'gateway' },
      style: SPECIAL_NODE_STYLE.upstream
    });
    ns.push({
      id: 'client-out', type: 'default', position: { x: stageX(3.5), y: SYNC_Y - 40 },
      data: { label: '🖥️ Client\nResponse', stage: 'client', name: 'Client', type: 'client' },
      style: SPECIAL_NODE_STYLE.client
    });

    pluginsByStage.pre_request.forEach((p, i) => {
      ns.push({
        id: `ex-sync-pre-${i}`, type: 'default',
        position: { x: stageX(0.3) + i * 40, y: SYNC_Y + i * 60 },
        data: { label: `${p.icon} ${p.name}`, stage: 'pre_request', name: p.name, type: p.type, plugin: p },
        style: pluginNodeStyle(stages.pre_request.color, stages.pre_request.glow)
      });
    });

    pluginsByStage.streaming.forEach((p, i) => {
      ns.push({
        id: `ex-sync-str-${i}`, type: 'default',
        position: { x: stageX(2) + i * 40, y: SYNC_Y + i * 60 },
        data: { label: `${p.icon} ${p.name}`, stage: 'streaming', name: p.name, type: p.type, plugin: p },
        style: pluginNodeStyle(stages.streaming.color, stages.streaming.glow)
      });
    });

    pluginsByStage.post_response.forEach((p, i) => {
      ns.push({
        id: `ex-sync-post-${i}`, type: 'default',
        position: { x: stageX(2.8), y: SYNC_Y },
        data: { label: `${p.icon} ${p.name}`, stage: 'post_response', name: p.name, type: p.type, plugin: p },
        style: pluginNodeStyle(stages.post_response.color, stages.post_response.glow)
      });
    });

    pluginsByStage.async.forEach((p, i) => {
      ns.push({
        id: `ex-async-${i}`, type: 'default',
        position: { x: stageX(0.8) + i * ASYNC_GAP, y: ASYNC_Y + i * 20 },
        data: { label: `${p.icon} ${p.name}`, stage: 'async', name: p.name, type: p.type, plugin: p },
        style: pluginNodeStyle(stages.async.color, stages.async.glow)
      });
    });

    pluginsByStage.pre_request.forEach((_, i) => {
      es.push({
        id: `e-cin-pre-${i}`, source: 'client-in', target: `ex-sync-pre-${i}`,
        animated: true, style: SYNC_EDGE,
        markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7', width: 20, height: 20 }
      });
    });
    pluginsByStage.pre_request.forEach((_, i) => {
      es.push({
        id: `e-pre-up-${i}`, source: `ex-sync-pre-${i}`, target: 'llm-upstream',
        animated: true, style: SYNC_EDGE,
        markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
      });
    });
    pluginsByStage.streaming.forEach((_, i) => {
      es.push({
        id: `e-up-str-${i}`, source: 'llm-upstream', target: `ex-sync-str-${i}`,
        animated: true, style: SYNC_EDGE,
        markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
      });
    });
    pluginsByStage.streaming.forEach((_, i) => {
      pluginsByStage.post_response.forEach((__, j) => {
        es.push({
          id: `e-str-post-${i}-${j}`, source: `ex-sync-str-${i}`, target: `ex-sync-post-${j}`,
          animated: true, style: SYNC_EDGE,
          markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
        });
      });
    });
    pluginsByStage.post_response.forEach((_, i) => {
      es.push({
        id: `e-post-cout-${i}`, source: `ex-sync-post-${i}`, target: 'client-out',
        animated: true, style: SYNC_EDGE,
        markerEnd: { type: MarkerType.ArrowClosed, color: '#6c5ce7' }
      });
    });

    pluginsByStage.post_response.forEach((_, i) => {
      pluginsByStage.async.forEach((__, j) => {
        es.push({
          id: `e-post-async-${i}-${j}`, source: `ex-sync-post-${i}`, target: `ex-async-${j}`,
          animated: false, style: ASYNC_EDGE,
          markerEnd: { type: MarkerType.ArrowClosed, color: '#fdcb6e' }
        });
      });
    });

    setNodes(ns);
    setEdges(es);
  }, []);

  const stageIndex = (s: string) => ['client', 'pre_request', 'upstream', 'streaming', 'post_response', 'async'].indexOf(s);

  const isValidConnection = useCallback((conn: Connection) => {
    if (!conn.source || !conn.target) return false;
    const sn = nodes.find(n => n.id === conn.source);
    const tn = nodes.find(n => n.id === conn.target);
    if (!sn || !tn) return false;
    const si = stageIndex(sn.data?.stage || '');
    const ti = stageIndex(tn.data?.stage || '');
    return ti === si + 1 || ti === si;
  }, [nodes]);

  const onConnect = useCallback((params: Connection) => {
    if (!isValidConnection(params)) return;
    const async = isAsyncEdge({ source: params.source || '', target: params.target || '' }, nodes);
    setEdges(eds => addEdge({
      ...params,
      animated: !async,
      style: async ? ASYNC_EDGE : SYNC_EDGE,
      markerEnd: { type: MarkerType.ArrowClosed, color: async ? '#fdcb6e' : '#6c5ce7' },
    }, eds));
  }, [isValidConnection, setEdges, nodes]);

  const onDragOver = useCallback((e: React.DragEvent) => { e.preventDefault(); e.dataTransfer.dropEffect = 'move'; setDragOver(true); }, []);
  const onDragLeave = useCallback(() => setDragOver(false), []);

  const onDrop = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    setDragOver(false);
    const data = event.dataTransfer.getData('application/plugin');
    if (!data || !rf) return;
    const plugin = JSON.parse(data);
    const pos = rf.screenToFlowPosition({ x: event.clientX, y: event.clientY });
    const isAsync = plugin.defaultStage === 'async';
    const col = isAsync ? stages.async.color : stages[plugin.defaultStage as keyof typeof stages]?.color || '#6c5ce7';
    const glow = isAsync ? stages.async.glow : stages[plugin.defaultStage as keyof typeof stages]?.glow;
    const newNode: Node = {
      id: `plugin-${Date.now()}`,
      type: 'default',
      position: pos,
      data: { label: `${plugin.icon || '🔌'} ${plugin.name}`, stage: plugin.defaultStage, name: plugin.name, type: plugin.type, plugin },
      style: pluginNodeStyle(col, glow),
    };
    setNodes(nds => [...nds, newNode]);
  }, [rf, setNodes]);

  const handleDrag = (p: any, sId: string) => (e: React.DragEvent) => {
    e.dataTransfer.setData('application/plugin', JSON.stringify({ ...p, defaultStage: sId }));
    e.dataTransfer.effectAllowed = 'move';
  };

  const onNodeClick = useCallback((_: any, n: Node) => setSelected(n), []);
  const onPaneClick = useCallback(() => setSelected(null), []);

  const exportPipelineData = useCallback(() => {
    // 从画布实际节点导出——按阶段分组
    const stageMap: Record<string, any[]> = {};
    nodes.forEach(n => {
      if (n.id === 'client-in' || n.id === 'client-out' || n.id === 'llm-upstream') return;
      const stage = n.data?.stage || n.data?.placement || 'pre_request';
      if (!stageMap[stage]) stageMap[stage] = [];
      stageMap[stage].push({
        name: n.data?.name || n.data?.label?.replace(/^[^\s]+\s/, '') || 'plugin',
        type: n.data?.type || 'native',
        config: n.data?.plugin?.config || n.data?.config || '{}',
        enabled: n.data?.enabled !== false,
      });
    });
    if (Object.keys(stageMap).length === 0) {
      Object.entries(pluginsByStage).forEach(([placement, plugins]) => {
        stageMap[placement] = plugins.map(p => ({ name: p.name, type: p.type, config: '{}', enabled: true }));
      });
    }
    const stagesData = Object.entries(stageMap).map(([placement, plugins]) => ({
      placement, parallel: plugins.length > 1, plugins,
    }));
    return { name: `Pipeline-${pipelineId?.slice(0,8) || Date.now()}`, plan_id: pipelineId || `pipeline-${Date.now()}`, tenant: 'default', description: `${nodes.filter(n => n.id !== 'client-in' && n.id !== 'client-out' && n.id !== 'llm-upstream').length} plugins`, stages: stagesData };
  }, [nodes, pipelineId]);

  useEffect(() => {
    (window as any).__pipelineExport = exportPipelineData;
  }, [exportPipelineData]);

  return (
    <div style={{ display: 'flex', gap: 16, height: '100%', minHeight: 500 }}>
      <motion.div
        initial={{ x: -20, opacity: 0 }}
        animate={{ x: 0, opacity: 1 }}
        style={{
          width: showLibrary ? 260 : 48, flexShrink: 0, transition: 'width 0.3s ease',
          overflow: 'hidden',
        }}
      >
        <GlassCard style={{ height: '100%', padding: showLibrary ? 16 : 12, display: 'flex', flexDirection: 'column' }}>
          <button
            onClick={() => setShowLibrary(!showLibrary)}
            style={{
              background: 'rgba(108,92,231,0.1)', border: '1px solid rgba(108,92,231,0.3)',
              borderRadius: 8, padding: '6px 10px', cursor: 'pointer', display: 'flex',
              alignItems: 'center', gap: 6, color: '#6c5ce7', fontSize: 12, fontWeight: 600,
              marginBottom: showLibrary ? 12 : 0, alignSelf: showLibrary ? 'flex-start' : 'center',
            }}
          >
            {showLibrary ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
            {showLibrary && '组件库'}
          </button>

          {showLibrary && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              style={{ flex: 1, overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: 8 }}
            >
              <div style={{
                display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8,
                padding: '6px 10px', background: 'rgba(108,92,231,0.08)', borderRadius: 10,
              }}>
                <Zap size={14} style={{ color: '#6c5ce7' }} />
                <span style={{ fontSize: 11, fontWeight: 700, color: '#6c5ce7', letterSpacing: '0.05em' }}>同步阶段</span>
              </div>

              {['pre_request', 'streaming', 'post_response'].map((sid) => {
                const sg = stages[sid as keyof typeof stages];
                const pls = pluginsByStage[sid] || [];
                return (
                  <div key={sid} style={{ marginBottom: 8 }}>
                    <div style={{
                      display: 'flex', alignItems: 'center', gap: 6, padding: '4px 8px',
                      fontSize: 10, fontWeight: 700, color: sg.color, letterSpacing: '0.05em',
                      marginBottom: 4,
                    }}>
                      <span>{sg.emoji}</span>
                      <span>{locale === 'zh' ? sg.desc : sg.label}</span>
                    </div>
                    {pls.map(p => (
                      <div
                        key={p.id}
                        draggable
                        onDragStart={handleDrag(p, sid)}
                        style={{
                          padding: '8px 12px', borderRadius: 8, marginBottom: 4, cursor: 'grab',
                          background: 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.02) 100%)',
                          border: `1px solid ${sg.color}30`,
                          borderLeft: `3px solid ${sg.color}`,
                          transition: 'all 0.2s ease',
                        }}
                        onMouseEnter={e => {
                          e.currentTarget.style.background = `linear-gradient(135deg, ${sg.color}15 0%, ${sg.color}08 100%)`;
                          e.currentTarget.style.transform = 'translateX(4px)';
                          e.currentTarget.style.boxShadow = `0 0 20px ${sg.color}30`;
                        }}
                        onMouseLeave={e => {
                          e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.02) 100%)';
                          e.currentTarget.style.transform = 'translateX(0)';
                          e.currentTarget.style.boxShadow = 'none';
                        }}
                      >
                        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                          <GripVertical size={12} color="var(--text-tertiary)" />
                          <span style={{ flex: 1, fontWeight: 600, fontSize: 12, color: 'var(--text-primary)' }}>{p.name}</span>
                          <span style={{
                            fontSize: 9, padding: '2px 6px', borderRadius: 6,
                            background: `${sg.color}18`, color: sg.color, fontWeight: 600,
                          }}>{p.type}</span>
                        </div>
                        <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 2, paddingLeft: 18 }}>{p.description}</p>
                      </div>
                    ))}
                  </div>
                );
              })}

              <div style={{
                display: 'flex', alignItems: 'center', gap: 8, marginTop: 8, marginBottom: 8,
                padding: '6px 10px', background: 'rgba(253,203,110,0.08)', borderRadius: 10,
              }}>
                <Clock size={14} style={{ color: '#fdcb6e' }} />
                <span style={{ fontSize: 11, fontWeight: 700, color: '#fdcb6e', letterSpacing: '0.05em' }}>异步阶段</span>
              </div>

              {pluginsByStage.async.map(p => (
                <div
                  key={p.id}
                  draggable
                  onDragStart={handleDrag(p, 'async')}
                  style={{
                    padding: '8px 12px', borderRadius: 8, marginBottom: 4, cursor: 'grab',
                    background: 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.02) 100%)',
                    border: `1px dashed rgba(253,203,110,0.3)`,
                    borderLeft: `3px solid #fdcb6e`,
                    transition: 'all 0.2s ease',
                  }}
                  onMouseEnter={e => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(253,203,110,0.15) 0%, rgba(253,203,110,0.08 100%)';
                    e.currentTarget.style.transform = 'translateX(4px)';
                    e.currentTarget.style.boxShadow = '0 0 20px rgba(253,203,110,0.3)';
                  }}
                  onMouseLeave={e => {
                    e.currentTarget.style.background = 'linear-gradient(135deg, rgba(255,255,255,0.05) 0%, rgba(255,255,255,0.02) 100%)';
                    e.currentTarget.style.transform = 'translateX(0)';
                    e.currentTarget.style.boxShadow = 'none';
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <GripVertical size={12} color="var(--text-tertiary)" />
                    <span style={{ flex: 1, fontWeight: 600, fontSize: 12, color: 'var(--text-primary)' }}>{p.name}</span>
                    <span style={{
                      fontSize: 9, padding: '2px 6px', borderRadius: 6,
                      background: '#fdcb6e18', color: '#fdcb6e', fontWeight: 600,
                    }}>{p.type}</span>
                  </div>
                  <p style={{ fontSize: 10, color: 'var(--text-tertiary)', marginTop: 2, paddingLeft: 18 }}>{p.description}</p>
                </div>
              ))}
            </motion.div>
          )}
        </GlassCard>
      </motion.div>

      <motion.div
        ref={wrapper}
        initial={{ opacity: 0, scale: 0.98 }}
        animate={{ opacity: 1, scale: 1 }}
        style={{
          flex: 1, position: 'relative', borderRadius: 20, overflow: 'hidden',
          border: dragOver ? '2px solid rgba(108,92,231,0.6)' : '1px solid var(--border-default)',
          background: 'linear-gradient(180deg, var(--bg-secondary) 0%, var(--bg-primary) 100%)',
          transition: 'border 0.2s ease, box-shadow 0.2s ease',
          boxShadow: dragOver ? '0 0 40px rgba(108,92,231,0.3)' : '0 8px 32px rgba(0,0,0,0.3)',
        }}
        onDragOver={onDragOver} onDragLeave={onDragLeave} onDrop={onDrop}
      >
        <ParticleField />

        <ReactFlow
          nodes={nodes} edges={edges}
          onNodesChange={onNodesChange} onEdgesChange={onEdgesChange}
          onConnect={onConnect} isValidConnection={isValidConnection}
          onInit={setRf} onNodeClick={onNodeClick} onPaneClick={onPaneClick}
          fitView minZoom={0.3} maxZoom={2}
          style={{ width: '100%', height: '100%' }}
        >
          <div style={{
            position: 'absolute', left: 16, top: 16, zIndex: 10, pointerEvents: 'none',
            display: 'flex', alignItems: 'center', gap: 8, fontSize: 11, fontWeight: 700,
            background: 'rgba(108,92,231,0.12)', padding: '8px 16px', borderRadius: 24,
            color: '#6c5ce7', border: '1px solid rgba(108,92,231,0.3)',
            boxShadow: '0 0 20px rgba(108,92,231,0.2)',
          }}>
            <Zap size={14} />
            同步治理路径 — 实时阻塞
          </div>

          <div style={{
            position: 'absolute', left: 16, bottom: 60, zIndex: 10, pointerEvents: 'none',
            display: 'flex', alignItems: 'center', gap: 8, fontSize: 11, fontWeight: 700,
            background: 'rgba(253,203,110,0.12)', padding: '8px 16px', borderRadius: 24,
            color: '#fdcb6e', border: '1px solid rgba(253,203,110,0.3)',
            boxShadow: '0 0 20px rgba(253,203,110,0.2)',
          }}>
            <Clock size={14} />
            异步验证路径 — 后台执行
          </div>

          <div style={{
            position: 'absolute', left: 0, right: 0, top: 380, zIndex: 5, pointerEvents: 'none',
          }}>
            <div style={{
              borderTop: '1px dashed rgba(255,255,255,0.1)',
              position: 'relative', margin: '0 24px',
            }}>
              <span style={{
                position: 'absolute', right: 24, top: -10, fontSize: 9, color: 'var(--text-tertiary)',
                background: 'var(--bg-secondary)', padding: '0 8px', letterSpacing: '0.1em',
              }}>SYNC ──── ASYNC</span>
            </div>
          </div>

          {nodes.length === 0 && (
            <div style={{
              position: 'absolute', top: '50%', left: '50%', transform: 'translate(-50%,-50%)',
              color: 'var(--text-tertiary)', fontSize: 15, textAlign: 'center', pointerEvents: 'none',
            }}>
              <motion.div
                animate={{ y: [0, -10, 0] }}
                transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
              >
                <Sparkles size={40} style={{ margin: '0 auto 12px', opacity: 0.3, color: '#6c5ce7' }} />
              </motion.div>
              从左侧拖拽组件开始设计
            </div>
          )}

          <Background
            variant={BackgroundVariant.Dots}
            color="rgba(255,255,255,0.06)"
            gap={24}
            size={1.5}
          />
          <Controls
            showInteractive={false}
            style={{
              bottom: 16, right: 16,
              background: 'rgba(19,22,51,0.9)',
              border: '1px solid rgba(108,92,231,0.3)',
              borderRadius: 12,
              backdropFilter: 'blur(12px)',
            }}
          />
          <MiniMap
            nodeColor={(n) => {
              const map: Record<string, string> = {
                client: '#6c5ce7', upstream: '#f5576c',
                pre_request: '#00d4aa', streaming: '#74b9ff',
                post_response: '#a29bfe', async: '#fdcb6e'
              };
              return map[n.data?.stage] || '#6c5ce7';
            }}
            maskColor="rgba(10,14,39,0.8)"
            style={{
              borderRadius: 12, border: '1px solid var(--border-default)',
              background: 'rgba(19,22,51,0.95)',
            }}
          />
        </ReactFlow>
      </motion.div>

      {selected && (
        <motion.div
          initial={{ x: 20, opacity: 0 }}
          animate={{ x: 0, opacity: 1 }}
          style={{ width: 240, flexShrink: 0 }}
        >
          <GlassCard style={{ padding: 16 }}>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
              <Shield size={16} style={{ color: '#6c5ce7' }} />
              <h3 style={{ fontSize: 14, fontWeight: 700, color: 'var(--text-primary)' }}>组件属性</h3>
            </div>

            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <div>
                <label style={{ fontSize: 10, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4, letterSpacing: '0.05em', textTransform: 'uppercase' }}>组件名称</label>
                <p style={{ fontSize: 14, fontWeight: 600, color: 'var(--text-primary)' }}>{selected.data?.name || selected.data?.label}</p>
              </div>

              <div>
                <label style={{ fontSize: 10, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4, letterSpacing: '0.05em', textTransform: 'uppercase' }}>执行阶段</label>
                <span style={{
                  fontSize: 11, padding: '4px 10px', borderRadius: 8, fontWeight: 600,
                  background: `${stages[selected.data?.stage as keyof typeof stages]?.color || '#6c5ce7'}18`,
                  color: stages[selected.data?.stage as keyof typeof stages]?.color || '#6c5ce7',
                }}>
                  {stages[selected.data?.stage as keyof typeof stages]?.emoji || '🔌'} {stages[selected.data?.stage as keyof typeof stages]?.desc || selected.data?.stage}
                </span>
              </div>

              <div>
                <label style={{ fontSize: 10, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4, letterSpacing: '0.05em', textTransform: 'uppercase' }}>类型</label>
                <span style={{
                  fontSize: 10, padding: '2px 8px', borderRadius: 6,
                  background: 'rgba(108,92,231,0.12)', color: '#6c5ce7', fontWeight: 600,
                }}>
                  {selected.data?.type?.toUpperCase() || 'NATIVE'}
                </span>
              </div>

              {selected.data?.plugin?.description && (
                <div>
                  <label style={{ fontSize: 10, color: 'var(--text-tertiary)', display: 'block', marginBottom: 4, letterSpacing: '0.05em', textTransform: 'uppercase' }}>描述</label>
                  <p style={{ fontSize: 12, color: 'var(--text-secondary)', lineHeight: 1.5 }}>{selected.data.plugin.description}</p>
                </div>
              )}

              <button
                className="btn-secondary"
                style={{ marginTop: 8, width: '100%', justifyContent: 'center', display: 'flex', gap: 6, alignItems: 'center' }}
                onClick={() => setNodes(nds => nds.filter(n => n.id !== selected.id))}
              >
                <Trash2 size={14} /> 移除
              </button>
            </div>
          </GlassCard>
        </motion.div>
      )}
    </div>
  );
}

export default function PipelineDesigner() {
  const { t } = useI18n();
  const { id } = useParams<{ id: string }>();
  const navigate = useNavigate();
  const [saving, setSaving] = useState(false);
  const [publishing, setPublishing] = useState(false);
  const [showTemplateSelector, setShowTemplateSelector] = useState(false);
  const [toast, setToast] = useState<ToastProps>({ message: '', type: 'info', visible: false });
  const [pipelineId] = useState(id || null);

  const showToast = useCallback((message: string, type: ToastProps['type']) => {
    setToast({ message, type, visible: true });
    setTimeout(() => setToast(prev => ({ ...prev, visible: false })), 3000);
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    try {
      const pipelineData = (window as any).__pipelineExport?.();
      if (!pipelineData) throw new Error('No pipeline data');
      if (id) {
        await updatePipeline(id, pipelineData);
        showToast('✅ Pipeline 更新成功', 'success');
      } else {
        await createPipeline(pipelineData);
        showToast('✅ Pipeline 保存成功', 'success');
      }
    } catch (err) {
      showToast('❌ 保存失败: ' + (err instanceof Error ? err.message : 'Unknown error'), 'error');
    } finally {
      setSaving(false);
    }
  }, [id, showToast]);

  const handlePublish = useCallback(async () => {
    setPublishing(true);
    try {
      const pipelineData = (window as any).__pipelineExport?.();
      if (!pipelineData) throw new Error('No pipeline data');
      if (id) {
        await updatePipeline(id, pipelineData);
      } else {
        await createPipeline(pipelineData);
      }
      showToast('🎉 Pipeline 发布成功', 'success');
      setTimeout(() => navigate('/pipelines'), 1500);
    } catch (err) {
      showToast('❌ 发布失败: ' + (err instanceof Error ? err.message : 'Unknown error'), 'error');
    } finally {
      setPublishing(false);
    }
  }, [id, navigate, showToast]);

  return (
    <div style={{ position: 'relative', zIndex: 1 }}>
      <motion.div
        initial={{ y: -20, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 20 }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <motion.button
            className="btn-secondary"
            style={{ padding: '10px 14px' }}
            onClick={() => navigate('/pipelines')}
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
          >
            <ArrowLeft size={18} />
          </motion.button>
          <div>
            <h1 style={{ fontSize: 22, fontWeight: 700, color: 'var(--text-primary)', letterSpacing: '-0.01em' }}>
              {id ? (
                <>编辑流水线 <span style={{ color: '#6c5ce7' }}>#{id.slice(0, 8)}</span></>
              ) : (
                <>✨ 新建流水线</>
              )}
            </h1>
            <p style={{ color: 'var(--text-secondary)', fontSize: 12, marginTop: 4 }}>
              {t('designer.subtitle') || '拖拽组件设计你的 AI 治理流水线'}
            </p>
          </div>
        </div>

        <div style={{ display: 'flex', gap: 10, alignItems: 'center' }}>
          <div style={{
            display: 'flex', alignItems: 'center', gap: 6,
            padding: '6px 14px', borderRadius: 20,
            background: 'rgba(0,212,170,0.08)', border: '1px solid rgba(0,212,170,0.2)',
          }}>
            <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#00d4aa', boxShadow: '0 0 10px #00d4aa' }} />
            <span style={{ fontSize: 12, fontWeight: 600, color: '#00d4aa' }}>Draft</span>
          </div>

          <motion.button
            className="btn-secondary"
            onClick={handleSave}
            disabled={saving}
            whileHover={{ scale: 1.02 }}
            whileTap={{ scale: 0.98 }}
            style={{ display: 'flex', gap: 8, alignItems: 'center' }}
          >
            {saving ? <LoadingSpinner size={16} /> : <Save size={16} />}
            {saving ? '保存中...' : '保存草稿'}
          </motion.button>

          <motion.button
            className="btn-primary"
            onClick={handlePublish}
            disabled={publishing}
            whileHover={{ scale: 1.02, boxShadow: '0 0 30px rgba(108,92,231,0.5)' }}
            whileTap={{ scale: 0.98 }}
            style={{ display: 'flex', gap: 8, alignItems: 'center' }}
          >
            {publishing ? <LoadingSpinner size={16} /> : <Play size={16} />}
            {publishing ? '发布中...' : '发布'}
          </motion.button>
        </div>
      </motion.div>

      <ReactFlowProvider>
        <PipelineCanvas pipelineId={pipelineId} />
      </ReactFlowProvider>

      <Toast {...toast} />

      <style>{`
        @keyframes pulse-glow {
          0%, 100% { box-shadow: 0 0 20px rgba(108,92,231,0.3); }
          50% { box-shadow: 0 0 40px rgba(108,92,231,0.6); }
        }
        .btn-primary {
          animation: pulse-glow 2s ease-in-out infinite;
        }
      `}</style>
    </div>
  );
}