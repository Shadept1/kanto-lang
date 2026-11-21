import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import { spawn } from 'child_process';

export function activate(context: vscode.ExtensionContext) {
    console.log('KantoLang extension activated');

    // Register the "Run Current File" command
    const runCurrentFile = vscode.commands.registerCommand('kantolang.runCurrentFile', async () => {
        const editor = vscode.window.activeTextEditor;

        if (!editor) {
            vscode.window.showErrorMessage('No active editor. Open a .knt file to run.');
            return;
        }

        const document = editor.document;

        // Check if it's a KantoLang file
        if (document.languageId !== 'kantolang') {
            vscode.window.showErrorMessage('This command only works with KantoLang (.knt) files.');
            return;
        }

        // Save the file if it's dirty
        if (document.isDirty) {
            try {
                await document.save();
            } catch (err) {
                vscode.window.showErrorMessage(`Failed to save file: ${err}`);
                return;
            }
        }

        // Check if file has a path (not untitled)
        if (!document.fileName || document.isUntitled) {
            const saveOption = await vscode.window.showInformationMessage(
                'The file must be saved before running. Save it now?',
                'Save As...',
                'Cancel'
            );

            if (saveOption === 'Save As...') {
                const uri = await vscode.window.showSaveDialog({
                    defaultUri: vscode.Uri.file('main.knt'),
                    filters: { 'KantoLang': ['knt'] }
                });

                if (uri) {
                    try {
                        await document.saveAs(uri);
                    } catch (err) {
                        vscode.window.showErrorMessage(`Failed to save file: ${err}`);
                        return;
                    }
                } else {
                    return; // User cancelled
                }
            } else {
                return; // User cancelled
            }
        }

        // Verify file exists
        if (!fs.existsSync(document.fileName)) {
            vscode.window.showErrorMessage(`File not found: ${document.fileName}`);
            return;
        }

        const fileDir = path.dirname(document.fileName);
        const fileName = path.basename(document.fileName);

        // Check if kanto is available
        const kantoPath = findKantoExecutable();
        if (!kantoPath) {
            vscode.window.showErrorMessage(
                'KantoLang executable "kanto" not found in PATH. ' +
                'Please install KantoLang and ensure "kanto" is in your PATH. ' +
                'See: https://github.com/kantolang/kantolang'
            );
            return;
        }

        // Run the file using kanto
        const terminal = vscode.window.createTerminal({
            name: `KantoLang: ${fileName}`,
            cwd: fileDir,
            env: process.env
        });

        terminal.show();
        terminal.sendText(`"${kantoPath}" "${fileName}"`);
    });

    context.subscriptions.push(runCurrentFile);
}

function findKantoExecutable(): string | null {
    // On Windows, check for .exe extension
    const isWindows = process.platform === 'win32';
    const executableName = isWindows ? 'kanto.exe' : 'kanto';

    // Check PATH
    const pathEnv = process.env.PATH || process.env.Path || '';
    const paths = pathEnv.split(path.delimiter);

    for (const dir of paths) {
        const fullPath = path.join(dir, executableName);
        if (fs.existsSync(fullPath)) {
            return fullPath;
        }
    }

    // Also check common installation locations
    const commonPaths = [
        path.join(process.env.HOME || process.env.USERPROFILE || '', '.cargo', 'bin', executableName),
        path.join(process.env.LOCALAPPDATA || '', 'Programs', 'kanto', executableName),
    ];

    for (const p of commonPaths) {
        if (fs.existsSync(p)) {
            return p;
        }
    }

    return null;
}

export function deactivate() {}