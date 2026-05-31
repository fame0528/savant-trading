import styles from '../page.module.css';

export default function Risk() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◐</span> Risk Management</h1>
        <p className={styles.subtitle}>Circuit breaker status, daily loss, drawdown, position limits</p>
      </header>
      <div className={styles.grid}>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Circuit Breakers</span>
            <span className="badge badge-success">ACTIVE</span>
          </div>
          <div className={styles.riskGrid}>
            <div className={styles.riskItem}>
              <span>Daily Loss Limit</span>
              <span className="mono">3%</span>
            </div>
            <div className={styles.riskItem}>
              <span>Max Drawdown</span>
              <span className="mono">10%</span>
            </div>
            <div className={styles.riskItem}>
              <span>Max Positions</span>
              <span className="mono">3</span>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Current Exposure</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>◐</span>
            <span>No exposure</span>
          </div>
        </div>
      </div>
    </div>
  );
}
