import { Toaster as HotToaster } from 'react-hot-toast';

export function Toaster() {
  return (
    <HotToaster
      position="top-right"
      toastOptions={{
        duration: 4000,
        style: {
          background: 'rgb(var(--bg-primary))',
          color: 'rgb(var(--text-primary))',
          border: '1px solid rgb(var(--border-primary))',
          borderRadius: '8px',
        },
        success: {
          iconTheme: {
            primary: '#16a34a',
            secondary: '#fff',
          },
        },
        error: {
          iconTheme: {
            primary: '#dc2626',
            secondary: '#fff',
          },
        },
      }}
    />
  );
}

// Re-export toast utilities for convenience
export { toast } from 'react-hot-toast';
