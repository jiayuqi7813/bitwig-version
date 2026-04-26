import { useNavigate } from 'react-router-dom';

export function SavePage() {
  const nav = useNavigate();
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">保存版本</h2>
      <input className="input" placeholder="版本备注消息" />
      <div className="border border-slate-700 rounded p-2">diff 摘要将在此显示</div>
      <button className="btn" onClick={() => nav('/project')}>提交快照</button>
    </div>
  );
}
