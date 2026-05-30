import styles from '../page.module.css';

export default function Session() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◑</span> Session Report</h1>
        <p className={styles.subtitle}>Real-time session log — all decisions, trades, insight, positions</p>
      </header>
      <div className="card">
        <div className="card-header">
          <span className="card-title">Live Events</span>
          <span className="badge badge-warning">IDLE</span>
        </div>
        <div className={styles.emptyState}>
          <span className={styles.emptyIcon}>◑</span>
          <span>Start the engine to see live session events</span>
        </div>
      </div>
    </div>
  );
}
