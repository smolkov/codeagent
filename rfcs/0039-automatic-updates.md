# Automatic updates

- RFC: 0039
- Status: draft
- Milestone: none
- Spec: [`spec/config.md`](../spec/config.md), new "Update";
  [`spec/command.md`](../spec/command.md), new `codeagent update`;
  [`spec/sessions.md`](../spec/sessions.md), "Server";
  [`spec/architecture.md`](../spec/architecture.md), "Protocol" (notice);
  [`spec/security.md`](../spec/security.md), "Entry points and guards",
  "Accepted risks", "Checklist for a new feature"; RFC 0020, answer 5
  (signing)

## Summary

The background server looks once a day for a newer release of `codeagent`
and tells the clients when one is out. How far it goes is set in the user
config:

```toml
[update]
auto = "check"   # "off" | "check" | "install"
```

- `off`: no request is made.
- `check` (default): a notice names the new version and the command to
  update for this kind of install.
- `install`: for a binary from a release archive, the server also
  downloads the new one, checks its signature and puts it in place of the
  old one; it is used from the next server start.

`codeagent update` checks and installs at once, whatever the setting.

## Motivation

- `codeagent` is installed from a release archive, a Debian package or
  `cargo install`. None of them updates on its own, so users stay on old
  versions and miss fixes, also security fixes (see `spec/security.md`,
  "Dependency advisories": a fix is a patch release, which helps only
  those who install it).
- The user finds out about a release only by watching the forge.
- The server is long-lived and already does periodic work (the
  configuration poll), so a daily check fits there and costs no start time
  of a client.

## Scope

In:

- The `[update]` key, its values and default.
- The daily check in the server, its state file and the notice.
- Knowing the kind of install and the update command for each.
- Download, checks and replacing the binary for an archive install.
- Signing the release's `SHA256SUMS` (a change of RFC 0020).
- `codeagent update [--check]`.
- A newer client replacing an old server that has no working session.

Out:

- Replacing a binary owned by `dpkg` or installed by `cargo`: those get
  the notice only.
- Pre-releases, channels, pinning a version.
- Updating a binary the user cannot write (e.g. `/usr/local/bin` owned
  by root): notice only.
- Windows (not a target, RFC 0020).
- Updating language servers, skills or anything else than `codeagent`.

## Design

### Settings

```toml
[update]
auto = "check"
```

| Value | Request | Notice | Replaces the binary |
|-------|---------|--------|---------------------|
| `off` | none | none | no |
| `check` | once a day | yes | no |
| `install` | once a day | yes | archive install only |

- Default `check`. The request is one HTTPS `GET` to GitHub per day,
  with `User-Agent: codeagent/<version>` and nothing else about the user;
  `readme.md` says so next to the setting.
- User file only: a project file with `[update]` is an error naming the
  key, like `[session]`. A change applies at once, read with the next
  configuration poll.
- `CODEAGENT_UPDATE=off|check|install` overrides the file, for CI and
  packaged setups. It is read in `startup.rs` with the other variables
  (RFC 0026). **(proposal)**
- `codeagent run` and `codeagent commit` without a server never check: only
  the server does, so a script or a pipeline makes no request unless it
  starts the server.

### The check

- The server checks 60 s after it starts (so a short-lived server of one
  command does not) and then every 24 h while it runs. **(proposal)**
- The state file `$XDG_STATE_HOME/codeagent/update.json` (mode `0600`)
  keeps the time of the last check, the latest version found and the
  last error. A check is skipped when the last one is less than 24 h old,
  so many short server runs make one request a day.
- The request is
  `GET https://api.github.com/repos/smolkov/codeagent/releases/latest`
  with the client of the providers (`reqwest`, `rustls`, proxy from the
  environment, no redirects followed except to `github.com` and
  `objects.githubusercontent.com` for the download). Timeout 10 s.
- `latest` never returns a pre-release. The tag is `v` plus a semantic
  version; a version that is not newer than the running one, or that
  does not parse, is no update.
- A failure (no network, rate limit, bad answer) is logged at `info` and
  kept in the state file; it shows no notice. The next try is after 24 h,
  or at once with `codeagent update`.

### Kind of install

The server decides from `std::env::current_exe()` (links resolved):

| Kind | How it is known | Update command in the notice |
|------|-----------------|------------------------------|
| Debian package | `/var/lib/dpkg/info/codeagent.list` names the path | `download codeagent_X.Y.Z-1_ARCH.deb, then sudo apt install ./codeagent_X.Y.Z-1_ARCH.deb` |
| `cargo install` | The path is `$CARGO_HOME/bin/codeagent` (default `~/.cargo/bin`) | `cargo install codeagent --locked` |
| Release archive | Anything else, and the directory is writable by the user | `codeagent update` |
| Not writable | Anything else | the release page |

