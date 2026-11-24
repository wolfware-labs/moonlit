# Changelog

All notable changes to the Moonlit CLI are documented here.

## [4.0.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-v4.0.0...moonlit-v4.0.1) (2026-11-24)

### Bug Fixes

* **config:** bound nesting depth and alias expansion in release files ([#63](https://github.com/wolfware-labs/moonlit/issues/63)) ([de0af03](https://github.com/wolfware-labs/moonlit/commit/de0af0347b5755740bcaf6dd7bdfc6ef922d6163))

## [4.0.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v3.0.3...moonlit-v4.0.0) (2026-11-23)

### ⚠ BREAKING CHANGES

* plugins built against moonlit:plugin@0.3.0 no longer
load; rebuild them against moonlit-pdk with ABI 0.4.0.

* test(pdk-macros): check the shipped ABI version against the real WIT path

The test read ../engine/wit, which moved to ../pdk/wit, so it compared nothing. Missing WIT now panics instead of skipping. The sample fixture lock is the refresh from rebuilding it for 0.4.0.

* feat(pdk): declare plugin permissions with reasons in moonlit_plugin!

A permissions list on the macro fills plugin-metadata.permissions in describe() and init(). Each entry needs a reason; malformed entries are compile errors.

* feat(engine): coverage rules for declared plugin permissions

* feat(engine): check every plugin's contract version before loading

Plugins are resolved first and their component imports read for the
moonlit:plugin version. Every plugin that targets another version is
reported together in one IncompatiblePlugins error, before anything is
instantiated, instead of the first one failing at link time.

* feat(engine): refuse to run when a plugin's required permissions are not granted

After describe, every plugin's required declarations are checked against
its grants and all gaps are reported in one error before steps are
flattened. The declared list is stored on the plugin's host state so the
consent flow can reuse it.

* feat(engine): fail the step when the sandbox blocks a plugin

Exec and network denials are recorded per execute and turn the
middleware result into a failure naming the missing permission, even
when the plugin ignores the host error. The fixture gains try-exec and
try-fetch middlewares that swallow errors to prove it.

* feat(cli): show and publish declared plugin permissions

inspect --json now carries the declared permissions and the contract
world, and refuses components built for another world. The published
artifact config carries the same permissions list for the registry.

* feat(engine): add a client for the Moonlit registry's GraphQL API

* fix(engine): append /graphql to registry URLs given with a scheme

* feat(cli): search the registry for plugins

Adds moonlit search, which lists matching plugins with a card per hit
and a table of the middlewares of its latest version. Registry errors
exit 3. Also lists the command in the CLI README table.

* fix(cli): draw search middleware tables as the design shows

The middleware table uses a full grid with a single rule under the
header and sits indented two spaces under the description.

* feat(cli): edit release.yml plugin entries without disturbing the rest of the file

moonlit add, update and doctor need to write plugin entries and grants
into a user's release file. Re-serializing the YAML would drop comments
and reformat everything, so edits splice text at parser spans and every
result is re-parsed before it is returned.

The parser now records each plugin item's span and its permissions
span. Quoted scalar and flow collection spans used to run on over a
trailing comment; they now end at the closing quote or bracket.

* fix(cli): keep release.yml edits safe for every valid plugin layout

A plugin item whose first key sat below its dash, or behind a comment
on the dash line, made add_plugin underflow and panic. The item span
now starts at the dash wherever it is, and the key column is read from
the first key.

Plain names and URLs could parse back as a different value (a URL cut
at ' #', a name 'null' read as empty). Values that are not plain-safe
are now double-quoted, and every edit checks that the edited plugin
reads back with the requested name and URL.

add_plugin parses the original without whole-file validation, so a
stage that already runs the plugin being added no longer blocks it; the
edited file is still fully validated. Inserted lines follow the file's
CRLF line endings.

* feat(cli): render the plugin permission consent screen

The add, update and doctor commands show what a plugin declares before
anything is written to release.yml. The verification badge is shared with
search.

* fix(cli): keep a gap between consent sentences and their values

The sentence column was padded to a fixed 32 characters, shorter than the longest sentence, so values ran into it. The width now follows the longest sentence. The exec icon is a single codepoint and the empty screen gets the blank line before the diff.

* feat(engine): pull plugins from loopback OCI registries over plain HTTP

The resolver always spoke HTTPS, so a registry on localhost or 127.0.0.1
(a local registry, or a mock in the CLI's integration tests) could not
serve plugins. Loopback hosts now use plain HTTP, as Docker does for
local registries; every other host stays HTTPS-only.

* feat(cli): add a plugin to release.yml after showing its permissions

`moonlit add` takes org/name[@version] or a plugin URL, picks the newest
registry version built for this CLI's world (or checks the pinned one),
reads the permissions the component declares, and shows them with the
release.yml diff before writing. --yes grants the required permissions
only, --with-optional grants all; without a terminal and without --yes
it refuses.

The resolve-and-describe step moves to plugin_probe so update and
doctor can reuse it, and tests get an OCI mock that serves a component
as a tagged artifact.

* fix(cli): show the consent screen before asking about optional permissions

On a terminal, add asked "Also allow ...?" before the user had seen the
plugin or its required permissions. The screen now comes first, then
the optional questions, then the release.yml diff that reflects the
answers, then "Allow and add?". The diff moves out of ConsentView into
render_changes so update and doctor can split the screen the same way,
and its lines now sit at the file line's indent.

* feat(cli): update plugins, asking again only for new permissions

* fix(cli): bump plugin versions without touching their permissions

* feat(cli): suggest compatible plugin versions when the contract differs

run and validate now print one block per incompatible plugin with the worlds involved and what to do: a newer registry version, a rebuild hint for file plugins, or an upgrade-the-CLI note.

* fix(cli): suggest the version doctor would pick for an incompatible plugin

Try the same major first and fall back to any major, so the hint names plain doctor or doctor --major as appropriate.

* test(cli): pin the same-major preference of incompatibility suggestions

The mock now offers a same-major and a higher-major compatible version, so a single cross-major lookup fails the test.

* feat(cli): add doctor to check and fix a release file

Checks that each plugin resolves, matches this CLI's plugin contract and
has its required permissions granted, then offers one combined fix: move
an incompatible registry plugin to the version compat suggests (same major
unless --major) and grant missing required permissions after the consent
screen. Problems it cannot fix stay listed and exit 1.

* fix(cli): ask before doctor writes and exit 3 when the registry is down

On a terminal doctor now asks once before writing any fix, plain
version moves included. When the registry cannot be reached while
planning a fix, doctor still prints every check, writes nothing and
exits 3 so the user can rerun once it is back.

* fix(cli): report only an unreachable registry as registry down in doctor

A suggested version that fails to fetch (missing tag, auth, digest or
cache error) is now a problem of that plugin alone: doctor names the
target, exits 1 and still applies the other fixes. Only a registry lookup
that cannot be made stops every write and exits 3.

* docs: document plugin permissions and the add, update, search and doctor commands

* docs: correct what add writes and the SDK example reference

* fix(cli): check for an existing plugin before the consent screen

add rendered the consent screen and asked every optional question
before add_plugin reported a name or URL clash, so the user answered
questions about a plugin that could not be added. A URL already in the
file gave no hint, and re-adding a declared registry plugin suggested
--name, which invites a duplicate install.

The clash check now runs right after the probe. A clash with the same
plugin (same URL, or the same registry host, org and name) points at
`moonlit update <name>`; a clash with another plugin keeps the --name
hint. NameTaken carries the declared URL so add can tell the two apart.

* fix(cli): list only new permissions on the update and doctor screens

update and doctor pass only the newly uncovered permissions to the
consent screen, so its "Not requested:" line named every other kind,
including ones the release file already grants. A plugin with exec
granted that newly needs the internet was shown as not requesting
"run programs on your machine".

ConsentView gains not_requested; add keeps the line, update and doctor
turn it off, as the spec says their screen lists only what is new.

* fix(cli): strip terminal control characters from plugin-supplied text

Names, versions, descriptions, permission values and reasons come from
a component's describe() or from the registry and were printed raw. A
hand-built component could return ESC/CSI sequences that conceal or
erase lines, for example to hide "Run programs on your machine" on the
consent screen.

display_safe turns C0/C1 control characters and bidirectional overrides
into visible escapes such as \x1b. It is applied to every plugin- or
registry-sourced field printed by the consent screen, search, plugin
inspect, update, doctor, the incompatibility report, error lines and
the step error messages that carry permission denials. release_file
quoting escapes the same characters, so they never land raw in
release.yml or its diff.

* fix(cli): skip oci hosts that are not Moonlit registries

update, doctor and the incompatibility report treated every oci:// host
as a Moonlit GraphQL registry. For a plugin on ghcr.io, POST /graphql
fails, so update aborted the whole run with exit 3 and doctor reported
the registry as down.

RegistryClient now returns NotARegistry when /graphql answers 404 or
405, or with a body that is not a GraphQL response. update prints
"<name>: not from a Moonlit registry, skipped" and goes on; the
incompatibility report and doctor give the rebuild hint. Connection
failures and other HTTP errors still mean the registry is unavailable.

* fix(cli): check permissions when reading an edited release file back

The read-back after each release_file edit compared only the plugin
name and URL, so an edit that wrote broader grants than requested would
have passed. Writing grants nobody asked for is the anti-goal of the
feature.

validated now also compares the re-parsed permissions with the ones
requested; for set_plugin_url they must equal the permissions before
the edit, absent block included.

* fix(engine): one contract check for load, probe, inspect and publish

The plugin contract was checked three times with two meanings: the
loader rejected a component that imports nothing from moonlit:plugin,
while probe and inspect let it through, so doctor could pass a plugin
that run refuses. publish had no check, and a 0.3.0 component failed
with a link error and exit 3 instead of the contract message.

world::check is now the one check, with one message ("built for X, this
CLI supports moonlit:plugin@0.4.0"). The loader, probe, plugin inspect
and plugin publish all use it; publish rejects another contract with
exit 2 before anything is pushed.

* fix(engine): time out registry requests

RegistryClient used reqwest::Client::new(), which has no timeouts. run
looks up suggestions after an incompatibility error, so behind a
firewall that drops packets run and validate could hang instead of
reporting the error.

The client now has a 10 s connect timeout and a 10 s request timeout
(REGISTRY_TIMEOUT); with_timeout replaces both, which tests use with a
short value.

* refactor(engine): move the terminal sanitiser into the engine

The engine has to escape plugin text in its own diagnostics, so the
sanitiser lives there and the cli display module re-exports it.

* fix(cli): keep the lines of multi-line step errors

display_safe escapes newlines and tabs, which is right for one-line
fields but collapsed multi-line step errors in the run output. Errors
now go through display_safe_text, which keeps \n and \t, and
continuation lines are indented under the failure line.

* fix(cli): escape plugin log, progress, warning and version text in run

Plugin-sourced strings were printed raw by the plain and pretty
renderers, so a plugin could clear or rewrite terminal lines. They now
go through display_safe.

* fix(engine): quote the values in the missing-permission help line

yaml_line wrapped values in bare quotes, so a value holding a quote or
a backslash produced a line that did not parse back. It now uses the
same quoting as the release file writer, which moved next to the
sanitiser.

* fix(engine): escape plugin text in diagnostics printed by run

The missing-permission diagnostic embedded the plugin's reason and
values raw, and plugin load and execution errors reached miette
unchanged. The diagnostic now escapes them, and run prints load and
execution errors through PipelineError::for_terminal.

* feat(cli): list incompatible plugins in the json error object

The json error for incompatible plugins only carried a count in its
message. It now also has a plugins array with each plugin's name, url,
the contract it was built for and the one this CLI supports.

* fix(cli): keep the summary table aligned and drop an unused re-export

A tab in a step error shifted the table border, so the summary cell
turns tabs into spaces. The pretty renderer tests no longer depend on
the colour settings of the environment.

* fix(cli): skip a registry that answers 401 or 403 on /graphql

public.ecr.aws refuses anonymous GraphQL with 401, which update and
doctor reported as a registry failure and exited 3. The engine now
returns RegistryError::Denied with the status, update and run warn
with the status and carry on with the other plugins, and doctor
reports the plugin as a problem.

* fix(engine): fail loudly when the registry client cannot be built

unwrap_or_default hid a builder failure behind Client::default(),
which panics on the same failure anyway, so the timeout was never
dropped silently. Build once and say what failed.

* docs(engine): describe the fields of VersionRule

* perf(engine): look at ten candidates at most, all at once

newest_compatible asked the registry about every version one after
the other, so a package with a long history cost one round trip per
release. It now checks the newest ten candidates concurrently.

* perf(cli): ask each registry once when run lists incompatible plugins

Incompatible plugins were looked up one after the other, so a
registry that never answers cost a full timeout per plugin. Hosts
are now queried concurrently, and once a host has failed to answer
the remaining plugins on it get the same answer without a request.

* style(cli): pass the registry error by reference

* fix(engine): time out plugin downloads and OCI transfers

The OCI pull and push clients and the plain HTTP download used no
connect or read timeout, so a server that accepts and never answers
hung the command. Connect is capped at 10 s and each read at 30 s,
so large blobs still finish.

* fix(engine): build the OCI clients explicitly instead of falling back

Client::new swallows a builder failure and returns a client without
the timeouts. Client::try_from with an expect makes the failure loud.

* fix(cli): print the denied-registry skip line with the other skip lines

* fix(cli): stop asking a host only when it cannot be reached

A 500 or a GraphQL error from one plugin's lookup marked every other
plugin on the host unavailable. Transport failures and timeouts are
now a separate RegistryError::Unreachable, and only that short-circuits
the host. A panicking host task is propagated, and the timing and
ordering tests get more margin.

* refactor(engine): give PermissionKind a verb_phrase instead of two lower_first helpers

* test(engine): pin the missing-grant diagnostic text, help, label and span

* test(engine): pin declaration order of incompatible plugins and aggregate missing grants

* fix(engine): keep the plugin's own error and append sandbox denials

A denial used to replace the plugin's error message, and a trap dropped
the denial entirely. Both are now kept, joined with '; '. The denial
message matches every permission kind explicitly instead of treating
anything but network as exec.

* fix(engine): keep sandbox denials next to non-trap plugin errors

An output that failed to decode dropped the denial. The error text and
the denial are now both reported. Also pin the fan-out of several
uncovered permissions from one plugin and the escaping of denied targets.

* refactor(cli): share add, update and doctor helpers and build plugin URLs with RegistryRef::with_tag

add kept private copies of fail, display_path and prompt_failed that
update already exported to doctor. They now live in commands/common.rs,
and the oci URL that three commands formatted by hand comes from
RegistryRef::with_tag. RegistryRef also derives Clone, Debug, PartialEq
and Eq.

* fix(add): say a terminal is needed to confirm, not that permissions are granted

A plugin that asks for nothing was refused with 'refusing to grant
permissions'. add now says it needs a terminal to confirm; update and
doctor keep the permissions wording.

* fix(update): do not call an incompatible plugin up to date

When no compatible version newer than the current one exists, update
printed 'up to date' even though the current version itself may not be
compatible. It now says 'no newer compatible version' unless the newest
compatible version is the one in use.

* fix(update): word the optional-permission line as not granted

The line was printed for every optional permission the grants do not
cover, including ones the previous version already declared, so 'new' was
wrong.

* refactor(auth): reuse the engine's loopback check

auth kept its own is_loopback that only knew localhost, 127.0.0.1 and
::1, while the registry client accepts all of 127.0.0.0/8. Both now use
registry::is_loopback, which also accepts a bare ::1.

* fix(engine): accept only a well-formed loopback host and port

is_loopback cut the registry at the first colon, so localhost:x@evil.com
counted as loopback and auth built a plain http URL to evil.com. The
port must now be non-empty digits and the bracketed IPv6 form must end
at the bracket or a valid port; anything else is not loopback.

* fix(doctor): name the registry status of a moved plugin in the consent screen

The consent screen for a plugin moved to a newer version showed no verification badge. The suggestion now carries the status of the version it proposes.

* fix(doctor): leave a digest-pinned plugin alone and say how to move it

doctor replaced an oci digest pin with a tag. A digest is a deliberate pin, so doctor now reports the incompatibility as an unfixed problem, writes nothing for it and tells the user to change the url to a version tag by hand, as update skips pinned plugins.

* fix(doctor): exit 3 when the plugin's own registry cannot be reached

A network failure while resolving the plugin itself was reported as an ordinary unfixed problem (exit 1). ProbeError::Resolve now records whether the failure was a network error, and doctor treats that as the registry being down, matching the lookup path.

* test(doctor): cover an unreachable move target and check sibling plugins

Adds the case where the suggested target cannot be fetched over the network (exit 3, nothing written), gives the exit-3 and digest-pin tests a second plugin to show all checks print and other fixes are written, and builds ProbeError::Resolve through one closure.

* fix(pdk-macros): check the permission kind first and explain a bad filesystem argument

An unknown kind without parentheses used to report 'expected parentheses', and an empty or non-identifier filesystem argument reported a raw parser error.

* fix(inspect): omit the empty permissions header in plain output

Pretty output already left it out; plain printed a bare header.

* feat(search): mark a latest version built for another plugin world

The fetched version detail already carries the world, so the check costs no request. JSON gains a compatible field, null when no detail is known.

* test(config): cover quoted and flow collection span ends

* docs(pdk): reword the optional permission reason sentence

* fix(search): say what add does and dim the incompatibility marker

The marker now tells the user that add picks the newest compatible version, and pretty output styles it apart from the bold title.

* test(inspect): pin the exact output shape without permissions

* refactor: share release-file edits, grant merging and denial text

set_plugin and set_plugin_url were near-copies, and add, update and
doctor each read and wrote the release file their own way. They now
share one set_plugin with optional permissions and common
read_release/write_release helpers. grant_missing replaces two copies
of the grant merge, Denial renders its own sentence for every caller,
FilesystemAccess orders by derive, and the search badge moves to the
shared display module so consent no longer depends on a command.

* fix(cli): escape plugin names in update and doctor status lines

A release file can come from a cloned repo, so a plugin name may carry
terminal control sequences. Escape the name field on every status line
instead of wrapping whole formatted lines.

* fix(engine): match optional network permissions with the network globset

optional_for_target built the exec globset for every kind; pick the
globset by kind the way is_covered does.

* test(cli): stop the silent-registry test from depending on machine load

The request count already proves the plugins on one host share a single
timeout. The tight wall-clock bound only failed under a busy test run.

* fix(engine): check grants before any plugin's init runs

The loader called init right after describe, so a plugin whose init
failed reported "init failed" (exit 3) instead of the missing grant
(exit 2), and plugin code ran before the refusal. Loading now has two
phases: instantiate and describe every plugin, refuse on missing grants
across all of them, and only then init.

* fix(permissions): cover a declared value its grant names verbatim

A declared value with `[` or `{` was matched as a literal against the
granted globs, so granting exactly what the plugin declared never
covered it, and a grant that failed to compile as a glob was dropped.
A declared value equal to a granted value now always counts as
covered, and moonlit_plugin! rejects values containing `[`, `]`, `{` or
`}` so declarations stay literal strings with `*` and `?` wildcards.

* fix(engine): escape the plugin name in missing-grant messages

The reason and values were passed through display_safe but the plugin
name, which comes from the release file, was printed raw, so escape
codes in it reached the terminal.

* fix(cli): let add start from a release file with no stages

The read-back after an edit ran the full validation, which requires at
least one stage, so `add` into a fresh `name: demo` file failed after
the consent screen. The read-back now parses the structure and runs the
full validation only when the edited file has stages, so a file that
was valid stays valid. A plugins block appended at the end of the file
no longer leaves a trailing blank line.

* fix(config): end a keep-chomped block scalar after the lines it keeps

Block scalar spans ran to the next token and content_end trimmed every
trailing line break, so a `|+` or `>+` scalar ended at its last content
line. An entry added after it, or permissions inserted after it, landed
before its kept blank lines and the value lost them. A block scalar now
ends at its content, or after the blank lines it keeps when it keeps
more than one line break.

* fix(cli): reject an edit that changes anything besides its entry

The read-back only checked that the edited entry parsed back with the
requested url and permissions, so a misplaced edit could still change
another plugin's config or the stages unnoticed. It now also compares
the whole document before and after with the edited entry left out,
and the edited entry itself apart from its url and permissions, and
refuses the edit on any difference.

* fix(cli): leave the url alone when a fix only grants permissions

set_plugin always rewrote the url scalar, so a doctor fix that only
added grants dropped the quotes around the url and showed a url change
in the diff. The url is now rewritten only when it changes.

* fix(cli): let update skip a failing plugin instead of stopping

Any lookup or probe failure made update exit 3 and drop every other
update, though only an unreachable registry warrants that. Now only a
registry that cannot be reached exits 3 with nothing written; any other
failure (a server error, an answer that cannot be decoded, a missing
tag, a component for another contract) prints `<name>: <error>, skipped`,
the other plugins are updated, and update exits 1 at the end.

* fix(cli): tell a registry error apart from an unreachable registry

Every registry failure but a refusal became RegistryUnavailable, so a
server error or an undecodable answer made doctor say "cannot reach
the registry" and exit 3. Only an unreachable registry maps there now;
any other answer becomes a RegistryError suggestion, shown as "the
registry answered with an error: ...". doctor reports such a plugin as
unfixed, applies the other fixes and exits 1.

* fix(registry): refuse plain-HTTP GraphQL to a remote registry

`--registry http://<remote>` sent registry queries unencrypted. The
OCI client already allows plain HTTP only for loopback hosts, and the
GraphQL client now follows the same rule: for_host and with_base_url
return RegistryError::Insecure for an http:// URL whose host is not
localhost, 127.0.0.0/8 or [::1]. search and add exit 2 on it before
sending anything.

* refactor: share load errors and registry error mapping after the fixes

Splitting the loader into describe and init copied its error closure,
search grew a second error-print block, and compat built a registry
error suggestion by hand next to the mapper that already does it. One
helper or path each now. all_granted builds its own glob set so the
list and the set cannot disagree.

* fix(cli): find the edited entry by position, not by its YAML value

A plugin named 1.10, True or 0x1f reads back from serde_yaml_ng as a
number or boolean whose text differs from the raw name, so the read-back
check could not find the edited entry and refused a valid edit. Locate
the entry by its index in the engine's structural parse instead.

* fix(config): keep an empty keep-chomped block scalar's span in place

For an empty |+ or >+ scalar saphyr starts the span at the next token,
so counting kept newlines forward ran through the keys that follow and
gave the value a span over them. Keep saphyr's span when the scalar has
no content, and count kept newlines only within the blank run right
after the content.

* fix(cli): make update exit 1 when a registry refuses anonymous lookups

A registry answering 401 or 403 on /graphql left the plugin un-updated
while update still exited 0, so scripts could not tell. Treat it as a
per-plugin failure: print the skip line, update the rest, exit 1. A host
that is not a Moonlit registry stays a plain skip.

* fix(resolver): tell an OCI error answer apart from an unreachable registry

Every OCI failure that was not auth, not-found or a digest mismatch
became a network error, so a registry answering 500 made doctor and
update say they could not reach it and exit 3. Keep network for
connection failures and timeouts only, and report any other answer as
a registry error, which is a per-plugin failure (exit 1).

* fix(registry): refuse a redirect to plain HTTP on a remote host

The GraphQL client followed any redirect, so a registry reached over
HTTPS or loopback could send the POST on to a plain-HTTP remote host.
Stop such a redirect with the same Insecure error the client gives for
a plain-HTTP registry URL, and keep following the rest up to ten hops.

* fix(resolver): tell an HTTP plugin error answer apart from an unreachable host

A plugin URL whose server answered 500 or any other non-404 error status
was reported as a network error, so doctor and update said they could
not reach it and exited 3. Report it as an error answer from the server
(exit 1 per plugin), as the OCI resolver now does; connection, timeout
and body-read failures stay network errors.

* perf: add benchmarks for grant checks and release file edits

check_grants times the check run makes before any plugin code runs:
five declared permissions against glob grants. add_plugin and
set_plugin time the comment-preserving release.yml edits, including
the read-back of the whole document. The release.yml the engine
benchmarks parse moves to a fixture file so both crates' benchmarks
edit the same file.

* perf(cli): parse the release file once per edit

The read-back check parsed both versions of the file again to find the
edited entry's position. add_plugin and set_plugin already hold the
parsed file, and the read-back already parses the edited one, so pass
the positions in instead. add_plugin also stops parsing the file twice
through check_addable.

* refactor(registry): check plain HTTP the same way for URLs and redirects

with_base_url split the authority by hand while the redirect policy
read the parsed URL's host. Both now ask one predicate on the parsed
URL, so a URL with user info or a non-canonical loopback address is
judged the same whether it is given or redirected to.

* refactor: share test helpers and trim block_end

value_text reads through value_texts, doctor tests read combined output
through one helper, block_end measures the trimmed scalar once, and the
grant benchmark's builder no longer shares its name with the list it
fills.

* test(cli): answer consent prompts on a pseudo-terminal

The prompt branches of add, doctor and update could not run in tests
because stdin was never a terminal, so a flipped answer check or a
broken Ctrl-C path went unnoticed. A raw pty that already holds the key
presses lets the integration tests answer each prompt.

Search and update output in plain mode is now checked for ANSI codes
with colors forced, and the badges for their colors in pretty mode.

* test(cli): cover consent rendering, diffs and release edits

Unit tests now pin the consent icons, the value column, the secret
warning and the badge color, render_diff on inputs where the LCS table
decides the order, edits to files without a final newline, comments
after a dash, read-back by name and URL, per-host registry grouping,
same_plugin and step log printing.

Code that a mutant could not change is simplified: add passes the
default version rule (cross_major has no effect without a current
version), update compares versions with one cmp match, the diff walk
compares against the cell it is in, and key_column's unreachable
fallback no longer does arithmetic.

* test(engine): cover config spans, prerelease rule and client timeouts

Spans: a quoted scalar before a comment, a keep-chomped scalar keeping
two breaks, block and flow values ending a plugin, and a flow plugin
list under a line ending in a dash. VersionRule: a stable current
version does not pick a prerelease.

The pull and push OCI clients now build their config with one
client_config function, tested for both timeouts.

* test: cover update, doctor and compat error paths

Raise patch coverage toward the Codecov patch target. The tests reach
update's unknown-name, file-plugin and older-answer paths, doctor's
registry-down, non-component and still-incompatible move targets,
compat's offline, bad-host and no-versions lookups, the registry's
envelope and redirect-loop errors, and small conversion and quoting
arms. All are appended to trailing test modules or test files, so no
production line moves.

* test: cover permission expansion and OCI request error mapping

cargo-mutants runs only the mutated package's tests, and the macro's
success paths were exercised only from other crates. Parse each
permission kind and check the generated requests inside pdk-macros.

Pin which reqwest errors count as network failures: a body read timeout
maps to Network, a request that cannot be built maps to Registry.

* refactor(engine): match optional permissions with one glob builder

network_globset and exec_globset both build the same GlobSet, so the
per-kind match in optional_for_target chose between identical calls
and its Network arm could be deleted without any behaviour change.

* fix(registry): match the http scheme case-insensitively

URL schemes are case-insensitive, so `HTTP://remote/graphql` skipped the
plain-HTTP refusal and the GraphQL request went out unencrypted.

* test(pdk-macros): skip the ABI doc check outside the workspace

The packaged crate has no sibling pdk/wit directory, so the test
panicked when run from the published crate.

* test: fix the macOS pty, Windows url and docker smoke fixtures

openpty on macOS takes mutable termios and winsize pointers. A Windows
fixture path has backslashes, which the release file writer quotes. The
docker smoke pipeline now grants the network permission the test
plugin requires.

* test(pdk-macros): pin the rejection of extra permission arguments

syn already reports leftover tokens inside a permission's parentheses
as an unexpected token, so filesystem(read_only, read_write) does not
compile. Nothing tested it.

* fix(engine): cover a declared glob only by a grant that includes it

Preflight matched each grant glob against the declared pattern's text,
so a grant of a?c covered a required exec("a*c") even though a real
call to abbbc is denied at runtime. A declared * or ? pattern is now
covered when every value it matches also matches the grant: the grant's
* absorbs any run, its ? absorbs one literal or ?, never a *. Patterns
with classes, alternatives or escapes still need an exact grant.

* fix(engine): keep looking past ten versions for a compatible one

newest_compatible asked about the ten newest candidates and stopped.
After a plugin contract bump, a package can easily publish ten or more
versions for the new contract, so a CLI on the old one was told no
compatible version exists. It now asks in batches of ten, newest first,
and stops at the first batch with a match.

* fix(engine): escape the plugin name in terminal load errors

for_terminal escaped the load message but printed the plugin name as
written in the release file, so a name with control characters could
still reach the terminal.

* perf(cli): fetch search hit details ten at a time

search asked the registry about each hit's latest version one after
the other, so ten results cost ten round trips and a larger --limit
grew with it. It now asks about up to ten hits at once and keeps the
hit order.

* fix(cli): escape plugin names in run output and match any-case schemes

The plain and pretty renderers escaped the plugin version but printed
the release-file plugin name as is, in both the resolving and ready
lines, so a control character in a name still reached the terminal.

RegistryClient::for_host accepts HTTPS:// in any case, but add built
the OCI reference with a case-sensitive strip, so --registry
HTTPS://host/graphql gave oci://HTTPS:/... and the pull failed.

* fix(cli): name a flow-style plugin entry instead of failing on its yaml

update and doctor insert permissions as block lines, which cannot go
inside a flow mapping such as `- { name: b, url: ... }`. The read-back
check already refused the edit, so no broken file was written, but the
user saw a parser message ("did not find expected '-' indicator").

set_plugin now returns FlowStyle naming the plugin when it would have
to write permissions into a flow-style entry, and add_plugin does the
same for a flow-style plugins list. A url-only change to a flow entry
still works.

* fix(cli): read oci:// pins in any case in update and doctor

The release parser accepts a plugin URL scheme in any case, so
OCI://host/org/name:1.0.0 is a valid registry pin, but registry_ref
matched only lowercase oci://. update and doctor then took the pin for
a non-registry URL and skipped it.

strip_scheme matches a scheme in any case; registry_ref and add's
oci_host both use it now.

* fix: resolve plugin urls with any-case schemes; doctor blames the url

The release parser accepts OCI://, FILE:// and HTTPS:// in any case,
but PluginSource::from_str matched only lowercase schemes, so such a
pin passed parsing and then failed to resolve in doctor, update and
release. The resolver now matches the scheme in any case.

doctor treated every network error as the registry being down, which
made it exit 3 and hold back the fixes for the other plugins. A plugin
pinned to a direct http(s) or file url has no registry; its network
error is now reported for that plugin and the rest are still fixed.

* fix(cli): escape control characters in the consent diff

add, update and doctor print the release-file lines they are about to
change. The new lines came from escaped values, but the removed lines
were the file's raw text, so a stray \r or escape sequence in an
existing entry reached the terminal. Each diff line now goes through
display_safe_text.

* fix(cli): keep a search hit's middlewares when it has no version details

A hit with no latest version, or whose version lookup finds nothing,
lost the middleware names the search response already listed. search
now shows those names, without descriptions, in both the table and the
JSON output.

* fix(engine): ignore version details the registry returns for another version

newest_compatible filters the version list by the major and pre-release
rules, then asks for each candidate's details, but it took whatever
version those details named. A registry that answered the 1.3.0 lookup
with 2.0.0-rc.1 got update to pin a version the rules had excluded.
Details now count only when their version is the one asked for.

* fix(engine): fail plugin init and middleware listing on a denied host call

A middleware run fails when the plugin hits a denied host call, even if
the plugin ignores the error. init and list-middlewares did not check:
their denials stayed in the log until the next execute cleared it, so a
plugin could load cleanly after being blocked, and the message telling
the user which permission to grant was lost. Both now fail with the same
denial message. The test fixture gains execOnInit and execOnList config
keys to exercise this.

* fix(engine): reject a plugin that imports more than one contract version

component_world returned the version of the first moonlit:plugin import
it saw. A component importing both 0.4.0 and 0.3.0 interfaces passed the
contract check when the 0.4.0 import came first, and then failed at
instantiation instead of with the contract mismatch error. It now scans
every import and reports the first version other than the supported one.

* fix(cli): escape introspection errors before they reach the terminal

Since list_middlewares fails on a denied host call, its error carries
the program name the plugin asked to run, unescaped. `plugin inspect`
and `plugin publish` print introspect's error as is, so a plugin could
send escape sequences to the terminal through that name or a trap
message. introspect now escapes its error text, which covers those two
commands and the probe used by add, update and doctor.

* fix(pdk-macros): reject a backslash in a permission value

The macro let `\` through while the engine's coverage check rejects it,
and globset reads it as an escape on Unix but as a plain character on
Windows. `exec(["foo\\*"])` compiled, then meant a program literally
named `foo*` on Unix and anything under `foo\` on Windows. The macro
now rejects `\` with the other glob characters it does not support, and
the PDK README lists it.

* fix(engine): ignore a package version answer for another version

`moonlit add org/name@1.0.0` took the version, URL tag and status from
the registry's answer without checking that the answer was for 1.0.0,
so a registry that returned another version got that one added.
package_version now returns None when the answer names a different
version, which covers add, search and newest_compatible. The same check
in first_for_world is gone, since every lookup there goes through
package_version.

* fix(engine): reject declared permission values with unsupported globs

The moonlit_plugin! macro only allows `*` and `?` in permission values,
but a component built without it could declare `{git,sh}` or
`git[0-9]` in describe. The consent screen shows that string as is,
while the host's globset reads it as a pattern for several programs, so
a user could approve more than the text suggests. describe now fails
with a load error when a declared value uses `[`, `]`, `{`, `}` or `\`,
using the same parser as the grant coverage check. The test fixture can
declare an extra exec value from TEST_PLUGIN_EXTRA_EXEC to cover both
the rejected and the accepted case.

* fix(engine): check every rule of a permission declaration on load

describe only rejected unsupported glob syntax, so a component built
without moonlit_plugin! could still declare a blank reason, an empty
network, exec or env list, or values on filesystem. An empty required
list counts as covered, so such a plugin loaded with a declaration that
said nothing. describe now applies every rule the macro enforces and
fails with a load error naming the first one broken. The PDK README
states that the CLI checks all of them.

* feat(pdk)!: return nothing from init on success

describe and init both returned plugin-metadata. Once that record
carried permissions, a component could state one set in describe and
another in init, and the engine read only the version from init and
ignored the rest. init now returns result<_, string>, so describe is
the one place a plugin states its name, version and permissions, and
the loader takes the version for the ready event from it. Plugins
built with moonlit_plugin! need no change, since the macro writes init.
* the init export of moonlit:plugin@0.4.0 returns
result<_, string> instead of result<plugin-metadata, string>.
Components that implement Guest by hand must return Ok(()) from init.

* refactor(engine): walk item_start's lines with a finite iterator

item_start scanned backwards with `end = line_start - 1` in a bare
loop. Mutating that `-` to `+` or `/` kept `end` from moving back, so
the loop never ended and both mutants timed out in CI's mutants shard
instead of failing a test. It now walks `yaml[..start].rsplit('\n')`,
which always ends, and returns `start` when the lines run out, which is
where the old `line_start == 0` exit returned.

* ci: free disk for the mutants shards and log disk and memory use

Shard 3 died with "The runner has received a shutdown signal" on every
head, a minute into its first mutant, while the same mutant is caught
locally in about a second. The cache holds only dependencies, so each
shard builds every test binary on the runner, and an in-place mutant
rebuild adds incremental data on top. The job now removes the .NET,
Android, GHC and CodeQL toolchains the runner image ships, prints free
disk and memory, and logs both every 30 seconds while cargo-mutants
runs, so a shard that still dies shows which one ran out.

* ci: mutate pdk-macros too and name the processes using memory

The mutants job only mutated moonlit-engine and moonlit, so the
permission parsing in moonlit_plugin! had no mutation coverage in CI.
It now passes --package moonlit-pdk-macros as well.

The disk and memory log showed shard 3 is not short of disk: memory
went from 2.2 GB to 15.8 GB of 16 GB within half a minute of the first
mutant's tests, and the runner was shut down. The same mutant peaks at
3.7 GB locally under a 16 GB cgroup with 4 jobs. The log now samples
every 10 seconds and names the three processes with the most resident
memory, so the next run shows which one grows.

* build: move the pinned toolchain to Rust 1.99.1

The mutation-testing runner the project uses only ships 1.99.1, so a
1.97.0 pin made it refuse the repo. rust-version stays at 1.97; only the
toolchain CI and local builds use moves.

Clippy 1.99 adds two pedantic lints this code trips. assert_is_empty
wants every `assert!(x.is_empty())` in the tests rewritten as an
assert_eq! against a typed empty value, and unused_async_trait_impl
flags wasmtime host trait methods whose generated trait requires
`async fn`. Both are allowed in the workspace lints. result_large_err
now also fires on the CLI's execute, which returns PipelineError; it
gets the same allow the engine's pipeline and engine modules have.

* build: resolve the lockfile after merging main

The merge kept a wasmparser 0.259.0 entry for the engine, which the
workspace requirement of 0.258.3 does not allow, so --locked builds
refused the lockfile.

* refactor: share the scheme matcher, the silent sink and component checks

The branch had three copies of the any-case scheme matcher, eight of a
sink that drops host events, and the same component checks in both
plugin inspect and plugin publish. Each now lives in one place:
scheme::strip_scheme, plugin::host::NullSink and wasm::load_component.

config::Plugin becomes PluginDeclaration, which drops the re-export
alias that hid its clash with plugin::Plugin. The exec and network
globset aliases go, since both only called build_globset.

Two messages change. moonlit add on a version built for another
contract prints the engine's own mismatch text, and plugin publish
strips oci:// in any case, as the other scheme checks do.

* fix(cli): show the consent screen on the stream the prompt uses

The permission list and the release.yml diff went to stdout while the
question went to stderr. With stdout redirected or piped, the user saw
only "Allow and add?" and a yes wrote grants they had not read.

When a prompt follows, add, update and doctor now write both to
stderr. With --yes, or when nothing is asked, they stay on stdout.

* fix(cli): refuse a release.yml edit that cannot be checked

The check that an edit changes nothing outside the plugin's entry was
skipped when the second YAML parser rejected the file, for one on a
duplicate key in a plugin's config. An edit to one plugin's url then
reached another plugin through an anchor. The edit is now refused.

* fix(resolver): do not follow a redirect from https to plain http

The download client followed any redirect, so a plugin asked for over
https could be fetched over http. A download that is on https now
stops at such a redirect; the error names the target's scheme and
host only, since the rest of the URL can hold a token.

* fix(engine): warn when an optional permission is denied

A denial failed the step even for a permission the plugin declared
optional and handled itself, although the PDK README calls those
permissions ones the plugin can run without.

A denial covered by an optional declaration now adds one warning per
target and leaves the result alone. Init and list-middlewares no
longer fail on one. Any other denial fails the step as before.

* refactor(engine): share the redirect limit and refusal lookup

The download client and the registry client each had a hop limit and
a walk down a reqwest error's sources to find the policy's refusal.
Both now use redirect::too_many and redirect::refusal.

The registry client stopped after 9 redirects; it now follows 10,
as the download client and reqwest's default do.

The redirect download test uses wiremock, and the three terminal
tests share one shown-before-the-question assertion.

* fix(cli): say when release.yml itself cannot be checked

A file the check cannot read was reported as "the edited release
file would be invalid", which blames the edit. It now has its own
error, with a hint for the two causes a user can fix by hand: a
duplicate key, and an integer too large to read unquoted.

* fix(engine): bound optional-denial warnings and keep them on a trap

A plugin that trapped after an optional permission was denied showed
a bare trap, with nothing about the refused call. The trap message,
and a failed init or list-middlewares, now name those denials.

A call reports at most 20 such warnings and counts the rest, and they
are deduplicated through a set, not a scan of the list.

A redirect chain longer than the limit now has a test that fails
when the download client's redirect policy is not installed.

* refactor(engine): word a call's denials in one place

settle_call and apply_denials each joined a failed call's own error
with its denials, and apply_denials repeated the split that
PluginInstance::denied does. denial_report and failure_message now do
both, and apply_denials is one match with the same results.

The limit tests use MAX_DENIAL_WARNINGS, and the uncheckable-file
error takes the file name from SOURCE again.

* fix(cli): escape the file name in the core-module error

The error for a core wasm module printed the path as given, so a
file name with control characters reached the terminal.

The probe that add, search and plugin inspect use now tells the
instance what the plugin declared before it lists the middlewares.
Without that, a refused optional permission there failed the probe.

* fix(cli): escape the release file path shown beside the consent diff

add, update and doctor print the release file's path above the diff
and in their status lines. A --file-path value was printed as given,
so a newline or an escape sequence in it reached the terminal next to
the consent screen.

* fix(engine): let a required declaration win over an optional one

A plugin can declare one target both required and optional. A denial
of it was matched to the optional entry and only warned. The loader
does not start a plugin whose required permissions are not granted,
so such a denial is rare, but when it happens the step must fail.

* fix(cli): bound the walk that renders the release.yml diff

render_diff walked both files in a loop that ended only because every
branch moved a cursor. With one cursor stuck it pushed a line on each
pass, with no limit. The walk needs at most as many steps as the two
files have lines, so it now counts them.

CI found it: the mutant that turns one of those increments into a
no-op used up the runner's memory before its timeout, twice.

* fix(registry): refuse a redirect that leaves https

The registry client refused a redirect to plain http only when the
target was a remote host, so a remote https registry could send the
request to a plain-http service on this machine. A request that is on
https now stops at any redirect to another scheme. A local registry
on plain http still redirects to itself.

The plugin download client used the same test; both share it now.

* fix(cli): match a declared plugin's registry host in any case

moonlit add compared the host of the plugin being added with the one
already in release.yml byte for byte. A host written in another case
was taken for a different plugin, and the error suggested --name when
it should point to moonlit update.

* fix(deps): bump url to 2.5.9 for RUSTSEC-2026-0592

url 2.5.8 overflows the stack in Url::origin() on nested blob: URLs.
2.5.9 fixes it and needs form_urlencoded 1.2.3, so that moves too.

* style(cli): drop the comment on render_diff's step bound

The walk takes at most one step per line of the two files, and the
bound keeps a stuck cursor from pushing lines without end. That is
said in the commit that added the bound, so the code does not repeat
it.

* fix(cli): find a declared plugin URL in any scheme or host case

moonlit add refused a URL that release.yml already holds only when
the two matched byte for byte. The same registry written with another
case in its host or scheme got through and was added a second time.
The scheme and the host now compare in any case; the rest of the URL
compares as written.

### Features

* let plugins declare permissions and add search/add/update/doctor ([#52](https://github.com/wolfware-labs/moonlit/issues/52)) ([5c357fb](https://github.com/wolfware-labs/moonlit/commit/5c357fb63ed6b2c390e186bdd4bd9a9728fc0eba))

## [3.0.3](https://github.com/wolfware-labs/moonlit/compare/moonlit-v3.0.2...moonlit-v3.0.3) (2026-11-20)

### Bug Fixes

* **release:** build the macOS arm64 binary on macos-15 ([#59](https://github.com/wolfware-labs/moonlit/issues/59)) ([7dc33c0](https://github.com/wolfware-labs/moonlit/commit/7dc33c053f45a419b7809c3b90fab8eecb257c1f))

## [3.0.2](https://github.com/wolfware-labs/moonlit/compare/moonlit-v3.0.1...moonlit-v3.0.2) (2026-11-20)

### Bug Fixes

* **deps:** bump wasmtime to 49.0.7 and the TLS stack for advisories ([#57](https://github.com/wolfware-labs/moonlit/issues/57)) ([ffe2bf9](https://github.com/wolfware-labs/moonlit/commit/ffe2bf94474143fd3ec037d3bbd8cac21310e972))

## [3.0.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-v3.0.0...moonlit-v3.0.1) (2026-10-29)

### Bug Fixes

* **oci:** send a stored registry token as Basic credentials ([#45](https://github.com/wolfware-labs/moonlit/issues/45)) ([11459bb](https://github.com/wolfware-labs/moonlit/commit/11459bbef57bd71cfe1f89d40871d2c17eb0a50f))

## [3.0.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v2.0.0...moonlit-v3.0.0) (2026-09-27)

### ⚠ BREAKING CHANGES

* restructure the engine around Pipeline and a WASI-agnostic Engine (#24)

### Changes

* **cli:** move the modules into a lib target ([#12](https://github.com/wolfware-labs/moonlit/issues/12)) ([b85d3ba](https://github.com/wolfware-labs/moonlit/commit/b85d3ba33555bef6c48d7b9c6a277374d974d835))
* restructure the engine around Pipeline and a WASI-agnostic Engine ([#24](https://github.com/wolfware-labs/moonlit/issues/24)) ([2b60f7b](https://github.com/wolfware-labs/moonlit/commit/2b60f7b70e93e9931c79e701d078288a6dbb2d36))

## [2.0.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.2.0...moonlit-v2.0.0) (2026-09-12)

### ⚠ BREAKING CHANGES

* **cli:** `moonlit login`, `logout` and plugin resolution with no explicit
host now target registry.moonlit.rs instead of registry.moonlitbuild.dev. A
credential already stored for the old host is no longer found by default, so
re-run `moonlit login`. Plugin URLs that name registry.moonlitbuild.dev
explicitly are not rewritten by the CLI -- the old hostname answers with a
permanent redirect, which OCI clients follow inconsistently on the token-auth
request, so update them to registry.moonlit.rs.

### Features

* **cli:** default to registry.moonlit.rs ([74b74b0](https://github.com/wolfware-labs/moonlit/commit/74b74b02398b8f6318011bbf8111563b72a4bced))

## [1.2.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.10...moonlit-v1.2.0) (2026-09-05)

### Features

* **cli:** strict pipeline validation and a cleaner command surface ([dba9681](https://github.com/wolfware-labs/moonlit/commit/dba9681fd6792ce7c382529ddd9dcd500180c2d1))

### Changes

* **cli:** discover release.yaml instead of moonlit.yml ([f9f30b3](https://github.com/wolfware-labs/moonlit/commit/f9f30b3c38b1700fc52553b0b6eab18afa8b3540))
* **cli:** drop the -d alias for --working-dir ([33c3009](https://github.com/wolfware-labs/moonlit/commit/33c300952cbe717937a2a5247c050615a1808c06))
* **cli:** print help instead of the version banner with no subcommand ([5fa9075](https://github.com/wolfware-labs/moonlit/commit/5fa9075de461aebd0fcf6ad85f27f5cdba459524))
* **config:** match schema keys case-sensitively ([d9f36be](https://github.com/wolfware-labs/moonlit/commit/d9f36befb1bd56f26162789b0c71e192a10ca65a))
* **config:** reject a schema key with no value ([99d0bbc](https://github.com/wolfware-labs/moonlit/commit/99d0bbcf411503e1082e94e542f0837865e730c8))
* **config:** reject duplicate schema keys ([9321c05](https://github.com/wolfware-labs/moonlit/commit/9321c055fc71f5c7b32b380afab5acf00da0cb44))
* **config:** reject malformed schema values instead of defaulting ([90c17b5](https://github.com/wolfware-labs/moonlit/commit/90c17b5ab9b24f8486fb41431138554a0405d322))
* **config:** reject unknown schema keys ([9ae9cb2](https://github.com/wolfware-labs/moonlit/commit/9ae9cb2dc51deedf1c54bd3fce891b9d4c901118))
* **config:** write validation errors in Moonlit's own voice ([e26b24b](https://github.com/wolfware-labs/moonlit/commit/e26b24b7abda044368082bc5438ef5a224ec6752))
* **engine:** label diagnostics with the file that was read ([8a11538](https://github.com/wolfware-labs/moonlit/commit/8a115383b66d28b8010ad264d74c36725e47d546))

## [1.1.10](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.9...moonlit-v1.1.10) (2026-09-05)

### Bug Fixes

* **cli:** clear rustc wrappers too before the plugin wasm build ([657fe91](https://github.com/wolfware-labs/moonlit/commit/657fe911894794f8cef287330c09e725ac225ce8))

## [1.1.9](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.8...moonlit-v1.1.9) (2026-09-05)

### Bug Fixes

* **cli:** stop host rustflags leaking into the plugin wasm build ([330091b](https://github.com/wolfware-labs/moonlit/commit/330091bb1752910363cb6034ff52d736b595fa76))

## [1.1.8](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.7...moonlit-v1.1.8) (2026-09-05)

### Bug Fixes

* **choco:** give the package icon an opaque background ([2b033c4](https://github.com/wolfware-labs/moonlit/commit/2b033c4dd1c11e04c7acd98453dea93fe12d84d4)), closes [#0B0E0F](https://github.com/wolfware-labs/moonlit/issues/0B0E0F)

## [1.1.7](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.6...moonlit-v1.1.7) (2026-09-05)

### ⚠ BREAKING CHANGES

* **pdk:** rename moonlit-sdk to moonlit-pdk

### Bug Fixes

* **choco:** use the wolf mark as the icon and fail on a rejected push ([5f8d553](https://github.com/wolfware-labs/moonlit/commit/5f8d55395db5414df1a021dfeeb594c3a345a4fc))

### Code Refactoring

* **pdk:** rename moonlit-sdk to moonlit-pdk ([96c50e1](https://github.com/wolfware-labs/moonlit/commit/96c50e183ed56d6784f70be9faaf540f635f304b))

## [1.1.6](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.5...moonlit-v1.1.6) (2026-08-08)

### Bug Fixes

* **choco:** give the Chocolatey package an icon and full metadata ([77b6e5b](https://github.com/wolfware-labs/moonlit/commit/77b6e5b9ac968f9399a95fdd79726c6dc25c438c))

## [1.1.5](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.4...moonlit-v1.1.5) (2026-08-05)

### Bug Fixes

* **cli:** resolve plugin artifacts from cargo metadata ([9ca348a](https://github.com/wolfware-labs/moonlit/commit/9ca348a99ed5bd033777f53d4b71b67b3220393b))

## [1.1.4](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.3...moonlit-v1.1.4) (2026-08-03)

### Bug Fixes

* **engine:** stamp the plugin world this engine actually ships ([a6e5fb2](https://github.com/wolfware-labs/moonlit/commit/a6e5fb22d32cc25973a8a1ebe8af47399d56593b))

## [1.1.3](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.2...moonlit-v1.1.3) (2026-08-01)

### Bug Fixes

* **ci:** let the tolerated Chocolatey 403 actually pass the job ([08d0fb1](https://github.com/wolfware-labs/moonlit/commit/08d0fb10576e7b12a883aeaf4d00a5ede5f0f253))

## [1.1.2](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.1...moonlit-v1.1.2) (2026-08-01)

### Bug Fixes

* **cli:** do not promise a server-side revoke logout cannot always do ([355f34c](https://github.com/wolfware-labs/moonlit/commit/355f34ceb2b8c371bc1cf3cf921b9cc3322252ba))

## [1.1.1](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.1.0...moonlit-v1.1.1) (2026-08-01)

### Bug Fixes

* **login:** bound registry requests with request and connect timeouts ([debb9c1](https://github.com/wolfware-labs/moonlit/commit/debb9c17469da6c2bd16b97b1c61671c64a9edcd))
* **logout:** remove the local credential for Basic logins ([e025b0f](https://github.com/wolfware-labs/moonlit/commit/e025b0fed7d286dcac710cca643a099d7f11a921))

## [1.1.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v1.0.0...moonlit-v1.1.0) (2026-08-01)

### Features

* **login:** default to RFC 8628 device flow, keep --token for CI ([3a457f3](https://github.com/wolfware-labs/moonlit/commit/3a457f3c2d5d6e08869bdd17f7e11d8b7fc6f31f))
* **login:** optional host defaulting to registry.moonlitbuild.dev ([892f332](https://github.com/wolfware-labs/moonlit/commit/892f332ce81a6ece5044289c4e22fe995391a312))
* **logout:** revoke server-side and remove local credential ([9c29297](https://github.com/wolfware-labs/moonlit/commit/9c2929705336d84e39f50e04ca87f8c0b00634a9))

### Bug Fixes

* **login:** harden device flow — home-dir, atomic 0600, URL scheme, timeout, retries ([fc35e3e](https://github.com/wolfware-labs/moonlit/commit/fc35e3e31fc4275e2de5f81158459c7c9bf70ce8))
* **login:** match loopback host exactly so lookalikes aren't downgraded to http ([a1df84a](https://github.com/wolfware-labs/moonlit/commit/a1df84a8cb3726b3a7f2cd41cbab3683f26a0d6a))

## [1.0.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v0.2.0...moonlit-v1.0.0) (2026-07-31)

### ⚠ BREAKING CHANGES

* **cli:** `moonlit plugin inspect --output json` replaces each
middleware's `configSchema` field with `inputSchema` and `outputSchema`.
* **sdk:** moonlit:plugin world 0.2.0 -> 0.3.0. Middleware::Config renamed to
Input; new Middleware::Output. middleware-info.config-schema renamed to input-schema and
output-schema added. MiddlewareResult is now generic (MiddlewareResult<Output>) with ok()/
failure(); success()/success_with() removed. SDK adds NoInput/NoOutput markers.

### Features

* **cli:** declare middleware input and output schemas in inspect ([76ce63f](https://github.com/wolfware-labs/moonlit/commit/76ce63fcb3310026a2d325fb993feb172540c68d))
* **sdk:** rename Config to Input, add typed Output, emit input/output schema (ABI 0.3.0) ([8bac9d5](https://github.com/wolfware-labs/moonlit/commit/8bac9d560e2140cf89f5d6971b3f519b48a7390f))

### Bug Fixes

* **sdk:** derive JsonSchema on changelog Category and Entry ([2cae64e](https://github.com/wolfware-labs/moonlit/commit/2cae64e0ff1fdf088d18ca1c20acf0d525a4adb6))

## [0.2.0](https://github.com/wolfware-labs/moonlit/compare/moonlit-v0.1.1...moonlit-v0.2.0) (2026-07-30)

### ⚠ BREAKING CHANGES

* **sdk:** plugin ABI moonlit:plugin bumped 0.1.0 -> 0.2.0; Middleware::Config now requires schemars::JsonSchema.

### Features

* **cli:** scaffold plugins with JsonSchema config for SDK 0.2.0 ([62bb81c](https://github.com/wolfware-labs/moonlit/commit/62bb81c8e4fbda2d9820cab0c0e6bce5e1bdf2c3))
* **cli:** surface icon + middleware config schema in plugin inspect ([962663f](https://github.com/wolfware-labs/moonlit/commit/962663f775dd713e74ae57bf6c1364b1b12c33c2))
* **engine:** carry icon + config schema through host metadata ([bce5ec1](https://github.com/wolfware-labs/moonlit/commit/bce5ec128c21bc22aef3647820e2df241328a401))
* **macro:** embed plugin icon and emit middleware config schemas ([4f4585c](https://github.com/wolfware-labs/moonlit/commit/4f4585c13cfb050583926d467157ecdf4185d11e))
* **sdk:** require JsonSchema on Middleware::Config ([340aee7](https://github.com/wolfware-labs/moonlit/commit/340aee79bee39cfbb48bbc6fbec1585976aac446))
* **sdk:** require plugin ABI 0.2.0 (icon + middleware config schema) ([cd22a17](https://github.com/wolfware-labs/moonlit/commit/cd22a17656da2e0f607fed3b55a0c6d8b6f54dca))
* **wit:** bump plugin ABI to 0.2.0 with icon + middleware config-schema ([b5e8543](https://github.com/wolfware-labs/moonlit/commit/b5e8543df100a54aed5d474d80ae3ec03e3368ae))

## 0.1.1

- `moonlit version` now renders the wolf-and-moon logo with the wordmark,
  slogan, version, credits, and a clickable docs link on a truecolor terminal,
  falling back to a plain banner when color is unavailable.
