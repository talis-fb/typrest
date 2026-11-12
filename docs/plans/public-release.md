# Plan: Public Release

- **Status:** Draft
- **Owner:** Talison Fabio

## Objective

Publish Typrest v0.1.0 so people can install and run it without a Rust toolchain. The code and
metadata prerequisites (`--version`, loopback default bind, package metadata, changelog, security
policy, third-party notices) landed in `1ff3ee6`. The remaining prerequisite is merging the delivered
work into `main`.

The first release is deliberately small: **Linux x86_64 only**, as a static binary and as a Docker
image. There is no install script: users download a tarball, check it, and put the binary in their
`PATH` with a few commands they can read.
The repository stays private until the release is built, verified, and the website is ready. Going
public, tagging v0.1.0, and deploying the website happen together in one launch step.

Typrest is a server, so most users will deploy a container or try a binary locally. That sets the
channel order.

## Current behavior

The repository is private, with no tags or releases. Typrest can only be built from source or
through a local `docker build`. `scripts/check-container.sh` already checks the image, including a
clean stop. The website exists in `website/` and is deployed by hand with `wrangler`; see
[`docs/development/website.md`](../development/website.md).

## Intended change

| Channel | Command | When |
|---|---|---|
| Docker (GHCR), the main path | `docker run -p 127.0.0.1:8080:8080 -v "$PWD:/project" ghcr.io/talis-fb/typrest:0.1` | v0.1.0 |
| Manual download, the binary path | Static tarball and SHA-256 from GitHub Releases, installed with the commands below | v0.1.0 |
| Cargo from git | `cargo install --locked --git https://github.com/talis-fb/typrest typrest` | Once the repository is public |
| macOS and aarch64 binaries, arm64 image | Same channels as above | After v0.1.0 |
| Install script | One-line `curl \| sh` installer | Not planned |
| Homebrew tap | `brew install talis-fb/tap/typrest` | After v0.1.0 |
| crates.io | `cargo install typrest` | Later |

The v0.1.0 binary targets `x86_64-unknown-linux-musl`, which is fully static and runs on any Linux
x86_64 distribution. The image targets `linux/amd64`.

The manual install is five commands, documented in Getting Started:

```bash
curl -fLO https://github.com/talis-fb/typrest/releases/download/v0.1.0/typrest-v0.1.0-x86_64-linux.tar.gz
curl -fLO https://github.com/talis-fb/typrest/releases/download/v0.1.0/typrest-v0.1.0-x86_64-linux.tar.gz.sha256
sha256sum -c typrest-v0.1.0-x86_64-linux.tar.gz.sha256
tar -xzf typrest-v0.1.0-x86_64-linux.tar.gz
install -m 755 typrest ~/.local/bin/typrest
```

## Out of scope

- macOS and aarch64 binaries and the arm64 image for v0.1.0. The first release favors a simple,
  stable pipeline on `ubuntu-latest`; the other targets are added afterwards. This also means
  CI macOS coverage can wait.
- Windows binaries for v0.1.0. `crates/typrest/src/shutdown.rs` uses `tokio::signal::unix` with no
  `cfg(unix)` guard, so the crate doesn't compile there. Windows users can use Docker.
