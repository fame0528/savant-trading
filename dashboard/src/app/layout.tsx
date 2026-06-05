import type { Metadata } from "next";
import { Geist_Mono } from "next/font/google";
import "@fortawesome/fontawesome-free/css/all.min.css";
import "./globals.css";

const mono = Geist_Mono({
  variable: "--font-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "SAVANT [Autonomous Trading Agent] [v0.9.0]",
  description: "Autonomous Trading Dashboard",
  icons: {
    icon: "/favicon.ico",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={`${mono.variable} dark h-full`}>
      <body className="h-full antialiased">{children}</body>
    </html>
  );
}
