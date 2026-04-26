export function RestorePage() {
  return (
    <div className="card space-y-3">
      <h2 className="text-xl font-semibold">恢复快照</h2>
      <div>选中快照: snap-1</div>
      <input className="input" placeholder="输出文件夹路径" />
      <div className="err">不会覆盖当前项目文件</div>
      <button className="btn">恢复到此文件夹</button>
    </div>
  );
}
