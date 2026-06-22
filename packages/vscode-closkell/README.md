# Closkell VS Code Extension

Adds `.clsk` language support for Closkell:

- TextMate syntax highlighting for Lisp forms, keywords, type forms, reader prefixes, and `#html` templates.
- Document formatting through `closkell fmt --stdin`.
- Diagnostics through `closkell check --json --stdin`.

The extension looks for the compiler in this order:

1. `closkell.executablePath`, when configured.
2. Fresh `target/release/closkell` or `target/debug/closkell` in a Closkell workspace.
3. `cargo run -q -p cli --` from a Closkell workspace.
4. `closkell` on `PATH`.

No language server is started.
Editor buffers are passed to the compiler through stdin, so the extension does
not create temporary `.clsk` side files in your source directories.

## Development

From the repository root, use the VS Code Run and Debug configuration
`Run Closkell Extension`. The Extension Development Host loads this package
temporarily; it does not install it into your normal VS Code profile.

To install it permanently, package and install a VSIX:

```powershell
cd packages\vscode-closkell
npx @vscode/vsce package --no-dependencies
code --install-extension .\closkell-0.1.0.vsix
```
