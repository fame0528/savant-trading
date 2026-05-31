import styles from '../page.module.css';

export default function Settings() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◒</span> Settings</h1>
        <p className={styles.subtitle}>Configuration, insight sources, autonomy level</p>
      </header>
      <div className={styles.grid}>
        <div className="card">
          <div className="card-header">
            <span className="card-title">Engine</span>
          </div>
          <div className={styles.riskGrid}>
            <div className={styles.riskItem}>
              <span>Mode</span>
              <span className="mono">PAPER</span>
            </div>
            <div className={styles.riskItem}>
              <span>Pairs</span>
              <span className="mono">BTC/USD, ETH/USD</span>
            </div>
            <div className={styles.riskItem}>
              <span>Timeframe</span>
              <span className="mono">5m</span>
            </div>
            <div className={styles.riskItem}>
              <span>Starting Balance</span>
              <span className="mono">$100.00</span>
            </div>
          </div>
        </div>
        <div className="card">
          <div className="card-header">
            <span className="card-title">AI Agent</span>
          </div>
          <div className={styles.riskGrid}>
            <div className={styles.riskItem}>
              <span>Provider</span>
              <span className="mono">OpenGateway</span>
            </div>
            <div className={styles.riskItem}>
              <span>Model</span>
              <span className="mono">mimo-v2.5-pro</span>
            </div>
            <div className={styles.riskItem}>
              <span>Autonomy</span>
              <span className="mono">Level 3</span>
            </div>
            <div className={styles.riskItem}>
              <span>Max Decisions/Hour</span>
              <span className="mono">5</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
