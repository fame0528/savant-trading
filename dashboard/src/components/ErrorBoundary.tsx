"use client";

import React from "react";

interface Props {
  label: string;
  children: React.ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends React.Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error(`[ErrorBoundary:${this.props.label}]`, error, info.componentStack);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="flex flex-col items-center justify-center h-full p-4 text-center">
          <i className="fa-solid fa-triangle-exclamation text-[var(--amber)] text-lg mb-2" />
          <p className="text-[var(--dim)] text-xs mb-2">{this.props.label} failed to load</p>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            className="text-[9px] px-2 py-1 rounded bg-[var(--line)] text-[var(--dim)] hover:text-white transition-colors"
          >
            Retry
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
