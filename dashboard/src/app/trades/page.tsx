import styles from '../page.module.css';

export default function Trades() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◇</span> Transaction Log</h1>
        <p className={styles.subtitle}>All trades with entry/exit, PnL, fees, slippage</p>
      </header>
      <div className="card">
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Pair</th>
              <th>Side</th>
              <th>Entry</th>
              <th>Exit</th>
              <th>PnL</th>
              <th>Fees</th>
              <th>Strategy</th>
            </tr>
          </thead>
          <tbody>
            <tr>
              <td colSpan={8} style={{ textAlign: 'center', padding: '40px', color: 'var(--text-dim)' }}>
                No trades yet
              </td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  );
}
