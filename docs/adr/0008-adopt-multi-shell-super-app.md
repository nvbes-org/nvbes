# ADR 0008 - Adopt a Multi-Shell Architecture for the Cloud and Account Super App

## Status

Accepted.

## Context

nvbes needs to make the Cloud and Account products available through a
coherent application experience on the Web, Windows, macOS, Linux, iOS,
Android, Apple TV, Android TV, Samsung Tizen TVs and LG webOS TVs.

The products must preserve a native-quality interaction model on each device
family. A desktop interface is driven by a keyboard and pointer, a mobile
interface by touch and platform navigation, and a TV interface by directional
focus and a remote control. Reusing the same rendered pages on every platform
would reduce implementation work but would produce weak mobile and TV
experiences.

The repository already uses React, Vite, TypeScript, generated API clients,
Rust services and an Nx project graph. The client architecture must reuse these
investments without merging the ownership boundaries of Account and Cloud.

The initial product scope includes Cloud and Account only. Console,
Enterprise and Backoffice are not part of this super app decision. Platform
delivery will be progressive: Web and desktop first, mobile second, and TV
clients third.

## Decision

Adopt a multi-shell client architecture. Share contracts, domain behavior and
platform-neutral application capabilities, while implementing presentation
and navigation for each device family.

Use the following platform shells:

| Target | Shell | Presentation runtime |
|---|---|---|
| Web and PWA | Browser | React DOM and Vite |
| Windows, macOS and Linux | Electron | React DOM |
| iOS and Android | Expo | React Native |
| Apple TV and Android TV | React Native TV | React Native TV components |
| Samsung TVs | Tizen Web application | React DOM with a TV-specific UI |
| LG TVs | webOS Web application | React DOM with a TV-specific UI |

### Shared client foundation

Share only platform-neutral capabilities across the shells:

- generated OpenAPI types and API clients;
- Account and Cloud domain models and use cases;
- authentication and authorization policy;
- query keys, cache policy and synchronization state machines;
- telemetry contracts and privacy controls;
- localization resources;
- design tokens and semantic visual primitives;
- platform capability interfaces.

Do not require screens, navigation containers or rendered UI components to be
shared across all targets. Web and desktop may share React DOM components when
their interaction and accessibility contracts are equivalent. Mobile and TV
must use presentations appropriate to their form factors.

Platform-specific code implements explicit capability interfaces for secure
storage, file selection, downloads, uploads, notifications, deep links,
background execution, media playback and operating-system integration. Domain
modules must not import Electron, Expo, React Native TV, Tizen or webOS APIs
directly.

### Product composition

The super app is a composition shell, not a new business domain. It exposes
Account and Cloud modules according to the authenticated principal's grants and
entitlements.

Account retains ownership of profile, preferences, legal choices, privacy and
account lifecycle behavior. Identity remains the OAuth 2.1 Authorization
Server and OpenID Connect Provider defined by ADR 0005. Cloud retains ownership
of files, folders, transfers, shares, search, quotas and Cloud resource policy.

The shell must consume each product through its published contracts. It must
not query product databases, duplicate product source-of-truth state or create
cross-product mutation paths that bypass service APIs.

### Electron desktop architecture

Electron owns the desktop application lifecycle, Chromium renderers, windows,
menus, tray integration, notifications, deep links, updates and other desktop
shell concerns.

Use a sandboxed renderer with context isolation and without Node.js integration.
A narrow, typed preload API is the only renderer entry point to privileged
desktop operations. The renderer must not receive raw access to Electron IPC,
the filesystem, process execution or local credentials.

Run synchronization, resumable transfers, encryption, local indexing,
filesystem watching and other long-running native work in a separately
packaged Rust sidecar. The sidecar is not a Tauri runtime and does not create or
manage a WebView. It reuses platform-neutral nvbes Rust crates through an
explicit binary adapter.

The communication path is:

```text
React renderer -> typed preload API -> Electron main -> authenticated RPC -> Rust sidecar
```

The sidecar protocol must be versioned, bounded and validated. Electron main
owns sidecar startup, readiness, shutdown, restart and crash reporting. A
renderer compromise must not grant unrestricted sidecar access.

### TV interaction model

TV clients provide the complete Cloud product capability set, but they do not
reuse pointer- or touch-oriented layouts. Every action must be reachable with
directional focus and a remote control. Text-heavy operations must support
platform input facilities and may additionally offer a secure handoff to an
authenticated phone or browser without removing the corresponding TV
capability.

Apple TV and Android TV share React Native TV domain bindings and presentation
where their focus and media contracts match. Tizen and webOS use dedicated
React DOM TV applications because their vendor runtimes, packaging, device APIs
and store requirements differ.

## Consequences

The architecture preserves native interaction models while retaining high
reuse of API contracts, domain behavior, security policy and design language.
Electron provides a consistent Chromium runtime on all supported desktop
systems, and the Rust sidecar allows existing Rust capabilities to perform
native and long-running work outside the renderer and Electron main process.

Account and Cloud remain independently owned products. Adding a product to the
super app requires an explicit composition contract rather than moving its
logic into a shared shell.

The project must maintain several presentation implementations, packaging
pipelines and store release processes. A feature is not complete merely because
its Web page exists; each supported shell requires an appropriate interaction
design, accessibility behavior and validation evidence.

Desktop distribution carries the installation size and memory cost of
Electron. The Rust sidecar adds protocol versioning, process lifecycle,
platform packaging, signing and failure-recovery responsibilities.

React Native TV is maintained separately from React Native core. Tizen and
webOS devices ship different browser-engine generations and vendor APIs. TV
compatibility therefore requires a declared device support matrix and testing
on physical representative devices, not only simulators.

The multi-shell architecture deliberately rejects a goal of one reusable page
tree for every platform. Reuse is measured at contract and behavior boundaries,
not by the percentage of identical UI source files.

## Delivery Sequence

Deliver the architecture incrementally:

1. Extract and enforce platform-neutral Account and Cloud client boundaries.
2. Establish the Web composition shell and Electron desktop shell.
3. Introduce the Rust desktop sidecar and its versioned protocol.
4. Deliver the Expo mobile shell for iOS and Android.
5. Deliver React Native TV clients for Apple TV and Android TV.
6. Deliver dedicated Tizen and webOS TV shells.

Each phase must preserve deployable Web clients and the existing Account and
Cloud service boundaries. Later platform phases must not be prerequisites for
shipping fixes to earlier clients.

## Validation

This decision is implemented only when:

- Nx boundaries prevent platform shells from leaking into shared domain and
  contract libraries;
- Account and Cloud remain separate modules backed by their published service
  contracts;
- generated API clients are used instead of duplicated platform-specific HTTP
  calls;
- the Electron renderer is sandboxed, context-isolated and has no Node.js
  integration;
- the preload surface is typed, minimal and covered by contract tests;
- the renderer cannot connect directly to or spawn the Rust sidecar;
- the sidecar protocol is authenticated, versioned and validates every input;
- sidecar crash, restart, upgrade and incompatible-version behavior are tested;
- secure storage, deep links, notifications and background behavior have
  platform-specific integration tests;
- mobile navigation and accessibility are validated on iOS and Android;
- every TV action is reachable by directional focus without a pointer;
- Apple TV, Android TV, Tizen and webOS have explicit physical-device test
  coverage for supported versions;
- Web, desktop, mobile and TV releases can be built and delivered independently.
