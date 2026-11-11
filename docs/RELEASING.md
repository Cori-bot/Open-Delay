# Windows releases and in-app updates

Repository: https://github.com/Cori-bot/Open-Delay. Stable updates are read from
`https://github.com/Cori-bot/Open-Delay/releases/latest/download/latest.json`.

## Signing keys

The Tauri public key is committed in `desktop/tauri.conf.json`. The private key
must remain outside Git and source archives. Keep a separate secure backup:
losing it prevents existing installations from trusting future updates.

Configure GitHub repository Actions secrets:

- `TAURI_SIGNING_PRIVATE_KEY`: the private signing-key file contents.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: its password, empty when the key is unencrypted.

Updater signatures protect the downloaded update package. They are separate from
Windows Authenticode certificates. This pipeline does not apply Authenticode;
Windows can still show an unknown-publisher warning on a first installation.

## Publish a version

1. Update `[workspace.package].version` in `Cargo.toml`, `desktop/tauri.conf.json`,
   `ui/package.json` and the root entries in `ui/package-lock.json`. Refresh
   `Cargo.lock`. The Stream Deck plugin has its own independent version.
2. Commit the verified source. Run the checks before tagging.
3. Push a stable tag matching those versions, for example `v0.2.0`.

`Windows signed release` verifies versions, runs UI/Rust/plugin checks, builds the
NSIS installer using pinned Tauri CLI 2.12.3, and signs updater artifacts. It then
packages the installer, detached signature, source ZIP, Stream Deck plugin,
`latest.json`, `release.json` and `SHA256SUMS.txt`.

Only after all checks and artifacts succeed does it create a draft GitHub
release. All assets are uploaded before publication. Failed uploads leave the
release in draft state; rerunning the workflow can replace assets in that draft.
Published releases are immutable to this workflow: create a new version to fix one.
The manual workflow entry accepts an existing matching tag for retries.

The checks and release jobs pin third-party actions by verified commit. Review
dependency and action updates before changing those references. Signing secrets
are only exposed to the build step and a presence check, never pull-request jobs.

## Build locally

From the repository root, set `TAURI_SIGNING_PRIVATE_KEY` to the path or contents
of your private key and set `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` as appropriate.
Do not put either value in a committed `.env` file or shell transcript.

```powershell
npm --prefix ui ci
npm --prefix integrations/streamdeck ci
npm --prefix integrations/streamdeck run pack
Set-Location desktop
cargo tauri build --bundles nsis -- --locked
Set-Location ..
python scripts/package_release.py --repository Cori-bot/Open-Delay --tag v0.2.0
```

Artifacts are collected in `dist/`. Packaging fails if the native executable
embeds stale UI assets, application versions disagree, or the detached updater
signature is missing or malformed. The packaging script checks the signature
envelope; the Tauri updater performs cryptographic signature verification before
installation. Local research, temporary files, signing keys and agent state are
excluded from the source archive.

## App behavior

The desktop UI checks for a newer stable version through the Rust updater bridge.
Only the update returned by the signed-manifest flow is installed; the webview
cannot supply an arbitrary download URL. Installation requires the user action
and is refused while any streaming output is connected. Downloads show progress
and failures remain visible. The local engine stops before Windows starts the
installer, then the app restarts through the updater.

A full end-to-end replacement test needs two signed versions installed in a
Windows test environment. Unit checks and successful package signing do not by
themselves prove an installed application's upgrade path.
