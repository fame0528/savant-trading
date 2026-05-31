import styles from '../page.module.css';

export default function Knowledge() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">●</span> Knowledge Base</h1>
        <p className={styles.subtitle}>Browse all knowledge units, filter by topic/condition</p>
      </header>
      <div className="card">
        <div className="card-header">
          <span className="card-title">141 Knowledge Units</span>
          <span className="mono" style={{ color: 'var(--text-dim)', fontSize: '12px' }}>11 transcripts</span>
        </div>
        <div className={styles.emptyState}>
          <span className={styles.emptyIcon}>●</span>
          <span>Connect to API to browse knowledge units</span>
        </div>
      </div>
    </div>
  );
}
