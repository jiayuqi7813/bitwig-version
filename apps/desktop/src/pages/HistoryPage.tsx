import { useNavigate } from 'react-router-dom';

export function HistoryPage() {
  const nav = useNavigate();
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">历史记录</h2>
      <div className="border border-slate-700 rounded p-3 flex justify-between">
        <span>Initial save | You | 2026-04-25 | 3 变更</span>
        <span className="space-x-2">
          <button className="btn" onClick={() => nav('/compare')}>比较</button>
          <button className="btn" onClick={() => nav('/restore')}>恢复</button>
        </span>
      </div>
    </div>
  );
}
