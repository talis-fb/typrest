# Releasing

How a Typrest release is built, verified, and launched. The release plan lives in
[`docs/plans/public-release.md`](../plans/public-release.md) until the first release lands.

## Version policy

Typrest follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html). The compatibility
surface is the [HTTP API](../specs/http-api.md), [`typrest.toml`](../specs/typrest-toml.md), and
[`schema.yml`](../specs/schema-v1.md). Before 1.0, a minor bump may break any of them; record the
break under `Changed` or `Removed` in [`CHANGELOG.md`](../../CHANGELOG.md).

## Artifacts

A `v*` tag triggers two workflows:

| Workflow | Produces |
|---|---|
| [`release.yml`](../../.github/workflows/release.yml) | A GitHub release with `typrest-<tag>-x86_64-linux.tar.gz` (static `x86_64-unknown-linux-musl` binary, `LICENSE`, third-party notices) and its `.sha256` |
| [`docker.yml`](../../.github/workflows/docker.yml) | The `linux/amd64` image at `ghcr.io/talis-fb/typrest`, checked with `scripts/check-container.sh` before the push, with a build provenance attestation |

Stable tags push `:<version>`, `:<major>.<minor>`, and `:latest`. A tag with a hyphen (`v0.1.0-rc.1`) is
marked as a prerelease and pushes only its exact version. A stable tag must point at a commit on
`main`; both workflows fail otherwise. The package version in `Cargo.toml` stays at the
base version (`0.1.0`) for its release candidates, and the workflow checks `typrest --version`
against the tag.

## Cutting a release

1. Move `CHANGELOG.md` entries from `Unreleased` to the new version, and confirm `Cargo.toml`
   carries that version.
2. Tag a release candidate from the branch to verify: `git tag v0.1.0-rc.1 && git push origin v0.1.0-rc.1`.
3. Verify the candidate on a machine without Rust. While the repository is private, download with
   `gh release download` and `docker login ghcr.io`.
   - Run the five manual install commands from the
     [public release plan](../plans/public-release.md#intended-change) (Getting Started carries them
     only after launch) with the downloaded files, then
     `typrest --version`, `init`, `check`, and `serve`.
   - `file typrest` reports a statically linked binary, and it runs on an older distribution image.
   - Time a render against a gnu build: musl's allocator is slow under heavy multithreaded
     allocation, and `mimalloc` or `jemalloc` is the fix if the difference is noticeable.
   - Render with `curl` and confirm a valid PDF; confirm `serve` binds to `127.0.0.1` by default.
   - Run the GHCR image and confirm `docker stop` exits with status 0.
   - Verify the SHA-256 and the `/_webui` page in the released binary.
4. Fix findings and repeat with the next `-rc.N`. When a candidate passes, delete its release, tag,
   and GHCR image.
5. Tag the final version from `main`, confirm the release assets and the GHCR package are public,
   and run the install commands anonymously, with no `gh` or `docker` login.

## First public launch

Do these together, each with the maintainer's go-ahead, because making the repository public cannot
easily be undone:

1. Make the repository public; set the description, topics (`typst`, `pdf`, `pdf-generation`,
   `rust`, `http-server`), and homepage (`https://typrest.talison.dev`).
2. Enable branch protection on `main` and private vulnerability reporting.
3. Tag `v0.1.0` from `main` and verify it as in step 5 above.
4. Launch the website: switch Getting Started to the published install commands, set
   `SITE_INDEXABLE = true` in `website/src/config.ts` (and update the
   [search engine notes](website.md#search-engines)), then run `npm ci`, `npm run build`, and
   `npx wrangler deploy` in `website/` from a clean checkout of `main`.
5. Confirm the `README.md` install command, badges, and site links work anonymously.
