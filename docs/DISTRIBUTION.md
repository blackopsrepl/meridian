# Desktop distribution

A Meridian release is complete only when a user can install the native
application, calculate and archive a chart without a network connection, save
an optional `.meridian` file, quit, and reopen both the archive entry and the
file. The credential-free macOS build also requires the documented one-time
Gatekeeper approval.

## Release artifacts

The GitHub tag workflow builds on each native operating system and publishes
all of these artifacts together:

- Linux x86-64: AppImage, DEB, and RPM
- Windows x86-64: per-user NSIS setup executable
- macOS: universal DMG containing both Apple Silicon and Intel code

RPM and DEB are built from the same nFPM manifest and contain the same binary,
desktop entry, MIME registration, icons, and offline resources. The AppImage,
DMG, and Windows installer are built by `cargo-packager`. The raw executable is
not a release download because it lacks the data and operating-system
integration required by the application.

## Offline data contract

Every artifact contains all 102 long-range Swiss Ephemeris files and the three
GeoNames atlas files. `packaging/data.sha256` pins the exact release bytes.
`tools/verify-release-data` checks every digest and the expected coefficient
file count before any native build begins and again on every platform runner.

The workflow downloads and verifies the data once, then passes that identical
artifact to Linux, Windows, and macOS. An upstream atlas change fails the build
instead of silently producing different installers. Updating the atlas
therefore requires an intentional manifest change.

Installed resource locations are read-only:

- DEB/RPM: `/usr/share/meridian/data`
- AppImage: `usr/lib/meridian/data` inside the image
- Windows: `data` beside `meridian.exe`
- macOS: `Meridian.app/Contents/Resources/data`

The SQLite archive is created in the current user's application-data directory
at runtime and is never stored beside the bundled resources. The application
performs no first-run download.

## Signing and first launch

Signing credentials improve operating-system trust but are not required to
assemble a complete release.

When `WINDOWS_CERTIFICATE` and `WINDOWS_CERTIFICATE_PASSWORD` are configured as
repository secrets, the workflow Authenticode-signs both `meridian.exe` and the
NSIS installer with a SHA-256 timestamp. Without them, the same installer is
published unsigned and Windows displays an **Unknown publisher** confirmation.

When `LINUX_GPG_PRIVATE_KEY` contains a base64-encoded OpenPGP private key, nFPM
signs the RPM and the workflow publishes its ASCII-armored public key. Without
that secret, the RPM remains a normal unsigned package. DEB signing is left to
a future APT repository; a standalone GitHub release has no repository metadata
to authenticate.

The macOS build always uses Apple's ad-hoc `-` identity. It requires no Apple
Developer account and is not notarized. On first launch, the user attempts to
open Meridian, then approves it under **System Settings → Privacy & Security →
Open Anyway**. macOS remembers the exception. Administrators of managed Macs
can disable this override.

Every installer and package receives a GitHub build-provenance attestation.

## Workflow gates

The release workflow verifies more than successful compilation:

- the Git tag exactly matches the version in `Cargo.toml`;
- the complete offline data manifest passes on every runner;
- the AppImage contains both resource directories;
- the DEB and RPM metadata can be read and each package contains the ephemeris;
- the macOS DMG mounts, its application passes strict code-signature
  verification, and both resource directories are present;
- a Windows installer is produced and its signature is verified when signing
  credentials are available;
- publication waits for all five required package formats.

Only after every platform build and attestation succeeds does the final job
create the GitHub release.

## Publishing

1. Update the version in `Cargo.toml` and regenerate `Cargo.lock`.
2. Run `make check` and `make verify-data`.
3. Create and push a matching annotated tag, such as `v0.2.0`.
4. Wait for the tag workflow to build, inspect, attest, and publish all five
   artifacts.
5. Install each package on a clean supported system and exercise chart
   creation, city search, archive persistence, `.meridian` file reopening,
   SVG/CSV export, and an offline relaunch.

The tag, Cargo package version, and GitHub release version must be identical.
Failed or partial builds are never published.
