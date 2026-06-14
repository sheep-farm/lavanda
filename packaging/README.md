# AUR packaging (prepared, not yet published)

These PKGBUILDs are kept on the `packaging` branch so `master`/`dev` stay free
of packaging files until lavanda is actually published to the AUR. Everything
here is already updated and verified for **v1.0.1**.

- `PKGBUILD` — builds from source (`lavanda` AUR package)
- `PKGBUILD-bin` — installs the prebuilt release binary (`lavanda-bin` AUR package)

## Verified checksums (v1.0.1)

| Artifact | sha256 |
|---|---|
| source tarball (`v1.0.1.tar.gz`) | `5892c44ac16a8585f11b30b19e379d4f8a72602d0eabf9be1e2deb4f3da510af` |
| binary tarball (release asset) | `3f0122ef7cae961089a599914f87d55c31057950549f8d676f3d8b2977d4b854` |
| `assets/lavanda.desktop` | `2a022becf7cf19460f0ec2a04f483446d0408abd66467f5a719184e8965ab9b1` |
| `LICENSE` | `eb250b5d739669135e041b6e800ea5b09d4c07103985925430c6f6bb93e96db0` |

The binary tarball (`lavanda-1.0.1-x86_64-linux.tar.gz`) is already uploaded as
an asset on the GitHub release for v1.0.1, so `PKGBUILD-bin` works as-is.

## How to publish when ready

```bash
# Source package
git clone ssh://aur@aur.archlinux.org/lavanda.git aur-lavanda
cp packaging/PKGBUILD aur-lavanda/PKGBUILD
cd aur-lavanda
makepkg --printsrcinfo > .SRCINFO
namcap PKGBUILD            # optional lint
makepkg -si               # local test build/install
git add PKGBUILD .SRCINFO && git commit -m "lavanda 1.0.1" && git push

# Binary package — same flow with the lavanda-bin repo and PKGBUILD-bin
```

## Releasing a new version (checklist)

1. Tag and push `vX.Y.Z`; create the GitHub release.
2. Build the release binary **from the tag** (the version is embedded via
   `CARGO_PKG_VERSION`) and upload `lavanda-X.Y.Z-x86_64-linux.tar.gz` as a
   release asset.
3. Recompute the four checksums above and bump `pkgver` in both PKGBUILDs.
4. Regenerate `.SRCINFO` and push to the AUR repos.
