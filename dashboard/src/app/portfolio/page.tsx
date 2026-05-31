import styles from '../page.module.css';

export default function Portfolio() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◊</span> Portfolio</h1>
        <p className={styles.subtitle}>Balance history, equity curve, drawdown chart, daily P&L</p>
      </header>
      <div className={styles.grid}>
        <div className={`card ${styles.cardLarge}`}>
          <div className="card-header">
            <span className="card-title">Equity Curve</span>
          </div>
          <div className={styles.chartPlaceholder}>
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
          </div>
        </div>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Daily P&L</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>◊</span>
            <span>No data yet</span>
          </div>
        </div>
      </div>
    </div>
  );
}
