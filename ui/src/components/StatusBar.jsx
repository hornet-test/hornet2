import React from 'react';

export function StatusBar({ message, type }) {
  return (
    <div id="status" className={`status ${type}`} role="status">
      {message}
    </div>
  );
}
