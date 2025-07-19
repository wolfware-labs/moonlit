using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;
using Wolfware.Moonlit.Plugins.Docker.Abstractions;
using Wolfware.Moonlit.Plugins.Docker.Middlewares;
using Wolfware.Moonlit.Plugins.Docker.Services;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker;

public sealed class DockerPluginStartup : PluginStartup
{
  protected override void ConfigurePlugin(IServiceCollection services, IConfiguration configuration)
  {
    services.AddSingleton<IDockerClient, DockerClient>();
  }

  protected override void AddMiddlewares(IMiddlewareCollection middlewares)
  {
    middlewares.Add<Login>("login");
    middlewares.Add<SetupBuildx>("setup-buildx");
  }
}
