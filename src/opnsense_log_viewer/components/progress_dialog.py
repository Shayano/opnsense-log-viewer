"""
Progress dialog component for long-running operations.
"""
import tkinter as tk
from tkinter import ttk


class ProgressDialog:
    """Progress dialog for long operations"""

    def __init__(self, parent, title="Loading..."):
        """
        Initialize progress dialog.

        Args:
            parent: Parent window
            title: Dialog title
        """
        self.parent = parent
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

    def update_text(self, text):
        """
        Update dialog text.

        Args:
            text: New text to display
        """
        self.label.config(text=text)

    def cancel(self):
        """Cancel the operation"""
        self.cancelled = True
        self.close()

    def close(self):
        """Close the dialog"""
        self.progress.stop()
        self.dialog.destroy()
