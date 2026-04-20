# StratBrowser (Netscape-style shell)

This project is a modern Linux browser shell that recreates the classic **Netscape Navigator** aesthetic using **GTK4** and **libadwaita**, designed to integrate with the **Ladybird** rendering engine.

The primary StratOS build expects the **full GTK UI browser**. The top-level build script will
attempt to install missing GTK build dependencies automatically on supported distros.

You can still force a local fallback binary for experimentation with:
`-DSTRATBROWSER_REQUIRE_GTK=OFF`

## Features
- **Retro Aesthetic:** Classic "Netscape Grey" UI with 3D-style embossed buttons.
- **Independent Engine:** Architected to use Ladybird's `LibWeb` and `LibWebView`.
- **Iconic Logo:** Includes the classic "N" logo container.
- **Functional Layout:** Toolbar, Location Bar, and Status Bar.

## How to Build and Run

### Prerequisites
You will need the following dependencies installed on your Linux system:
- `cmake`
- `ninja-build`
- `pkg-config`
- `libgtk-4-dev`
- `libadwaita-1-dev`
- `build-essential`

On Ubuntu/Debian, you can install them with:
```bash
sudo apt-get install cmake ninja-build pkg-config libgtk-4-dev libadwaita-1-dev build-essential
```

### Compilation
1. Navigate to the project directory.
2. Create a build folder and compile:
```bash
mkdir build
cd build
cmake ..
make
```

### Running
Launch from the build directory:
```bash
./stratbrowser
```

## Architecture
The UI is built using GTK4 with custom CSS styling to override the modern "Adwaita" look with the retro "Netscape" look. The `BrowserWindow` class manages the main window structure, while the content area is prepared to host a Ladybird `WebContentView` widget.
