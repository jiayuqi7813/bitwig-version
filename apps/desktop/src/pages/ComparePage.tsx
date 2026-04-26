export function ComparePage() {
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">比较快照</h2>
      <div className="grid grid-cols-2 gap-2">
        <select className="input"><option>快照 A</option></select>
        <select className="input"><option>快照 B</option></select>
      </div>
      <ul className="list-disc list-inside">
        <li>➕ Track 'Drums' was added</li>
      </ul>
      <div className="err">本地和远程版本均已变更，自动合并已禁用。你可以比较两个版本，或将远程版本恢复为副本。</div>
    </div>
  );
}
