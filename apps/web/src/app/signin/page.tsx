import type { Metadata } from 'next'
import { Suspense } from 'react'
import { AuthShell } from '@/components/auth/AuthShell'
import { SignInForm } from '@/components/auth/SignInForm'

export const metadata: Metadata = { title: 'Entrar' }

export default function SignInPage() {
  return <AuthShell><Suspense fallback={<p role="status">Carregando acesso…</p>}><SignInForm /></Suspense></AuthShell>
}
