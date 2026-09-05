# Installing Moonlit

The command you run is always `moonlit`.

## Shell (macOS / Linux)

    curl --proto '=https' --tlsv1.2 -LsSf https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.sh | sh

## PowerShell (Windows)

    irm https://github.com/wolfware-labs/moonlit/releases/latest/download/moonlit-installer.ps1 | iex

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
runs instead of fetching them every time, mount the cache as well:

    docker run --rm \
      -v "$PWD:/work" \
      -v moonlit-cache:/home/moonlit/.cache/moonlit \
      wolfware/moonlit:latest run

Tags follow the CLI: `1.2.3`, `1.2`, `1`, and `latest`. Images are published for
`linux/amd64` and `linux/arm64`.

## From a GitHub Release

Download the archive for your platform from the
[latest release](https://github.com/wolfware-labs/moonlit/releases/latest)
and put `moonlit` on your `PATH`.
