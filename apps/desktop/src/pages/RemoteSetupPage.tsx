import { useNavigate } from 'react-router-dom';

export function RemoteSetupPage() {
  const nav = useNavigate();
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">配置远程仓库（可选）</h2>
      <input className="input" placeholder="git@github.com:user/repo.git" />
      <div className="flex gap-2">
        <button className="btn">设置远程</button>
        <button className="btn">测试拉取</button>
        <button className="btn" onClick={() => nav('/project')}>跳过</button>
      </div>
    </div>
  );
}