A development build (`cargo run`, the path under `target/`) never checks:
`auto` is treated as `off`. **(proposal)**

### Notice

A new server-wide notice `update_available` with the version, the release
page and the command; protocol version up by one (`12`, or the next free
one when another RFC takes `12` first). Clients show it once per version:

```
codeagent 1.1.0 is available (installed 1.0.1): cargo install codeagent --locked
```

- The TUI shows it in the conversation of the attached session, like
  `config_reloaded`; `codeagent chat` prints it as a notice line;
  `codeagent ls` and other one-shot commands do not print it.
- "Once per version" is kept in the state file, so a version the user saw
  is not shown again in every new TUI.
- After an install, the notice is
  `codeagent updated to 1.1.0 (was 1.0.1); used from the next server start`.
- A failed install: `codeagent 1.1.0 is available; the update failed: <reason>`
  and the command for a manual update. Not repeated for the same version.

### Install

Only with `auto = "install"` or `codeagent update`, and only for a release
archive install:

1. Download `codeagent-X.Y.Z-<target>.tar.gz`, `SHA256SUMS` and
   `SHA256SUMS.minisig` of the release into
   `$XDG_STATE_HOME/codeagent/update/` (mode `0700`). The target is the one
   the running binary was built for (`env!("TARGET")` through
   `ci/build.sh`, as the commit is). At most 64 MiB per file.
2. Check `SHA256SUMS.minisig` against the public key built into the binary
   (below). A bad or missing signature stops the install.
3. Check the archive's SHA-256 against its line in `SHA256SUMS`.
4. Unpack only `codeagent-X.Y.Z-<target>/codeagent` from the archive: a
   regular file, at most 64 MiB; any other entry is skipped, links and
   absolute paths are refused.
5. Run `<new binary> --version` with a timeout of 5 s and an empty
   environment; it must print `codeagent X.Y.Z`. This catches a binary for
   the wrong platform.
6. In the directory of the running binary: write the new one as
   `.codeagent.new` (mode `0755`), copy the old one to
   `codeagent.previous`, then `rename` the new one over `codeagent`. The
   rename is atomic: a client that starts during it gets the old or the
   new binary, never half of one.
7. Remove the download directory.

- The running server keeps the old binary (its file stays open) and
  works on unchanged. Running TUIs and `codeagent chat` too.
- Any failure leaves the old binary in place, removes the temporary
  files and shows the failed notice; it never ends the server or a
  session (`AGENTS.md`, "Never crash").
- Only one install runs at a time (a lock file in the download
  directory), also across a server and `codeagent update`.

### The new binary takes over

A server exits after 60 s without clients and without a working session
(`spec/sessions.md`, "Server"); the next client starts the new binary,
which resumes the saved sessions. Nothing more is needed in most cases.

When a new client meets an old server with another protocol version,
today it fails with "restart the server (`codeagent server stop`)". With
this RFC: **(proposal)**

- If the server's `hello` shows another version and the server has no
  session `working` or `waiting`, the client sends `stop`, waits for the
  server to exit (5 s) and starts a new one, with the line
  `restarted the server (1.0.1 → 1.1.0)`. The `stop` keeps sessions, as
  today.
- Otherwise the error stays, and it now names the working sessions.
- For that the server answers `stop` and a small `status` even on a
  version mismatch, before it closes the connection. These two messages
  are kept unchanged across versions from now on.

### Signing

RFC 0020 chose provenance attestations and no signing key (answer 5).
Provenance is checked with `gh attestation verify`, which needs `gh` and
sigstore; it is too heavy to build into `codeagent`. A checksum from the
same release proves nothing when the account or the release is
changed. So for updates the release gets a signature: **(proposal)**

- A `minisign` key pair (Ed25519). The secret key is a secret of the
  `release` environment on GitHub and a protected, masked variable on
  GitLab; the public key is in `ci/minisign.pub` and built into the
  binary with `include_str!`.
- `ci/checksums.sh` also writes `SHA256SUMS.minisig`; the release uploads
  it next to `SHA256SUMS`.
- The binary checks with the crate `minisign-verify` (no dependencies).
- `readme.md` shows `minisign -Vm SHA256SUMS -p ci/minisign.pub` for a
  manual install.
