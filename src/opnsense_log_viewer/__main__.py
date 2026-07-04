"""
Main entry point for OPNsense Log Viewer application.
Run with: python -m opnsense_log_viewer
"""
import tkinter as tk

from opnsense_log_viewer.components.log_viewer import LogViewerApp


def main():
    """Main entry point"""
    root = tk.Tk()
    app = LogViewerApp(root)
    root.mainloop()


if __name__ == "__main__":
    main()
