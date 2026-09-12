# Installing Moonlit

The command you run is always `moonlit`.

## Shell (macOS / Linux)

    curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.sh | sh

## PowerShell (Windows)

    irm https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.ps1 | iex

## Windows Installer (MSI)

Download
[moonlit-x86_64-pc-windows-msvc.msi](https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-x86_64-pc-windows-msvc.msi)
and run it, or install it unattended:

    msiexec /i moonlit-x86_64-pc-windows-msvc.msi /qn

The installer places `moonlit.exe` under `Program Files\moonlit\bin`, adds that
directory to the system `PATH`, and registers the app so it can be removed from
Add/Remove Programs. Installing a newer MSI upgrades in place. It needs
administrator rights and installs per-machine rather than per-user.

The MSI is not code-signed, so SmartScreen shows a publisher warning on the
first run. Verify the download against its published checksum before installing:

    (Get-FileHash moonlit-x86_64-pc-windows-msvc.msi -Algorithm SHA256).Hash

Compare that against
[moonlit-x86_64-pc-windows-msvc.msi.sha256](https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-x86_64-pc-windows-msvc.msi.sha256)
on the release.

## Homebrew

    brew install wolfware-labs/tap/moonlit

## Chocolatey

    choco install moonlit

## npm

    npx @moonlitbuild/cli --help

## Docker

    docker run --rm -v "$PWD:/work" wolfware/moonlit:latest run

The image runs as a non-root user and treats `/work` as the pipeline's working
directory, so mount your repository there. To reuse resolved plugins across
runs, mount the cache as well:

    docker run --rm \
      -v "$PWD:/work" \
      -v moonlit-cache:/home/moonlit/.cache/moonlit \
      wolfware/moonlit:latest run

On Linux the container runs as uid 1000, and a bind mount keeps the host's
ownership, so a pipeline that writes into your repository needs the container to
run as you. If your host uid differs from 1000 (as on most CI runners), pass your own
uid with the root group:

    docker run --rm \
      --user "$(id -u):0" \
      -v "$PWD:/work" \
      wolfware/moonlit:latest run

Tags follow the CLI: `1.2.3`, `1.2`, `1`, and `latest`. Images are published for
`linux/amd64` and `linux/arm64`.

## GitHub Actions

    - uses: wolfware-labs/setup-moonlit@v1
    - run: moonlit run

The action installs the CLI on Linux, macOS and Windows runners, puts it on `PATH`,
and caches the plugin content store between runs. See
[setup-moonlit](https://github.com/wolfware-labs/setup-moonlit) for inputs and outputs.

## From a GitHub Release

Download the archive for your platform from the
[latest release](https://github.com/wolfware-labs/moonlit/releases/latest)
and put `moonlit` on your `PATH`.
