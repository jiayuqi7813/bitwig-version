import { Link } from 'react-router-dom';

export function WelcomePage() {
  return (
    <div className="card space-y-3">
      <h1 className="text-2xl font-bold">Bitwig Versions</h1>
      <div>检查 git: ✅ git 2.x.x</div>
      <div>检查 git-lfs: ✅ git-lfs 3.x.x</div>
      <Link className="btn inline-block" to="/bind">绑定 Bitwig 项目</Link>
    </div>
  );
}
