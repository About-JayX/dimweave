import { Component, type ErrorInfo, type ReactNode } from "react";
import { useBridgeStore } from "@/stores/bridge-store";

let _logId = 1_000_000;

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
}

export class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false };

  static getDerivedStateFromError(): State {
    return { hasError: true };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error("[ErrorBoundary]", error, info.componentStack);
    const message = `[UI] ${error.message}${info.componentStack ? `\n${info.componentStack.slice(0, 300)}` : ""}`;
    useBridgeStore.setState((s) => ({
      terminalLines: [
        ...s.terminalLines.slice(-200),
        {
          id: ++_logId,
          agent: "system",
          kind: "error" as const,
          line: message,
          timestamp: Date.now(),
        },
      ],
    }));
    // Auto-recover on next frame
    requestAnimationFrame(() => this.setState({ hasError: false }));
  }

  render() {
    if (this.state.hasError) {
      return this.props.children;
    }
    return this.props.children;
  }
}
