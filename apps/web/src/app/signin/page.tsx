'use client'

import { GoogleIcon } from '@/components/icons/google-icon' 
import { Suspense, useState } from 'react'
import { useRouter, useSearchParams } from 'next/navigation'
import Link from 'next/link'
import { createClient } from '@/lib/supabase/client'

export default function LoginPage() {
  return (
    <Suspense fallback={null}>
      <LoginPageInner />
    </Suspense>
  )
}

function LoginPageInner() {
  const router = useRouter()
  const searchParams = useSearchParams()
  const redirectTo = searchParams.get('redirect') ?? '/dashboard'

  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function handleContinue(e: React.FormEvent) {
    e.preventDefault()

    if (!showPassword) {
      setShowPassword(true)
      return
    }

    setLoading(true)
    setError(null)

    const supabase = createClient()
    const { error } = await supabase.auth.signInWithPassword({ email, password })

    if (error) {
      setError(error.message)
      setLoading(false)
      return
    }

    router.push(redirectTo)
    router.refresh()
  }

  async function handleGoogleLogin() {
    setLoading(true)
    const supabase = createClient()
    const { error } = await supabase.auth.signInWithOAuth({
      provider: 'google',
      options: {
        redirectTo: `${window.location.origin}/auth/callback?redirect=${encodeURIComponent(redirectTo)}`,
      },
    })
    if (error) {
      setError(error.message)
      setLoading(false)
    }
  }

  return (
    <div className="relative min-h-screen bg-white">

      <div className="flex min-h-screen flex-col items-center justify-center px-4">
        {}
        <div className="w-full max-w-[440px]">
          {}
          <div className="mb-10 w-full text-center">
            <h1
              style={{ fontSize: '32px', textAlign: 'center' }}
              className="whitespace-nowrap font-semibold leading-tight text-zinc-900"
            >
              Sign in to Astra
            </h1>
          </div>

          {}
          <form onSubmit={handleContinue} className="space-y-4">
            <div>
              <input
                type="email"
                required
                autoFocus
                disabled={showPassword}
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="Enter your email"
                className="w-full rounded-xl border-2 border-zinc-300 bg-white px-4 py-4 text-[15px] text-zinc-900 placeholder:text-zinc-500 outline-none transition-colors focus:border-[#7c4fd9] disabled:opacity-50"
              />
            </div>

            {showPassword && (
              <div>
                <input
                  type="password"
                  required
                  autoFocus
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  placeholder="Enter your password"
                  className="w-full rounded-xl border border-zinc-300 bg-white px-4 py-4 text-[15px] text-zinc-900 placeholder:text-zinc-500 outline-none transition focus:border-[#241350]"
                />
              </div>
            )}

            {error && <p className="text-sm text-red-500">{error}</p>}

            <button
              type="submit"
              disabled={loading}
              className="w-full rounded-full bg-[#7c4fd9] py-4 text-[15px] font-semibold text-white transition hover:bg-[#32195e] disabled:opacity-50"
            >
              {loading ? 'Signing in...' : 'Continue'}
            </button>
          </form>

          {}
          <div className="my-6 flex items-center gap-3">
            <div className="h-px flex-1 bg-zinc-200" />
            <span className="text-xs font-medium text-zinc-500">OR</span>
            <div className="h-px flex-1 bg-zinc-200" />
          </div>

          <button
            onClick={handleGoogleLogin}
            disabled={loading}
            className="flex w-full items-center justify-center gap-3 rounded-full border border-zinc-300 bg-white py-4 text-[15px] font-medium text-zinc-900 transition hover:bg-zinc-50 disabled:opacity-50 mb-10"
          >
            <GoogleIcon className="h-4 w-4" />
            Sign in with Google
          </button>

          {}
          <p
            className="text-center text-sm normal-case font-normal tracking-normal"
            style={{ color: '#000000', textTransform: 'none' }}
          >
            Don&apos;t have an account?{' '}
            <Link
              href="/signup"
              className="font-medium hover:underline normal-case"
              style={{ color: '#7c4fd9', textTransform: 'none' }}
            >
              Sign up
            </Link>
          </p>
        </div>
      </div>
    </div>
  )
}
