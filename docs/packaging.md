# Packaging

AI Ledger is bundled with the Tauri CLI. The output for each OS is:

| OS | Formats |
|---|---|
| macOS | `.app`, `.dmg` (universal: Apple Silicon and Intel) |
| Windows | `.msi` (WiX), `.exe` (NSIS) |
| Linux | `.AppImage`, `.deb`, `.rpm` |

Bundle metadata (publisher, license, category, descriptions) is set in
`src-tauri/tauri.conf.json`.

## Builds

macOS is not built in CI for now (neither PR checks nor releases). A local
`tauri build` on a Mac still works.


- **Every PR:** the `tauri build` CI job builds bundles on all three OSes
  and uploads them as workflow artifacts. They are kept for 7 days.
- **Releases:** pushing a `v*` tag runs `.github/workflows/release.yml`. It
  builds the bundles and attaches them to a **draft** GitHub release, which
  a maintainer publishes by hand.

Local build: `npm ci --prefix ui && ui/node_modules/.bin/tauri build`
(run it from the repo root).

## Builds are unsigned

The repository holds no signing certificates or secrets, so all builds are
**unsigned**:

- **macOS:** Gatekeeper blocks the first launch. Right-click the app, choose
  **Open**, and confirm. Or run
  `xattr -dr com.apple.quarantine "/Applications/AI Ledger.app"`.
- **Windows:** SmartScreen shows "Windows protected your PC". Click
  **More info**, then **Run anyway**.
- **Linux:** no signing is needed. Run `chmod +x` on the AppImage.

## Adding signing later

Signing needs credentials that must live only in GitHub Actions secrets,
never in the repo:

- **macOS:** an Apple Developer ID Application certificate plus
  notarization. Set `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`, `APPLE_ID`, `APPLE_PASSWORD` and
  `APPLE_TEAM_ID` as secrets and pass them as `env` to `tauri-action`.
- **Windows:** an Authenticode code-signing certificate, either imported
  in the job (which sets `bundle.windows.certificateThumbprint`) or used
  through a cloud signing service via `bundle.windows.signCommand`.

See https://v2.tauri.app/distribute/sign/ for the details.