- A key change: the old binary accepts only the old key, so a new key is
  first shipped in a release signed with the old one. A lost key means a
  manual update for everyone, said in the release notes.

### codeagent update

```
codeagent update            check now and install if possible
codeagent update --check    check now, never install
```

- Works without a server and with `auto = "off"`; prints what it found
  and did, and the command for kinds it cannot update.
- Exit codes: `0` up to date or updated; `1` the check or install failed;
  `3` an update exists that this kind of install does not allow (with
  `--check`, an update exists). **(proposal)**
- `--json` prints `{"installed":"1.0.1","latest":"1.1.0","kind":"archive","action":"updated"}`.
- After an update by hand, a running server keeps the old version until it
  stops; `codeagent update` says so and suggests `codeagent server stop`
  when no session works.

### Modules

| Module | Change |
|--------|--------|
| `src/update/mod.rs` (new) | Settings, state file, the check, kind of install. |
| `src/update/install.rs` (new) | Download, signature, checksum, unpack, `--version`, replace. |
| `src/config/` | `[update] auto`; project-file check. |
| `src/startup.rs` | `CODEAGENT_UPDATE`. |
| `src/server/` | Daily task; `update_available` notice; `stop` and `status` on a version mismatch. |
| `src/protocol/` | The notice; version. |
| `src/client/`, `src/tui/`, `src/command/chat/` | Show the notice; replace an old idle server. |
| `src/command/update.rs` (new), `src/cli.rs` | `codeagent update`. |
| `ci/checksums.sh`, `ci/build.sh`, pipelines | Signature, target in the binary. |

## Dependencies

- `minisign-verify`: checks the signature; small, no dependencies,
  ISC licence.
- `tar`: reads the archive. `flate2` and `sha2` are already in
  `Cargo.lock` (through other crates) and become direct dependencies.
- `semver` (already in `Cargo.lock`) for comparing versions.

Each is checked with `ci/audit.sh` (licences, advisories) when added.

## Spec changes

- `spec/config.md`: new "Update" with `[update] auto`; "Project files"
  lists `[update]` as not allowed; "Environment variables" adds
  `CODEAGENT_UPDATE`.
- `spec/command.md`: `codeagent update`.
- `spec/sessions.md`: "Server": the daily check, and a newer client
  replacing an idle old server.
- `spec/architecture.md`: the `update_available` notice; the messages kept
  across versions; the protocol version; `src/update/` in the tree.
- `spec/tui.md`: the notice.
- `spec/security.md`: the entry-point row and the accepted risk below.
- RFC 0020: a note that RFC 0039 adds the signature.
- `readme.md`: updating, the setting and what it sends, the signature
  check for a manual install, the release secret.

## Security

Answers to "Checklist for a new feature" in `spec/security.md`:

1. **Input:** the release API answer and the downloaded files, from
   GitHub, or from someone who took over the account or the release.
   The answer is parsed as JSON with size limits; files must pass the
   signature with a key in the binary.
2. **Writes:** the state file and the download directory in the state
   directory (`0600`, `0700`), and the binary in its own directory. It
   cannot widen any permission; it replaces the program that checks them,
   which is why the signature is required.
3. **Child processes:** the new binary with `--version`, an empty
   environment and a timeout, only after the signature and checksum
   passed.
4. **Terminal:** the version (parsed, so only digits and dots), the
   release page URL (fixed) and the command; no text from the API answer.
5. **Limits:** one request a day, 10 s timeout, 64 MiB per file, one
   install at a time.
6. **Turned on by others:** a project file may not set `[update]`; an
   imported session cannot touch it.

New row in "Entry points and guards":

| Source | Trusted | Guard |
|--------|---------|-------|
| Release API and update files | No: the forge or a taken-over release account | Only with `auto = "check"` or `"install"` in the user file or `CODEAGENT_UPDATE`; a project file cannot set it. HTTPS to GitHub only; size limits; the archive is installed only when `SHA256SUMS` is signed with the key built into the binary and the checksum matches; one file unpacked, no links; atomic replace with the old binary kept. |

New accepted risk:

| Risk | Why it stays |
|------|--------------|
| Whoever has the signing key can ship a binary that every `install` setup runs. | The key is only in the release environment of the forges; `install` is opt-in, and the default `check` only shows a notice. |

## Testing

- Versions: newer, same, older, pre-release tags, junk; the state file
  (skip within 24 h, once per version, errors kept).
- The check against a local HTTP server (the test model server of the
  end to end tests): an answer, a 403 rate limit, a timeout, a body that
  is too large. The base URL is set by a test-only variable.
