import { useNavigate } from 'react-router-dom';

export function ProjectPage() {
  const nav = useNavigate();
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">当前项目</h2>
      <div>项目名称: Demo</div>
      <div>当前分支: main</div>
      <div>远程地址: (未设置)</div>
      <div>最新待导入文件: demo.dawproject</div>
      <div className="flex flex-wrap gap-2">
        <button className="btn">导入最新 DAWproject</button>
        <button className="btn" onClick={() => nav('/save')}>保存版本</button>
        <button className="btn">推送 (Push)</button>
        <button className="btn">拉取 (Pull)</button>
        <button className="btn">打开导入文件夹</button>
        <button className="btn" onClick={() => nav('/history')}>查看历史</button>
      </div>
    </div>
  );
}
