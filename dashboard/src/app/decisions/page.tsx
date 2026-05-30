import styles from '../page.module.css';

export default function Decisions() {
  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}><span className="accent">◆</span> AI Decisions</h1>
        <p className={styles.subtitle}>Full decision log with reasoning, knowledge sources, confidence</p>
      </header>
      <div className={styles.emptyState}>
        <span className={styles.emptyIcon}>◆</span>
        <span>No decisions recorded yet</span>
      </div>
    </div>
  );
}
