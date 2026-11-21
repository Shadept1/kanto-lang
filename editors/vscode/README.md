# KantoLang VS Code Extension

## Installing the Extension

### From Source (Development)
1. Open this folder (`editors/vscode`) in VS Code
2. Press `F5` to launch the Extension Development Host
3. Open a `.knt` file in the new window

### Packaging as VSIX
```bash
cd editors/vscode
npm install
npm run compile
npx vsce package
```
This creates `kantolang-0.1.0.vsix`. Install with:
```bash
code --install-extension kantolang-0.1.0.vsix
```

## Features

- **Syntax Highlighting** - Keywords, types, strings, numbers, comments, functions, built-ins, operators
- **Code Snippets** - Type `def`, `if`, `for`, `let`, `mut`, `print`, etc. and press Tab
- **Tasks** - "kanto: Run" (Ctrl+F5) and "kanto: Check" via Tasks menu
- **Run Command** - "KantoLang: Run Current File" in command palette, editor title bar, and context menu
- **Auto-save** - Saves file before running
- **Working Directory** - Uses file's directory as working directory
- **Error Detection** - Clear error if `kanto` executable not found

## Language Icons

### Extension Icon (Marketplace)
- **File**: `images/icon.png` (128x128 PNG, required)
- Replace with your actual KantoLang logo

### File Icons (Explorer Tab)
- **Light theme**: `images/file-icon-light.svg`
- **Dark theme**: `images/file-icon-dark.svg`

**Current state**: Placeholder icons with a "K" letter are provided.

**To use your actual logo:**
1. Place your logo files in `images/`:
   - `icon.png` - 128x128 PNG for marketplace
   - `file-icon-light.svg` - SVG for light themes (readable on light background)
   - `file-icon-dark.svg` - SVG for dark themes (readable on dark background)
2. Recompile and repackage the extension

**Note on icon themes:**
- File icons in the explorer depend on the active **File Icon Theme** (e.g., "Seti", "Material Icon Theme", VS Code default)
- Some icon themes override language-specific icons
- Users can enable "File > Preferences > File Icon Theme > KantoLang" if you publish a dedicated icon theme
- The `icon` property in `package.json` (languages section) works with VS Code's built-in icon support, but third-party themes may not respect it

## Requirements

- `kanto` CLI must be installed and in PATH
- Built from the KantoLang repository: `cargo build --release --bin kanto`

## Commands

| Command | Keybinding | Description |
|---------|------------|-------------|
| KantoLang: Run Current File | Ctrl+F5 / Cmd+F5 | Run the active .knt file |
| Tasks: Run Task > kanto: Run | - | Run via task system |
| Tasks: Run Task > kanto: Check | - | Type-check without running |

## Troubleshooting

**"kanto" executable not found:**
- Ensure KantoLang is built: `cargo build --release --bin kanto`
- Add the release binary to your PATH
- Or install globally: `cargo install --path . --bin kanto`

**Syntax highlighting not working:**
- Reload VS Code window (Ctrl+Shift+P > "Developer: Reload Window")
- Ensure file has `.knt` extension

**Tasks not appearing:**
- Open a `.knt` file first
- Run "Tasks: Configure Task" to see available tasks

## License

MIT