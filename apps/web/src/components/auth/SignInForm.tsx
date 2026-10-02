'use client'

import { useEffect, useRef, useState, type FormEvent } from 'react'
import Link from 'next/link'
import { useRouter, useSearchParams } from 'next/navigation'
import { GoogleIcon } from '@/components/icons/google-icon'
import { createClient } from '@/lib/supabase/client'
import { getAuthRedirect, getSignInError } from '@/lib/auth-navigation'
import styles from './Auth.module.css'

export function SignInForm() {
  const router = useRouter()
  const params = useSearchParams()
  const redirectTo = getAuthRedirect(params.get('redirect'))
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [step, setStep] = useState<'email' | 'password'>('email')
  const [visible, setVisible] = useState(false)
  const [pending, setPending] = useState<'password' | 'google' | null>(null)
  const [emailError, setEmailError] = useState(false)
  const [error, setError] = useState<string | null>(params.has('error')
    ? 'Não foi possível concluir o acesso. Tente entrar novamente.' : null)
  const emailRef = useRef<HTMLInputElement>(null)
  const passwordRef = useRef<HTMLInputElement>(null)
  const busy = pending !== null

  useEffect(() => {
    if (step === 'password') passwordRef.current?.focus()
  }, [step])

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault()
    if (busy) return
    setError(null)
    if (step === 'email') {
      setEmail(email.trim())
      setStep('password')
      return
    }
    setPending('password')
    try {
      const { error: authError } = await createClient().auth.signInWithPassword({ email, password })
      if (authError) {
        setError(getSignInError(authError.code))
        setPending(null)
        return
      }
      router.replace(redirectTo)
      router.refresh()
    } catch {
      setError('Não foi possível conectar. Confira sua conexão e tente novamente.')
      setPending(null)
    }
  }

  async function handleGoogle() {
    if (busy) return
    setError(null)
    setPending('google')
    try {
      const { error: authError } = await createClient().auth.signInWithOAuth({
        provider: 'google',
        options: { redirectTo: `${window.location.origin}/auth/callback?redirect=${encodeURIComponent(redirectTo)}` },
      })
      if (authError) {
        setError(getSignInError(authError.code))
        setPending(null)
      }
    } catch {
      setError('Não foi possível conectar ao Google. Tente novamente ou entre com seu e-mail.')
      setPending(null)
    }
  }

  function editEmail() {
    setStep('email')
    setPassword('')
    setVisible(false)
    setError(null)
    requestAnimationFrame(() => emailRef.current?.focus())
  }

  return (
    <>
      <form onSubmit={handleSubmit} className={styles.form} aria-busy={busy}>
        <div className={styles.fieldGroup}>
          <div className={styles.labelRow}>
            <label htmlFor="signin-email">E-mail</label>
            {step === 'password' && <button type="button" className={styles.textButton} onClick={editEmail} disabled={busy}>Alterar e-mail</button>}
          </div>
          <input ref={emailRef} id="signin-email" name="email" type="email" autoComplete="username" inputMode="email"
            autoCapitalize="none" spellCheck={false} required readOnly={step === 'password'} disabled={busy}
            value={email} placeholder="voce@exemplo.com" className={styles.input}
            aria-invalid={emailError} aria-describedby={emailError ? 'email-error' : undefined}
            onInvalid={() => setEmailError(true)}
            onBlur={(event) => setEmailError(!!email && !event.currentTarget.validity.valid)}
            onChange={(event) => { setEmail(event.target.value); setEmailError(false); setError(null) }} />
          {emailError && <p id="email-error" className={styles.fieldError}>Informe um e-mail válido para continuar.</p>}
        </div>
        {step === 'password' && (
          <div className={styles.fieldGroup}>
            <label htmlFor="signin-password">Senha</label>
            <div className={styles.passwordField}>
              <input ref={passwordRef} id="signin-password" name="password" type={visible ? 'text' : 'password'}
                autoComplete="current-password" required disabled={busy} value={password} placeholder="Digite sua senha"
                className={styles.input} onChange={(event) => { setPassword(event.target.value); setError(null) }} />
              <button type="button" className={styles.reveal} disabled={busy} aria-label={visible ? 'Ocultar senha' : 'Mostrar senha'}
                aria-pressed={visible} onClick={() => setVisible(!visible)}>{visible ? 'Ocultar' : 'Mostrar'}</button>
            </div>
          </div>
        )}
        {error && <p className={styles.error} role="alert">{error}</p>}
        <button type="submit" className={styles.primary} disabled={busy || !email.trim() || (step === 'password' && !password)}>
          {pending === 'password' ? 'Entrando…' : step === 'email' ? 'Continuar' : 'Entrar'}
        </button>
      </form>
      <div className={styles.divider} aria-hidden="true"><span />ou<span /></div>
      <button type="button" onClick={handleGoogle} disabled={busy} className={styles.provider}>
        <span aria-hidden="true"><GoogleIcon className={styles.providerIcon ?? ''} /></span>
        {pending === 'google' ? 'Conectando ao Google…' : 'Entrar com o Google'}
      </button>
      <p className={styles.signup}>Ainda não tem conta?{' '}<Link href={`/signup?redirect=${encodeURIComponent(redirectTo)}`}>Criar conta</Link></p>
      <p role="status" className={styles.srOnly}>{pending === 'password' ? 'Entrando na sua conta.' : pending === 'google' ? 'Redirecionando para o Google.' : ''}</p>
    </>
  )
}
