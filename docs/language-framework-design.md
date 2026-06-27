# Closkell Design Documents

Closkell has three separate design surfaces:

- [Language Design](language-design.md): the framework-neutral language core.
- [Browser Framework Design](web-framework-design.md): the frontend browser
  framework built on the language.
- [Server Framework Design](server-framework-design.md): the backend service
  framework built on the language.

The language core does not contain browser concepts such as `Html`, `#html`,
DOM events, CSS, hydration, or browser commands. Those belong to the browser
framework.

The language core also does not contain server concepts such as HTTP route
registration, request/reply objects, filesystem access, streams, cookies,
process spawning, or server resources. Those belong to the server framework.

Browser apps and backend services are independent programs. They may share pure
Closkell modules, type declarations, validators, encoders, decoders, and
protocol helpers, but there is no implicit fullstack routing, hidden RPC, or
server action mechanism.
