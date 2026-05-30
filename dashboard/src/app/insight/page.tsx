import styles from '../page.module.css';

export default function Insight() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">○</span> Market Insight</h1>
        <p className={styles.subtitle}>Live Fear & Greed, BTC Dominance, funding rates, RSS feed</p>
      </header>
      <div className={styles.grid}>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Sentiment</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>○</span>
            <span>Start the engine to see live data</span>
          </div>
        </div>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Derivatives</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>○</span>
            <span>No data yet</span>
          </div>
        </div>
        <div className={`card ${styles.cardWide}`}>
          <div className="card-header">
            <span className="card-title">RSS News Feed</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>○</span>
            <span>15 feeds configured — start engine to fetch</span>
          </div>
        </div>
      </div>
    </div>
  );
}
