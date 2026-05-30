import styles from './page.module.css';

export default function Overview() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>
          <span className="accent">◈</span> Overview
        </h1>
        <p className={styles.subtitle}>Real-time engine status and performance</p>
      </header>

      <div className={styles.grid}>
        {/* Portfolio Card */}
        <div className={`card ${styles.cardLarge}`}>
          <div className="card-header">
            <span className="card-title">Portfolio Value</span>
            <span className="badge badge-info">PAPER</span>
          </div>
          <div className={styles.portfolioValue}>
            <span className={`mono ${styles.bigNumber}`}>$100.00</span>
            <span className={`mono positive ${styles.change}`}>+$0.00 (0.0%)</span>
          </div>
          <div className={styles.chartPlaceholder}>
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
            <div className={styles.chartLine} />
          </div>
        </div>

        {/* Positions Card */}
        <div className="card">
          <div className="card-header">
            <span className="card-title">Open Positions</span>
            <span className="mono">0</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>◇</span>
            <span>No open positions</span>
          </div>
        </div>

        {/* AI Status Card */}
        <div className="card">
          <div className="card-header">
            <span className="card-title">AI Agent</span>
            <span className="badge badge-warning">IDLE</span>
          </div>
          <div className={styles.aiStats}>
            <div className={styles.aiRow}>
              <span>Model</span>
              <span className="mono">mimo v2.5 pro</span>
            </div>
            <div className={styles.aiRow}>
              <span>Provider</span>
              <span className="mono">OpenGateway</span>
            </div>
            <div className={styles.aiRow}>
              <span>Autonomy</span>
              <span className="mono">Level 3</span>
            </div>
            <div className={styles.aiRow}>
              <span>Decisions Today</span>
              <span className="mono">0</span>
            </div>
          </div>
        </div>

        {/* Insight Card */}
        <div className="card">
          <div className="card-header">
            <span className="card-title">Market Insight</span>
          </div>
          <div className={styles.insightGrid}>
            <div className={styles.insightItem}>
              <span className={styles.insightLabel}>Fear & Greed</span>
              <span className="mono">—</span>
            </div>
            <div className={styles.insightItem}>
              <span className={styles.insightLabel}>BTC Dominance</span>
              <span className="mono">—</span>
            </div>
            <div className={styles.insightItem}>
              <span className={styles.insightLabel}>Funding Rate</span>
              <span className="mono">—</span>
            </div>
            <div className={styles.insightItem}>
              <span className={styles.insightLabel}>RSS Items</span>
              <span className="mono">—</span>
            </div>
          </div>
        </div>

        {/* Recent Trades */}
        <div className={`card ${styles.cardWide}`}>
          <div className="card-header">
            <span className="card-title">Recent Trades</span>
            <span className="mono" style={{ color: 'var(--text-dim)', fontSize: '12px' }}>Last 10</span>
          </div>
          <div className={styles.emptyState}>
            <span className={styles.emptyIcon}>◇</span>
            <span>No trades yet</span>
          </div>
        </div>

        {/* Risk Status */}
        <div className="card">
          <div className="card-header">
            <span className="card-title">Risk Status</span>
            <span className="badge badge-success">OK</span>
          </div>
          <div className={styles.riskGrid}>
            <div className={styles.riskItem}>
              <span>Circuit Breaker</span>
              <span className="badge badge-success">ACTIVE</span>
            </div>
            <div className={styles.riskItem}>
              <span>Daily Loss</span>
              <div className="gauge"><div className="gauge-fill ok" style={{ width: '0%' }} /></div>
            </div>
            <div className={styles.riskItem}>
              <span>Drawdown</span>
              <div className="gauge"><div className="gauge-fill ok" style={{ width: '0%' }} /></div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
