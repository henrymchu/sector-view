import { useEffect } from "react";
import "./Toast.css";

export interface ToastMessage {
  id: number;
  text: string;
  type: "success" | "error";
}

interface ToastProps {
  toasts: ToastMessage[];
  onDismiss: (id: number) => void;
}

function Toast({ toasts, onDismiss }: ToastProps) {
  return (
    <div className="toast-container" aria-live="polite">
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ToastItem({ toast, onDismiss }: { toast: ToastMessage; onDismiss: (id: number) => void }) {
  useEffect(() => {
    const timer = setTimeout(() => onDismiss(toast.id), 3000);
    return () => clearTimeout(timer);
  }, [toast.id, onDismiss]);

  return (
    <div className={`toast toast-${toast.type}`} role="status">
      <span className="toast-text">{toast.text}</span>
      <button
        className="toast-close"
        onClick={() => onDismiss(toast.id)}
        aria-label="Dismiss notification"
      >
        &times;
      </button>
    </div>
  );
}

export default Toast;
