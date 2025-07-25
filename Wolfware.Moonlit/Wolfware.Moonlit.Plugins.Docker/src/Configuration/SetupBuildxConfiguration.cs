﻿namespace Wolfware.Moonlit.Plugins.Docker.Configuration;

public sealed class SetupBuildxConfiguration
{
  public string? Name { get; set; }

  public string? Driver { get; set; }

  public string? Endpoint { get; set; }

  public bool Bootstrap { get; set; } = true;

  public bool SetBuilderVariable { get; set; } = true;

  public string[] Platforms { get; set; } = [];
}
