import type { StatusMessage } from '../types/graph';

export function StatusBar({ message, type }: StatusMessage) {
  return (
    <div id="status" className={`status ${type}`} role="status">
      {message}
    </div>
  );
}
