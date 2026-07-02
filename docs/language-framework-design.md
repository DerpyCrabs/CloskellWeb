# Closkell Documentation Map

Closkell is implemented as a small language core plus compiler targets.

- [Language core](language-design.md): parser, macros, type checking, purity
  validation, inspection, tests, and JavaScript ESM emission.
- [Browser target](web-framework-design.md): `#html`, `Html`, `Cmd`, `Sub`,
  browser command handlers, Vite app wrapping, and no-VDOM DOM updates.
- [Server target](server-framework-design.md): server-target type checking,
  app wrapping, and typed route, response, resource, and boot helpers.

Target selection:

- default: browser target,
- `--target core`: core language only; browser and server framework forms are
  rejected,
- `--target server`: server helper types and emit rules are enabled.

Browser apps and server programs are independent modules. They can share pure
Closkell modules, type declarations, validators, encoders, decoders, and
protocol helpers. There is no hidden RPC layer, implicit fullstack routing, or
server action generation.
