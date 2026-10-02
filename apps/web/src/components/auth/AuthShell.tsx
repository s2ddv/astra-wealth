import type { ReactNode } from 'react'
import Link from 'next/link'
import { AstraLogo } from '@/components/astra-logo'
import styles from './Auth.module.css'

type AuthShellProps = {
  children: ReactNode
  title?: string
  subtitle?: string
}

export function AuthShell({ children, title = 'Entre na Astra', subtitle = 'Seu patrimônio, em um só lugar.' }: AuthShellProps) {
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
        <section className={styles.panel} aria-labelledby="auth-title">
          <h1 id="auth-title" className={styles.title}>{title}</h1>
          <p className={styles.subtitle}>{subtitle}</p>
          {children}
        </section>
      </main>
    </div>
  )
}
