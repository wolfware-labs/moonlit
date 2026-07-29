# Moonlit

Moonlit is a build and release automation engine. You declare a pipeline in
YAML and Moonlit runs it, driving each step through sandboxed WebAssembly
plugin components on a `wasmtime`-based host.

This package installs the `moonlit` command-line tool.

> **Status:** early development.

## Install

**npm**

```sh
npm install -g @moonlitbuild/cli   # puts `moonlit` on your PATH
# or run without installing:
npx @moonlitbuild/cli --help
```

**Shell (macOS / Linux)**

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.sh | sh
```

**PowerShell (Windows)**

```powershell
irm https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.ps1 | iex
```

**Homebrew**

```sh
brew install wolfware-labs/tap/moonlit
```

**Chocolatey**

```sh
choco install moonlit
```

## Quick start

Create a `release.yml` describing your pipeline. Plugins are WebAssembly
components referenced by URL (`oci://…`, `https://…`, or `file://…`); stages
run their middlewares as `<plugin>.<middleware>`:

```yaml
name: my-app
plugins:
  - name: git
    url: oci://<your-registry>/git:latest
    permissions:
      exec: [git]
      filesystem: read-write
stages:
  release:
    - name: context
      run: git.repo-context
    - name: tag
      run: git.latest-tag
      config:
        prefix: v
```

Then run it from the directory containing `release.yml`:

```sh
moonlit run
```

First-party plugins (git, github, gitlab, docker, dotnet, nodejs,
semantic-release, slack, moonlit) live at
[wolfware-labs/moonlit-plugins](https://github.com/wolfware-labs/moonlit-plugins).

## Commands

| Command | Description |
| --- | --- |
| `moonlit run` | Run a release pipeline. |
| `moonlit validate` | Parse, resolve plugins, and verify middleware refs without executing. |
| `moonlit plugin new` | Scaffold a new plugin crate. |
| `moonlit plugin build` | Build the plugin in the current directory to a WASI-P2 component. |
| `moonlit plugin inspect` | Print a component's metadata and middlewares. |
| `moonlit plugin publish` | Publish a built component to an OCI registry. |
| `moonlit login` | Store credentials for an OCI registry. |
| `moonlit cache` | Inspect or clear the plugin content cache. |
| `moonlit version` | Print the version, author, and license. |

Run `moonlit <command> --help` for the full options of any command.

## Links

- Homepage: <https://moonlitbuild.dev/>
- Source & docs: <https://github.com/wolfware-labs/moonlit>

## License

Dual-licensed under either of MIT or Apache-2.0, at your option.
"Moonlit" is a trademark of Wolfware LLC.
