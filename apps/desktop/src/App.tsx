import { Link, Route, Routes } from 'react-router-dom';
import { WelcomePage } from './pages/WelcomePage';
import { BindPage } from './pages/BindPage';
import { RemoteSetupPage } from './pages/RemoteSetupPage';
import { ProjectPage } from './pages/ProjectPage';
import { SavePage } from './pages/SavePage';
import { HistoryPage } from './pages/HistoryPage';
import { ComparePage } from './pages/ComparePage';
import { RestorePage } from './pages/RestorePage';

export function App() {
  return (
    <div className="min-h-screen bg-slate-950 text-slate-100 p-4">
      <header className="mb-4 border-b border-slate-700 pb-3 flex gap-3">
        <Link to="/">Welcome</Link>
        <Link to="/bind">Bind</Link>
        <Link to="/project">Project</Link>
        <Link to="/history">History</Link>
      </header>
      <Routes>
        <Route path="/" element={<WelcomePage />} />
        <Route path="/bind" element={<BindPage />} />
        <Route path="/remote-setup" element={<RemoteSetupPage />} />
        <Route path="/project" element={<ProjectPage />} />
        <Route path="/save" element={<SavePage />} />
        <Route path="/history" element={<HistoryPage />} />
        <Route path="/compare" element={<ComparePage />} />
        <Route path="/restore" element={<RestorePage />} />
      </Routes>
    </div>
  );
}
