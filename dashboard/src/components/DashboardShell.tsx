'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import styles from './shell.module.css';

const navItems = [
  { href: '/', label: 'Overview', icon: '◈' },
  { href: '/trades', label: 'Transactions', icon: '◇' },
  { href: '/decisions', label: 'AI Decisions', icon: '◆' },
  { href: '/portfolio', label: 'Portfolio', icon: '◊' },
  { href: '/insight', label: 'Insight', icon: '○' },
  { href: '/knowledge', label: 'Knowledge', icon: '●' },
  { href: '/risk', label: 'Risk', icon: '◐' },
  { href: '/session', label: 'Session', icon: '◑' },
  { href: '/settings', label: 'Settings', icon: '◒' },
];

export default function DashboardShell({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  return (
    <div className={styles.shell}>
      <div className="ambient-bg" />

      {/* Sidebar */}
      <aside className={styles.sidebar}>
        <div className={styles.logo}>
          <span className="accent">SAVANT</span>
          <span className={styles.version}>v0.2.1</span>
        </div>

        <nav className={styles.nav}>
          {navItems.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className={`${styles.navItem} ${pathname === item.href ? styles.active : ''}`}
            >
              <span className={styles.navIcon}>{item.icon}</span>
              <span>{item.label}</span>
            </Link>
          ))}
        </nav>

        <div className={styles.status}>
          <div className={styles.statusDot} />
          <span className="mono">Engine: IDLE</span>
        </div>
      </aside>

      {/* Main Content */}
      <main className={styles.main}>
        {children}
      </main>

      {/* Right Panel */}
      <aside className={styles.rightPanel}>
        <div className={styles.panelSection}>
          <div className="card-header">
            <span className="card-title">Quick Stats</span>
          </div>
          <div className={styles.statGrid}>
            <div className={styles.stat}>
              <span className={styles.statLabel}>Balance</span>
              <span className={`mono ${styles.statValue}`}>$100.00</span>
            </div>
            <div className={styles.stat}>
              <span className={styles.statLabel}>Positions</span>
              <span className={`mono ${styles.statValue}`}>0</span>
            </div>
            <div className={styles.stat}>
              <span className={styles.statLabel}>Today P&L</span>
              <span className={`mono ${styles.statValue} positive`}>$0.00</span>
            </div>
            <div className={styles.stat}>
              <span className={styles.statLabel}>AI Status</span>
              <span className={`mono ${styles.statValue}`}>
                <span className="badge badge-info">IDLE</span>
              </span>
            </div>
          </div>
        </div>

        <div className={styles.panelSection}>
          <div className="card-header">
            <span className="card-title">Market</span>
          </div>
          <div className={styles.marketRow}>
            <span>BTC/USD</span>
            <span className="mono">—</span>
          </div>
          <div className={styles.marketRow}>
            <span>Fear & Greed</span>
            <span className="mono">—</span>
          </div>
          <div className={styles.marketRow}>
            <span>BTC Dominance</span>
            <span className="mono">—</span>
          </div>
        </div>
      </aside>
    </div>
  );
}
