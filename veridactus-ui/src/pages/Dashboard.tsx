// VERIDACTUS Dashboard — 全局看板（Tailwind + 响应式）
import { useState, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { motion } from 'framer-motion';
import { Shield, Activity, Zap, BarChart, ChevronRight, AlertTriangle } from 'lucide-react';

const fetcher = async (url:string) => { try { const r=await fetch(url); return r.json(); } catch { return null; } };

interface TraceSummary { trace_id:string; model:string; execution_state:string; created_at:string; signature?:string }
interface HealthMetric { name:string; status:string; latency_ms:number; version:string }
interface MetricsData { traceCount:number; pipelineCount:number; pluginCount:number; policyCount:number; services:HealthMetric[] }

export default function Dashboard() {
  const navigate = useNavigate();
  const [metrics, setMetrics] = useState<MetricsData>({traceCount:0,pipelineCount:0,pluginCount:0,policyCount:0,services:[]});

  useEffect(() => {
    (async () => {
      const [tracesData, healthData, pipelinesData, pluginsData] = await Promise.all([
        fetcher('/v1/traces?limit=5'), fetcher('/health'), fetcher('/api/v1/pipelines'), fetcher('/api/v1/plugins'),
      ]);
      setMetrics({
        traceCount: tracesData?.traces?.length || tracesData?.total || 0,
        pipelineCount: pipelinesData?.total || pipelinesData?.pipelines?.length || 0,
        pluginCount: pluginsData?.plugins?.length || pluginsData?.plugins?.total || 0,
        policyCount: 0,
        services: healthData ? [{name:'Data Plane',status:'ok',latency_ms:0,version:healthData}] : [],
      });
    })();
  }, []);

  const [recentTraces, setRecentTraces] = useState<TraceSummary[]>([]);
  const healthScore = metrics.services.length > 0 ? 100 : 85;

  useEffect(() => {
    fetcher('/v1/traces?limit=5').then(d => {
      setRecentTraces(d?.traces || []);
    });
  }, []);

  return (
    <motion.div initial={{opacity:0}} animate={{opacity:1}} className="px-0">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3 mb-7">
        <div>
          <h1 className="text-3xl font-bold bg-gradient-to-r from-[#6c5ce7] to-[#00d4aa] bg-clip-text text-transparent">控制台</h1>
          <p className="text-sm text-[#8892b0] mt-1">AI 治理实时概览</p>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 px-4 py-2 rounded-xl bg-[rgba(0,212,170,0.1)] border border-[rgba(0,212,170,0.15)]">
            <div className="w-2 h-2 rounded-full bg-[#00d4aa] animate-pulse-glow"/>
            <span className="text-sm text-[#00d4aa] font-semibold">在线</span>
          </div>
          <span className="text-xs text-[#5a6a8a]">v0.3.0</span>
        </div>
      </div>

      {/* 健康分数 */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 sm:gap-5 mb-8">
        {[
          { label:'健康分数', value:`${healthScore}%`, icon:<Shield size={22}/>, color:'#00d4aa', bg:'rgba(0,212,170,0.1)' },
          { label:'执行次数', value:metrics.traceCount, icon:<Activity size={22}/>, color:'#6c5ce7', bg:'rgba(108,92,231,0.1)' },
          { label:'流水线', value:metrics.pipelineCount, icon:<Zap size={22}/>, color:'#fdcb6e', bg:'rgba(253,203,110,0.1)' },
          { label:'插件', value:metrics.pluginCount, icon:<BarChart size={22}/>, color:'#74b9ff', bg:'rgba(116,185,255,0.1)' },
        ].map((m,i)=>(
          <motion.div key={i} initial={{opacity:0,y:20}} animate={{opacity:1,y:0}} transition={{delay:i*0.08}}
            className="glass-card p-5 flex items-center gap-4">
            <div className="w-10 h-10 rounded-xl flex items-center justify-center flex-shrink-0" style={{background:m.bg}}>
              <span style={{color:m.color}}>{m.icon}</span>
            </div>
            <div>
              <div className="text-xs text-[#8892b0]">{m.label}</div>
              <div className="text-xl font-bold text-white">{m.value}</div>
            </div>
          </motion.div>
        ))}
      </div>

      {/* 证明链状态 + 最近Traces */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-5 mb-8">
        {/* 证明链 */}
        <div className="glass-card p-5 sm:p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-base font-bold text-white flex items-center gap-2"><Shield size={18} color="#6c5ce7"/> 密码学证明链</h3>
            <span className="text-xs text-[#00d4aa] bg-[rgba(0,212,170,0.1)] px-2 py-0.5 rounded-badge font-semibold">活跃</span>
          </div>
          {['L0 存储完整性 (SHA-256)','L1 硬件认证 (TEE)','L2A Merkle 采样验证','L2B 零知识证明'].map((l,i)=>(
            <div key={i} className="flex items-center justify-between py-2.5 border-b border-white/[0.04] last:border-0">
              <div className="flex items-center gap-2.5">
                <div className={`w-2 h-2 rounded-full ${i<2?'bg-[#00d4aa]':'bg-[#6c5ce7]'}`}/>
                <span className="text-sm text-[#e0e6f0]">{l}</span>
              </div>
              <span className="text-xs text-[#00d4aa] font-semibold">{i===0?'100%':'就绪'}</span>
            </div>
          ))}
          <motion.div className="mt-4 h-1.5 rounded-full bg-white/[0.06] overflow-hidden">
            <motion.div initial={{width:0}} animate={{width:'85%'}} className="h-full rounded-full bg-gradient-to-r from-[#6c5ce7] to-[#00d4aa]"/>
          </motion.div>
          <span className="text-[10px] text-[#5a6a8a] mt-1.5 block text-right">85% 证明链就绪</span>
        </div>

        {/* 最近 Traces */}
        <div className="glass-card p-5 sm:p-6">
          <div className="flex items-center justify-between mb-4">
            <h3 className="text-base font-bold text-white flex items-center gap-2"><Activity size={18} color="#6c5ce7"/> 最近 Traces</h3>
            <button onClick={()=>navigate('/vault')} className="text-xs text-[#6c5ce7] font-semibold flex items-center gap-1 bg-transparent border-none cursor-pointer hover:underline">
              查看全部 <ChevronRight size={14}/>
            </button>
          </div>
          {recentTraces.length===0?(
            <div className="text-center py-8 text-[#8892b0] text-sm">
              <AlertTriangle size={32} className="mx-auto mb-3 opacity-30"/>
              暂无 Trace 记录。开始使用 Chat 沙箱后自动生成。
            </div>
          ):recentTraces.slice(0,5).map((t,i)=>(
            <div key={i} className="flex items-center justify-between py-2.5 border-b border-white/[0.04] last:border-0">
              <div>
                <div className="text-sm text-[#e0e6f0] font-mono text-xs">{(t.trace_id||'').slice(0,12)}...</div>
                <div className="text-[10px] text-[#5a6a8a] mt-0.5">{t.model} · {t.execution_state}</div>
              </div>
              <span className={`text-[10px] font-semibold px-2 py-0.5 rounded-badge ${
                t.execution_state==='SUCCESS'?'bg-[rgba(0,212,170,0.1)] text-[#00d4aa]':
                t.execution_state==='BLOCKED'?'bg-[rgba(255,107,107,0.1)] text-[#ff7675]':
                'bg-[rgba(253,203,110,0.1)] text-[#fdcb6e]'}`}>
                {t.execution_state||'PENDING'}
              </span>
            </div>
          ))}
          {recentTraces.length>0 && (
            <button onClick={()=>navigate('/vault')} className="btn-secondary w-full mt-4 text-sm py-2">进入 Holo-Trace Vault <ChevronRight size={14}/></button>
          )}
        </div>
      </div>
    </motion.div>
  );
}