- crates.io for v0.1.0. It is a one-way step: published versions can only be yanked, and
  `typrest-core` becomes a public crate. It also requires moving the Vite output inside
  `crates/typrest/`, because `webui.rs` embeds `../../webui/dist/` from outside the crate, and giving
  the `typrest-core` dependency a version. The first publish must be manual, and
  [trusted publishing](https://blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07)
  can replace tokens after that.
- Building the website. It already exists in `website/`; this plan only sets the launch order.

## Steps

### Pipeline

1. Add `.github/workflows/release.yml`, triggered by `v*` tags, with no release tooling beyond
   Cargo and the `gh` CLI. On `ubuntu-latest` it builds `typrest` for `x86_64-unknown-linux-musl`
   (the `vendor-openssl` feature already compiles OpenSSL from source, so only a musl toolchain such
   as `musl-tools` is added), packs `typrest`, `LICENSE` and the third-party notices into
   `typrest-<tag>-x86_64-linux.tar.gz`, writes its `.sha256`, and runs `gh release create`, marking
   `-rc` tags as prereleases. Done when a tag produces a release with the tarball and checksum.
   Before relying on musl, time a render against the gnu build: musl's allocator is slow under heavy
   multithreaded allocation, and the fix is `mimalloc` or `jemalloc` if the difference is noticeable.
2. Add `.github/workflows/docker.yml`, triggered by `v*` tags. Build the `linux/amd64` image on
   `ubuntu-latest`, run `scripts/check-container.sh` before pushing, and push with `GITHUB_TOKEN` and
   a build provenance attestation. Stable tags push `:0.1.0`, `:0.1`, and `:latest`; prerelease tags
   push only their exact version.
3. Add `docs/development/releasing.md`. It holds the release steps, the launch order below, and the
   version policy: the HTTP API, `typrest.toml`, and `schema.yml` are the compatibility surface, and
   a minor bump may break them before 1.0.

### Validating while private

Both workflows run on `ubuntu-latest`, which works on a private repository, so nothing needs to be
public yet.

4. Run `gitleaks detect` over full history. Done when it reports no findings, or each is resolved.
   Earlier regex scans found nothing, but they cover only common token patterns.
5. Review the 10 stale remote branches, including `worktree-bridge-*` and
   `worktree-feat-graceful-shutdown`, and delete those that shouldn't be public.
6. Tag `v0.1.0-rc.1`. The workflow marks it as a prerelease, so it never becomes GitHub's "latest"
   release.
7. Verify the release candidate. Private release assets and GHCR packages need authentication, so
   download with `gh release download` and `docker login ghcr.io`. On a clean machine or container
   without Rust:
   - Follow the five install commands with the downloaded files, then run `typrest --version`,
     `init`, `check`, and `serve`.
   - Confirm `file typrest` reports a statically linked binary and that it runs on an older distro
     image.
   - Render with `curl` and confirm a valid PDF.
   - Confirm `serve` binds to `127.0.0.1` by default.
   - Run the GHCR image and confirm `docker stop` exits with status 0.
   - Verify the SHA-256 and the `/_webui` page in the released binary.
8. Fix anything the release candidate finds and repeat from step 6 with the next `-rc.N`. When it
   passes, delete the rc release, tag, and GHCR image so no stale artifacts become public.

### Launch

Each step needs the maintainer's go-ahead at the time. Step 9 can't easily be taken back once people
fork the repository. Do 9 to 13 in one sitting.

9. Make the repository public, then set the description, topics (`typst`, `pdf`, `pdf-generation`,
   `rust`, `http-server`), and homepage (`https://typrest.talison.dev`).
10. Enable branch protection on `main` and private vulnerability reporting.
11. Tag `v0.1.0` from `main`. Confirm the release assets are public and the GHCR package is public.
12. Check the install commands anonymously, with no `gh` or `docker` login: the five manual install
    commands and `docker run` from GHCR.
13. Launch the website:
    - Update Getting Started to use the published install commands (the five commands above and
      `docker run`) instead of source and `docker build`.
    - Set `SITE_INDEXABLE = true` in `website/src/config.ts` and update the matching wording in
      [`docs/development/website.md`](../development/website.md#search-engines).
    - From a clean checkout of `main`, run `npm ci`, `npm run build`, and `npx wrangler deploy` in
      `website/`.
    - Apply the approved `README.md` change below.

### After v0.1.0

14. Add macOS and aarch64 binaries and the arm64 image: extra matrix entries in `release.yml`,
    native arm and macOS runners (free once the repository is public), and `macos-latest` in the CI
    `rust` job.
15. Create `talis-fb/homebrew-tap`.
16. For Windows: guard `shutdown.rs` with `cfg(unix)`, add a `ctrl_c` fallback, and run CI there.

## Open questions

- Does the maintainer approve this `README.md` wording? The [README policy](../README.md#readme-policy)
  asks for wording first, and it applies only once v0.1.0 exists, because the commands fail before
  then.

  In the Docker quickstart, the build steps become:

  ````markdown
  Prerequisites: Docker (or Podman) and `curl`. The published image serves the bundled
  [`examples/minimal`](examples/minimal/README.md) project from `/project`:

  ```bash
  docker run --rm -p 127.0.0.1:8080:8080 ghcr.io/talis-fb/typrest:0.1
  ```
  ````

  The "serve your own project" command uses the same image name. A new section goes before
  "Quickstart from Source":

  ````markdown
  ## Install

  Linux x86_64 only. Download the binary, check it, and put it in your `PATH`:

  ```bash
  curl -fLO https://github.com/talis-fb/typrest/releases/download/v0.1.0/typrest-v0.1.0-x86_64-linux.tar.gz
  curl -fLO https://github.com/talis-fb/typrest/releases/download/v0.1.0/typrest-v0.1.0-x86_64-linux.tar.gz.sha256
  sha256sum -c typrest-v0.1.0-x86_64-linux.tar.gz.sha256
  tar -xzf typrest-v0.1.0-x86_64-linux.tar.gz
  install -m 755 typrest ~/.local/bin/typrest
  ```

  The binary is statically linked and installs nothing else. Other platforms can use Docker.
  ````

## Documents to update on completion

- `README.md`, with the approved wording
- [`ROADMAP.md`](../../ROADMAP.md)
- [`docs/README.md`](../README.md) and [`docs/development/README.md`](../development/README.md)
- This file: delete it when the work lands, and update the [plans index](README.md).
