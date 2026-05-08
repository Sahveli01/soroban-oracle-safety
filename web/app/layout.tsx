import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { LenisProvider } from "@/components/lenis-provider";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "safe-oracle — Trust the oracle. Verify the integrator.",
  description:
    "Drop-in oracle protection for Stellar Soroban. Five mathematically-verified guardrails between your protocol and the next oracle manipulation attack.",
  metadataBase: new URL("https://safe-oracle.vercel.app"),
  openGraph: {
    title: "safe-oracle",
    description: "Trust the oracle. Verify the integrator.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable}`}
    >
      <body>
        <LenisProvider>{children}</LenisProvider>
      </body>
    </html>
  );
}
