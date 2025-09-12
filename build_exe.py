"""
Build script to create Windows executable with PyInstaller
"""
import PyInstaller.__main__
import os
import sys

def build_executable():
    """Build Windows executable as single file"""
    
    # Check if we are in the correct directory
    if not os.path.exists('main_app.py'):
        print("ERROR: main_app.py not found in current directory")
        return False
    
    # Arguments for PyInstaller
    args = [
        'main_app.py',
        '--name=OPNsense_Log_Viewer',
        '--onefile',  # Single executable file (all-in-one)
        '--windowed',  # GUI application (no console)
        '--icon=icon/icon64.ico',  # Use larger icon for better visibility
        '--add-data=icon;icon',  # Include icon folder in the executable
        '--noconfirm',  # Overwrite without asking
        '--clean',  # Clean cache
        '--optimize=2',  # Optimize bytecode
        '--hidden-import=multiprocessing',  # Ensure multiprocessing works
        '--hidden-import=concurrent.futures',  # Ensure futures work
        '--collect-data=tkinter',  # Ensure Tkinter resources are included
    ]
    
    print("Starting executable build...")
    print(f"PyInstaller arguments: {' '.join(args)}")
    
    try:
        # Run PyInstaller
        PyInstaller.__main__.run(args)
        
        # Check if executable was created
        exe_path = os.path.join('dist', 'OPNsense_Log_Viewer.exe')
        if os.path.exists(exe_path):
            print(f"\nSUCCESS: Executable created successfully: {exe_path}")
            print(f"File size: {os.path.getsize(exe_path) / (1024*1024):.1f} MB")
            return True
        else:
            print("\nERROR: Executable was not created")
            return False
            
    except Exception as e:
        print(f"\nERROR during build: {e}")
        return False

if __name__ == "__main__":
    success = build_executable()
    if success:
        print("\nSUCCESS: Build completed successfully!")
        print("Executable is located in the dist/ folder")
    else:
        print("\nFAILED: Build failed!")
        sys.exit(1)
