import type { Metadata } from "next"
import { GeistSans } from 'geist/font/sans'
import '@/app/globals.css'
import { Providers } from './providers'

export const metadata: Metadata = {
  title: { default: "Astra Wealth", template: "%s | Astra Wealth" },
  description: "Acompanhe seu patrimônio, carteiras e mercados em um só lugar com Astra Wealth.",
  applicationName: "Astra Wealth",
  openGraph: {
    title: "Astra Wealth",
    description: "Seu patrimônio, carteiras e mercados em um só lugar.",
    siteName: "Astra Wealth",
    locale: "pt_BR",
    type: "website",
  },
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="pt-BR" className={GeistSans.className}>
      <body>
        <Providers>{children}</Providers>
      </body>
    </html>
  )
}