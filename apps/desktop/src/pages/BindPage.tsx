import { useState } from 'react';
import { useNavigate } from 'react-router-dom';

export function BindPage() {
  const [projectPath, setProjectPath] = useState('');
  const [projectName, setProjectName] = useState('');
  const nav = useNavigate();

  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">绑定 Bitwig 项目</h2>
      <input className="input" value={projectPath} onChange={e => setProjectPath(e.target.value)} placeholder="项目文件夹路径" />
      <input className="input" value={projectName} onChange={e => setProjectName(e.target.value)} placeholder="项目名称" />
      <button className="btn" onClick={() => nav('/remote-setup')}>创建版本库</button>
    </div>
  );
}
