import type { Metadata } from 'next'
import { Suspense } from 'react'
import { AuthShell } from '@/components/auth/AuthShell'
import { SignUpForm } from '@/components/auth/SignUpForm'

export const metadata: Metadata = { title: 'Criar conta' }

export default function SignupPage() {
  return (
    <AuthShell title="Crie sua conta na Astra" subtitle="Seu patrimônio começa a se organizar aqui.">
      <Suspense fallback={<p role="status">Carregando cadastro…</p>}>
        <SignUpForm />
      </Suspense>
    </AuthShell>
  )
}
