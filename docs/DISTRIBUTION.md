# Desktop distribution

Meridian is released as a Tauri 2 desktop application. A release is complete
only when a non-technical user can install it, launch it from the operating
system, calculate and save a chart without a network connection, and reopen the
same chart after restarting the application. The credential-free macOS build
also requires the documented one-time Gatekeeper approval.

## Release artifacts

The GitHub release workflow builds on each native operating system and attaches
these user-facing artifacts:

- Windows x86-64: signed per-user NSIS setup executable
- macOS: ad-hoc-signed universal DMG for Apple Silicon and Intel, with no Apple
  Developer account or notarization dependency
- Linux x86-64: OpenPGP-signed RPM for Fedora, RHEL-compatible distributions,
  and openSUSE; DEB for Debian/Ubuntu; and AppImage for portable use, all built
  on Ubuntu 22.04

Do not publish the raw executable as the primary download. It omits the data
resources and platform integration that make Meridian usable as a desktop
application.

## Offline data contract

The installers contain all 102 long-range Swiss Ephemeris files and the three
GeoNames atlas files. `packaging/data.sha256` pins the exact release bytes.
`tools/verify-release-data` verifies every digest and the expected ephemeris
file count before any platform build begins and again on every native runner.

The release workflow downloads the immutable ephemeris revision and the
reviewed GeoNames snapshot once, verifies them, and passes the same artifact to
all platform builds. A changed GeoNames upstream snapshot therefore fails the
release instead of silently producing different Windows, macOS, and Linux
installers. Updating the atlas requires an intentional manifest update.

At runtime, the coefficient and atlas directories are read-only bundled
resources. The SQLite chart archive lives in Tauri's per-user application-data
directory. The supported environment overrides remain available for local
development but are not required by an installed application.

The application itself performs no first-run data download. Windows 10 and 11
normally provide WebView2 as an operating-system component. On a system whose
runtime is missing or older than `110.0.1531.0`, the NSIS installer runs the
visible Microsoft bootstrapper before launch.

## Signing and trust contract

The release workflow refuses to publish the Windows and RPM artifacts unless
the repository has their required signing credentials. The macOS artifact
deliberately has no Apple credential dependency.

Windows repository secrets:

- `WINDOWS_CERTIFICATE`: raw base64 encoding of the Authenticode PFX bytes
- `WINDOWS_CERTIFICATE_PASSWORD`

Linux repository secret:

- `LINUX_GPG_PRIVATE_KEY`: raw base64 encoding of a dedicated, passphrase-free
  RSA OpenPGP private key used only for release-package signing

The Windows runner imports the PFX into its temporary user certificate store
and generates a Tauri configuration override containing its thumbprint. Tauri
signs both the application and installer with SHA-256 and a DigiCert timestamp.
The macOS runner uses Tauri's `-` signing identity to apply the ad-hoc signature
required by Apple Silicon, but it does not submit the application to Apple.
Gatekeeper therefore requires the user to attempt one launch and approve
Meridian under **System Settings → Privacy & Security → Open Anyway**. The Linux
runner signs the RPM after bundling, replaces the unsigned draft asset, and
attaches the corresponding ASCII-armored public key to the release.

Every generated installer and package also receives a GitHub build-provenance
attestation after platform signing is complete.

## Publishing

1. Update the version in `Cargo.toml` and `tauri.conf.json` together.
2. Run `make check` and `make verify-data`.
3. Create and push the matching annotated tag, such as `v0.1.0`.
4. The workflow verifies the tag/version contract, builds a draft release on
   the three native runners, and publishes it only after all builds and
   attestations succeed.
5. Install each artifact on a clean supported system and exercise chart
   creation, city search, persistence, SVG/CSV export, and offline relaunch.

The tag, Cargo package version, Tauri bundle version, and GitHub release version
must be identical. Failed or partial builds remain unpublished.
