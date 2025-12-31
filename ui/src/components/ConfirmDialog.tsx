import React, { useEffect } from 'react';

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  onConfirm: () => void;
  onCancel: () => void;
  variant?: 'danger' | 'warning' | 'info';
}

export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  isOpen,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  onConfirm,
  onCancel,
  variant = 'info',
}) => {
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        onCancel();
      } else if (event.key === 'Enter') {
        event.preventDefault();
        onConfirm();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onConfirm, onCancel]);

  if (!isOpen) return null;

  const variantColors = {
    danger: {
      iconBg: '#dc3545',
      confirmBg: '#dc3545',
      confirmHover: '#bb2d3b',
    },
    warning: {
      iconBg: '#ffc107',
      confirmBg: '#ffc107',
      confirmHover: '#ffca2c',
    },
    info: {
      iconBg: '#0d6efd',
      confirmBg: '#0d6efd',
      confirmHover: '#0b5ed7',
    },
  };

  const colors = variantColors[variant];

  return (
    <div className="confirm-dialog-overlay" onClick={onCancel}>
      <div className="confirm-dialog" onClick={(e) => e.stopPropagation()}>
        <div className="dialog-header">
          <div className="dialog-icon" style={{ background: colors.iconBg }}>
            {variant === 'danger' && '⚠️'}
            {variant === 'warning' && '⚠️'}
            {variant === 'info' && 'ℹ️'}
          </div>
          <h3 className="dialog-title">{title}</h3>
        </div>

        <div className="dialog-body">
          <p className="dialog-message">{message}</p>
        </div>

        <div className="dialog-footer">
          <button className="dialog-btn cancel-btn" onClick={onCancel}>
            {cancelLabel}
          </button>
          <button
            className="dialog-btn confirm-btn"
            onClick={onConfirm}
            style={{
              background: colors.confirmBg,
              borderColor: colors.confirmBg,
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.background = colors.confirmHover;
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.background = colors.confirmBg;
            }}
          >
            {confirmLabel}
          </button>
        </div>
      </div>

      <style>{getStyles()}</style>
    </div>
  );
};

function getStyles() {
  return `
    .confirm-dialog-overlay {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      background: rgba(0, 0, 0, 0.5);
      display: flex;
      align-items: center;
      justify-content: center;
      z-index: 9999;
      animation: fadeIn 0.2s ease-in-out;
    }

    @keyframes fadeIn {
      from {
        opacity: 0;
      }
      to {
        opacity: 1;
      }
    }

    .confirm-dialog {
      background: white;
      border-radius: 8px;
      box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
      max-width: 450px;
      width: 90%;
      animation: slideIn 0.2s ease-in-out;
    }

    @keyframes slideIn {
      from {
        transform: translateY(-20px);
        opacity: 0;
      }
      to {
        transform: translateY(0);
        opacity: 1;
      }
    }

    .dialog-header {
      display: flex;
      align-items: center;
      gap: 1rem;
      padding: 1.5rem;
      border-bottom: 1px solid #dee2e6;
    }

    .dialog-icon {
      width: 40px;
      height: 40px;
      border-radius: 50%;
      display: flex;
      align-items: center;
      justify-content: center;
      font-size: 1.25rem;
      flex-shrink: 0;
    }

    .dialog-title {
      margin: 0;
      font-size: 1.25rem;
      color: #212529;
      font-weight: 600;
    }

    .dialog-body {
      padding: 1.5rem;
    }

    .dialog-message {
      margin: 0;
      color: #495057;
      line-height: 1.6;
    }

    .dialog-footer {
      display: flex;
      justify-content: flex-end;
      gap: 0.75rem;
      padding: 1rem 1.5rem 1.5rem;
    }

    .dialog-btn {
      padding: 0.5rem 1.25rem;
      border: 1px solid #dee2e6;
      border-radius: 6px;
      font-size: 0.875rem;
      font-weight: 600;
      cursor: pointer;
      transition: all 0.2s;
    }

    .cancel-btn {
      background: #f8f9fa;
      color: #495057;
    }

    .cancel-btn:hover {
      background: #e9ecef;
      border-color: #6c757d;
    }

    .confirm-btn {
      color: white;
    }

    .dialog-btn:focus {
      outline: none;
      box-shadow: 0 0 0 3px rgba(13, 110, 253, 0.1);
    }
  `;
}