- Kind of install: a fake `dpkg` list, `CARGO_HOME`, a writable and a
  read-only directory, a path under `target/`.
- Install, in a temporary directory with a key pair made in the test:
  - a good release replaces the binary, keeps `codeagent.previous`;
  - a bad signature, a missing signature, a wrong checksum, an archive
    with a link, an absolute path, a second binary, a file over the limit,
    a binary that prints another version: each leaves the old binary and
    no temporary files;
  - two installs at once: the second waits or stops on the lock.
- Config: `[update]` in the user file; in a project file is an error with
  file:line; `CODEAGENT_UPDATE` wins.
- Server, in process: `off` makes no request; `check` sends one notice per
  version; a reload from `off` to `check` starts the check.
- Protocol: the notice round trips; a client of the old version ignores
  it.
- Old server, new client: the in-process server with another version
  answers `status` and `stop`; an idle one is replaced, a working one
  gives the error with the session names.
- `codeagent update` as a process: exit codes and `--json` against the
  local server.
- CI: `ci/checksums.sh` writes a signature that `minisign-verify` accepts
  (with a test key); `ci/test.sh` covers it.

## Implementation plan

1. **Signed releases**
   - Status: todo
   - Files: `ci/checksums.sh`, `ci/minisign.pub`, `ci/build.sh` (target),
     `.github/workflows/release.yml`, `.gitlab-ci.yml`, `readme.md`,
     RFC 0020 note
   - Result: a release has `SHA256SUMS.minisig`; the binary knows its
     target.
   - Tests: `ci/test.sh` with a test key.
2. **Check and state**
   - Status: todo
   - Files: `src/update/mod.rs`, `src/config/`, `src/startup.rs`,
     `Cargo.toml`, `spec/config.md`
   - Result: a function that finds the latest version and the kind of
     install, with the state file.
   - Tests: versions, state file, local HTTP server, kinds, config.
3. **codeagent update --check**
   - Status: todo
   - Files: `src/command/update.rs`, `src/cli.rs`, `spec/command.md`
   - Result: a manual check with exit codes and `--json`.
   - Tests: the command as a process.
4. **Install**
   - Status: todo
   - Files: `src/update/install.rs`, `Cargo.toml`, `spec/command.md`
   - Result: `codeagent update` replaces an archive install.
   - Tests: the install tests above.
5. **In the server**
   - Status: todo
   - Files: `src/server/`, `src/protocol/`, `src/tui/`,
     `src/command/chat/`, `spec/sessions.md`, `spec/architecture.md`,
     `spec/tui.md`
   - Result: the daily check, the notice, `install` in the background.
   - Tests: the server and protocol tests.
6. **Replacing an old server**
   - Status: todo
   - Files: `src/server/`, `src/client/`, `spec/sessions.md`,
     `spec/architecture.md`
   - Result: a new client replaces an idle old server.
   - Tests: the old server, new client tests.
7. **Spec, security, readme**
   - Status: todo
   - Files: `spec/security.md`, `readme.md`, `todo.md`
   - Result: the RFC is `done`.
   - Tests: `ci/check.sh`, `ci/test.sh`, `ci/audit.sh`.

## Alternatives

- **Notice only, never install.** Simpler and needs no signing key, but
  archive users, the ones with no package manager, would still update by
  hand. It is what `auto = "check"`, the default, does; `install` is
  opt-in.
- **Check provenance (sigstore) instead of a key.** No key to keep, but
  the sigstore crates are large and change often, and the check needs the
  network to the transparency log. A built-in Ed25519 key is a few lines.
- **Checksum over HTTPS only.** Protects against a broken download, not
  against a changed release, which is the case that matters.
- **Update in each client at start.** Costs start time, and many clients
  would check; the server checks once for all.
- **Restart the server by itself after an install.** It would have to wait
  for every session to be idle and would close every client's
  connection; the next start after the 60 s idle exit comes soon anyway.
- **An existing crate (`self_update`).** It brings its own HTTP stack and
  archive formats, and no signature check fitting the release; the few
  steps here are easier to check by hand.

## Open questions

1. Signing: is a `minisign` key in the release environment acceptable,
   which changes answer 5 of RFC 0020? Without it, only `check` can be
   built.
2. GitLab: should the check fall back to the GitLab releases when GitHub
   does not answer (only Linux archives are there)?
3. `codeagent update --rollback`, putting `codeagent.previous` back?
4. Replacing an old server: only when it has no working session (as
   proposed), or also ask the user in the TUI when sessions work?
5. Should the TUI show an available update in the status row as well as
   in the conversation?
