"""
Progress dialog component for long-running operations.
"""
import tkinter as tk
from tkinter import ttk


class ProgressDialog:
    """Progress dialog for long operations"""

    def __init__(self, parent, title="Loading...", on_cancel=None):
        """
        Initialize progress dialog.

        Args:
            parent: Parent window
            title: Dialog title
            on_cancel: Optional callable invoked when the user cancels
                (typically a CancellationToken.cancel)
        """
        self.parent = parent
        self._on_cancel = on_cancel
        self.dialog = tk.Toplevel(parent)
        self.dialog.title(title)
        self.dialog.geometry("400x100")
        self.dialog.resizable(False, False)
        self.dialog.transient(parent)
        self.dialog.grab_set()

        # Center the dialog
        self.dialog.update_idletasks()
        x = (self.dialog.winfo_screenwidth() // 2) - (400 // 2)
        y = (self.dialog.winfo_screenheight() // 2) - (100 // 2)
        self.dialog.geometry(f"400x100+{x}+{y}")

        self.label = ttk.Label(self.dialog, text="Loading...")
        self.label.pack(pady=10)

        self.progress = ttk.Progressbar(self.dialog, mode='indeterminate')
        self.progress.pack(pady=10, padx=20, fill='x')
        self.progress.start()

        self.cancel_button = ttk.Button(self.dialog, text="Cancel", command=self.cancel)
        self.cancel_button.pack(pady=5)

        self.cancelled = False
        self._determinate = False

    def set_progress(self, fraction):
        """Show real progress on the bar.

        Args:
            fraction: 0-1 completion, or None to revert to the indeterminate
                pulse (progress unknown).
        """
        try:
            if fraction is None:
                if self._determinate:
                    self.progress.config(mode='indeterminate')
                    self.progress.start()
                    self._determinate = False
                return
            if not self._determinate:
                self.progress.stop()
                self.progress.config(mode='determinate', maximum=100)
                self._determinate = True
            self.progress['value'] = max(0.0, min(100.0, fraction * 100))
        except tk.TclError:
            pass  # dialog already destroyed by a Cancel click

    def update_text(self, text):
        """
        Update dialog text.

        Args:
            text: New text to display
        """
        # A worker thread may call this right as the dialog is destroyed by a
        # Cancel click; tolerate the resulting TclError instead of crashing.
        try:
            self.label.config(text=text)
        except tk.TclError:
            pass

    def cancel(self):
        """Cancel the operation"""
        self.cancelled = True
        if self._on_cancel is not None:
            try:
                self._on_cancel()
            except Exception:
                pass
        self.close()

    def close(self):
        """Close the dialog"""
        # Idempotent: on_filter_applied/on_filter_error may call close() after a
        # Cancel already destroyed the dialog.
        try:
            self.progress.stop()
            self.dialog.destroy()
        except tk.TclError:
            pass
