import { NextResponse } from 'next/server'
import { createClient } from '@/lib/supabase/server'
import { getAuthRedirect } from '@/lib/auth-navigation'

export async function GET(request: Request) {
  const { searchParams, origin } = new URL(request.url)
  const code = searchParams.get('code')
  const redirectTo = getAuthRedirect(searchParams.get('redirect'))

  if (code) {
    try {
      const supabase = await createClient()
      const { error } = await supabase.auth.exchangeCodeForSession(code)
      if (!error) return NextResponse.redirect(`${origin}${redirectTo}`)
    } catch {
      // Connection failures return to a recoverable sign-in screen.
    }
  }

  const fallback = new URL('/signin', origin)
  fallback.searchParams.set('error', 'auth_callback_failed')
  fallback.searchParams.set('redirect', redirectTo)
  return NextResponse.redirect(fallback)
}
