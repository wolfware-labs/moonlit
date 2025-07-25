﻿using System.Text;
using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;
using Spectre.Console.Cli;
using Wolfware.Moonlit.Cli.Commands;
using Wolfware.Moonlit.Cli.Extensions;
using Wolfware.Moonlit.Cli.Injection;
using Wolfware.Moonlit.Core.Extensions;

var configuration = new ConfigurationBuilder()
  .AddEnvironmentVariables()
  .Build();

var services = new ServiceCollection();
services.AddSingleton<IConfiguration>(configuration);
services.AddLogging(builder => builder.AddCliConsole().SetMinimumLevel(LogLevel.Debug));
services.AddMoonlitCore();

Console.OutputEncoding = Encoding.UTF8;

var app = new CommandApp(new TypeRegistrar(services));

app.Configure(config =>
{
  config.SetApplicationName("Moonlit CLI");
  config.UseAssemblyInformationalVersion();

  config.AddCommand<VersionCommand>(VersionCommand.Name)
    .WithDescription(VersionCommand.Description);
  config.AddCommand<ReleaseCommand>(ReleaseCommand.Name)
    .WithDescription(ReleaseCommand.Description);
});

app.SetDefaultCommand<VersionCommand>();

return await app.RunAsync(args);
