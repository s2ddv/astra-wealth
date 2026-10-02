/** Authentication returns only to a dashboard route, never an external URL. */
export function getAuthRedirect(value: string | null): string {
  if (!value || /[\\\u0000-\u0020]/.test(value)) return '/dashboard'
  try {
    const url = new URL(value, 'https://astra.invalid')
    if (url.origin !== 'https://astra.invalid' || !value.startsWith('/')) return '/dashboard'
    if (url.pathname !== '/dashboard' && !url.pathname.startsWith('/dashboard/')) return '/dashboard'
    return `${url.pathname}${url.search}${url.hash}`
  } catch {
    return '/dashboard'
  }
}

export function getSignInError(code?: string): string {
  switch (code) {
    case 'invalid_credentials': return 'E-mail ou senha incorretos. Confira os dados e tente novamente.'
    case 'email_not_confirmed': return 'Confirme seu e-mail pelo link que enviamos antes de entrar.'
    case 'over_request_rate_limit':
    case 'over_email_send_rate_limit': return 'Muitas tentativas. Aguarde alguns minutos e tente novamente.'
    default: return 'Não foi possível entrar. Tente novamente em instantes.'
  }
}

export function getSignUpError(code?: string): string {
  switch (code) {
    case 'user_already_exists':
    case 'email_exists': return 'Este e-mail já está cadastrado. Entre na sua conta para continuar.'
    case 'weak_password': return 'Escolha uma senha mais forte, com letras, números e símbolos.'
    case 'email_address_invalid': return 'Informe um e-mail válido para continuar.'
    case 'signup_disabled': return 'O cadastro está indisponível no momento. Tente novamente mais tarde.'
    case 'over_request_rate_limit':
    case 'over_email_send_rate_limit': return 'Muitas tentativas. Aguarde alguns minutos e tente novamente.'
    default: return 'Não foi possível criar sua conta. Tente novamente em instantes.'
  }
}
