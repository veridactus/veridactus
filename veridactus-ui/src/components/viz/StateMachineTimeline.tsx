import { motion } from 'framer-motion';
import { CheckCircle, Clock, AlertCircle } from 'lucide-react';

interface StateMachineTimelineProps {
  currentState: string | undefined;
}

const states = [
  { id: 'INIT', label: 'Init', description: 'Request received', color: '#a29bfe' },
  { id: 'CONSTRAINT_EVAL', label: 'Eval', description: 'Constraints evaluated', color: '#74b9ff' },
  { id: 'EXECUTING', label: 'Exec', description: 'Model executing', color: '#fdcb6e' },
  { id: 'VALIDATION', label: 'Valid', description: 'Output validated', color: '#00d4aa' },
  { id: 'FINALIZED', label: 'Final', description: 'Proofs generated', color: '#00d4aa' },
  { id: 'FAILED', label: 'Failed', description: 'Execution failed', color: '#ff7675' },
];

export default function StateMachineTimeline({ currentState }: StateMachineTimelineProps) {
  const currentIndex = states.findIndex(s => s.id === currentState);

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="glass-card"
      style={{ padding: 20 }}
    >
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <div style={{ width: 3, height: 16, background: 'linear-gradient(180deg, #74b9ff 0%, #00d4aa 100%)', borderRadius: 2 }} />
        <h3 style={{ fontSize: 13, fontWeight: 600, color: 'var(--text-secondary)', display: 'flex', alignItems: 'center', gap: 6 }}>
          <Clock size={14} /> Execution Timeline
        </h3>
        <span style={{ marginLeft: 'auto', fontSize: 10, color: currentState === 'FINALIZED' ? '#00d4aa' : currentState === 'FAILED' ? '#ff7675' : '#fdcb6e', fontWeight: 600 }}>
          {currentState || 'N/A'}
        </span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: 4, position: 'relative' }}>
        {/* 连接线 */}
        <motion.div
          initial={{ scaleX: 0 }}
          animate={{ scaleX: 1 }}
          transition={{ duration: 0.8, delay: 0.3 }}
          style={{
            position: 'absolute',
            top: '50%',
            left: 0,
            width: '100%',
            height: 2,
            transform: 'translateY(-50%)',
            background: 'rgba(255,255,255,0.1)',
            zIndex: 0,
          }}
        >
          <motion.div
            initial={{ width: 0 }}
            animate={{ width: currentIndex >= 0 ? `${(currentIndex / (states.length - 1)) * 100}%` : '0%' }}
            transition={{ duration: 0.8, delay: 0.3 }}
            style={{
              height: '100%',
              background: 'linear-gradient(90deg, #6c5ce7 0%, #00d4aa 100%)',
            }}
          />
        </motion.div>

        {/* 状态节点 */}
        {states.map((state, index) => {
          const isActive = currentIndex === index;
          const isPast = index <= currentIndex;
          const isFailed = currentState === 'FAILED';

          return (
            <motion.div
              key={state.id}
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ delay: index * 0.1 }}
              style={{
                flex: 1,
                display: 'flex',
                flexDirection: 'column',
                alignItems: 'center',
                position: 'relative',
                zIndex: 1,
              }}
            >
              <motion.div
                initial={{ scale: 0 }}
                animate={{ scale: isActive ? 1.15 : 1 }}
                transition={{ duration: 0.3, delay: index * 0.1 + 0.2 }}
                style={{
                  width: 36,
                  height: 36,
                  borderRadius: '50%',
                  background: isActive ? state.color : isPast ? `${state.color}30` : 'rgba(255,255,255,0.1)',
                  border: `2px solid ${isActive ? state.color : isPast ? `${state.color}60` : 'rgba(255,255,255,0.2)'}`,
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  boxShadow: isActive ? `0 0 16px ${state.color}40` : 'none',
                  transition: 'all 0.3s ease',
                }}
                whileHover={{ transform: 'scale(1.2)', cursor: 'pointer' }}
              >
                {isActive ? (
                  <motion.div
                    animate={{ rotate: 360 }}
                    transition={{ duration: 2, repeat: Infinity, ease: 'linear' }}
                    style={{ width: 12, height: 12, borderRadius: '50%', background: 'white' }}
                  />
                ) : isPast ? (
                  <CheckCircle size={14} style={{ color: state.color }} />
                ) : (
                  <div style={{ width: 8, height: 8, borderRadius: '50%', background: 'rgba(255,255,255,0.3)' }} />
                )}
              </motion.div>

              <span style={{
                fontSize: 11,
                fontWeight: 600,
                color: isActive ? state.color : isPast ? 'var(--text-secondary)' : 'var(--text-tertiary)',
                marginTop: 8,
                textAlign: 'center',
              }}>
                {state.label}
              </span>

              <span style={{
                fontSize: 9,
                color: 'var(--text-tertiary)',
                marginTop: 2,
                textAlign: 'center',
              }}>
                {state.description}
              </span>
            </motion.div>
          );
        })}
      </div>

      {/* 状态详情 */}
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.8 }}
        style={{ marginTop: 20, padding: 12, borderRadius: 10, background: currentState === 'FAILED' ? 'rgba(255, 118, 117, 0.1)' : 'rgba(0, 212, 170, 0.05)' }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {currentState === 'FAILED' ? (
            <AlertCircle size={16} style={{ color: '#ff7675' }} />
          ) : (
            <CheckCircle size={16} style={{ color: '#00d4aa' }} />
          )}
          <div>
            <span style={{ fontSize: 12, fontWeight: 600, color: currentState === 'FAILED' ? '#ff7675' : '#00d4aa' }}>
              {currentState === 'FINALIZED' ? 'Execution completed successfully' :
               currentState === 'FAILED' ? 'Execution failed' :
               `Currently: ${states.find(s => s.id === currentState)?.description || 'Processing...'}`}
            </span>
          </div>
        </div>
      </motion.div>
    </motion.div>
  );
}
