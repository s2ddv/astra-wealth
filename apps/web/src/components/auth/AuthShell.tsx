import type { ReactNode } from 'react'
import Link from 'next/link'
import { AstraLogo } from '@/components/astra-logo'
import styles from './Auth.module.css'

export function AuthShell({ children }: { children: ReactNode }) {
  return (
    <div className={styles.shell}>
      <header className={styles.header}>
        <span className={styles.brand} aria-label="Astra Wealth">
          <Link href="/dashboard" className={styles.brandLink} aria-label="Voltar ao dashboard">
            <AstraLogo className={styles.logo ?? ''} aria-hidden="true" />
          </Link>
          <span aria-hidden="true">astra<span className={styles.brandSuffix}>wealth</span></span>
        </span>
      </header>
      <main className={styles.main}>
        <section className={styles.panel} aria-labelledby="signin-title">
          <h1 id="signin-title" className={styles.title}>Entre na Astra</h1>
          <p className={styles.subtitle}>Seu patrimônio, em um só lugar.</p>
          {children}
        </section>
      </main>
    </div>
  )
}
